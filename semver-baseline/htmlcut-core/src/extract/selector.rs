use std::cell::OnceCell;

use scraper::{ElementRef, Selector};
use serde_json::{Value, json};

use crate::contracts::{
    Diagnostic, ExtractionMatch, ExtractionMatchMetadata, ExtractionRequest, SelectorMatchMetadata,
    SelectorQuery, ValueSpec,
};
use crate::diagnostics::{DiagnosticCode, error_diagnostic, unresolved_effective_base_diagnostic};
use crate::document::{
    apply_whitespace_mode, build_node_path, build_preview, document_base_href, element_attributes,
    extract_document_title, parse_document_node, render_element_as_text, resolve_document_base_url,
    rewrite_urls_in_document, serialize_children, serialize_element,
};
use crate::source::LoadedSource;

use super::{ExtractionRun, select_candidates};

#[cfg(test)]
pub(crate) fn run_selector_extraction(
    request: &ExtractionRequest,
    source: &LoadedSource,
) -> ExtractionRun {
    let Some(selector) = request.extraction.selector_query() else {
        return ExtractionRun {
            document_title: None,
            effective_base_url: None,
            candidate_count: 0,
            diagnostics: vec![error_diagnostic(
                DiagnosticCode::InvalidSelector,
                "Selector extraction request is missing its selector.",
                None,
            )],
            matches: Vec::new(),
        };
    };
    let parsed_selector = match validate_selector_query(selector) {
        Ok(selector) => selector,
        Err(diagnostic) => {
            return ExtractionRun {
                document_title: None,
                effective_base_url: None,
                candidate_count: 0,
                diagnostics: vec![diagnostic],
                matches: Vec::new(),
            };
        }
    };

    run_validated_selector_extraction(request, source, &parsed_selector)
}

pub(crate) fn run_validated_selector_extraction(
    request: &ExtractionRequest,
    source: &LoadedSource,
    parsed_selector: &Selector,
) -> ExtractionRun {
    let document = parse_document_node(&source.text);
    let effective_base_url = resolve_document_base_url(&document, source.input_base_url.as_deref());
    let mut diagnostics = if request.output.rendering.rewrite_urls && effective_base_url.is_none() {
        vec![unresolved_effective_base_diagnostic(
            document_base_href(&document).as_deref(),
            true,
        )]
    } else {
        Vec::new()
    };
    let document_title = extract_document_title(&document);

    let candidates: Vec<ElementRef<'_>> = document.select(parsed_selector).collect();
    let mut rewritten_document = None;
    if request.output.rendering.rewrite_urls
        && let Some(base_url) = effective_base_url.as_deref()
    {
        let mut rewritten = document.clone();
        rewrite_urls_in_document(&mut rewritten, base_url);
        rewritten_document = Some(rewritten);
    }
    let mut text_projection_document = None;
    if let Some(base_url) = effective_base_url.as_deref() {
        let mut text_projection = document.clone();
        rewrite_urls_in_document(&mut text_projection, base_url);
        text_projection_document = Some(text_projection);
    }
    let candidate_count = candidates.len();
    let (selected, selection_diagnostics) =
        select_candidates(&candidates, request.extraction.selection());
    diagnostics.extend(selection_diagnostics);
    let mut matches = Vec::new();
    let match_count = selected.len();

    for (position, selected_candidate) in selected.iter().enumerate() {
        let rendered_candidate = rewritten_document
            .as_ref()
            .and_then(|rewritten| rewritten.tree.get(selected_candidate.candidate.id()))
            .and_then(ElementRef::wrap)
            .unwrap_or(selected_candidate.candidate);
        let text_projection_candidate = text_projection_document
            .as_ref()
            .and_then(|rewritten| rewritten.tree.get(selected_candidate.candidate.id()))
            .and_then(ElementRef::wrap)
            .unwrap_or(rendered_candidate);
        let built_match = build_selector_match(
            request,
            &rendered_candidate,
            &text_projection_candidate,
            position + 1,
            match_count,
            selected_candidate.candidate_index,
            candidate_count,
        );

        match built_match {
            Ok(extraction_match) => matches.push(extraction_match),
            Err(diagnostic) => diagnostics.push(diagnostic),
        }
    }

    ExtractionRun {
        document_title,
        effective_base_url,
        candidate_count,
        diagnostics,
        matches,
    }
}

pub(crate) fn validate_selector_query(selector: &SelectorQuery) -> Result<Selector, Diagnostic> {
    Selector::parse(selector.as_str()).map_err(|error| {
        error_diagnostic(
            DiagnosticCode::InvalidSelector,
            format!("Invalid selector: {selector}: {error}"),
            Some(json!({
                "selector": selector.as_str(),
                "parseError": error.to_string(),
            })),
        )
    })
}

pub(crate) fn build_selector_match(
    request: &ExtractionRequest,
    node: &ElementRef<'_>,
    text_node: &ElementRef<'_>,
    match_index: usize,
    match_count: usize,
    candidate_index: usize,
    candidate_count: usize,
) -> Result<ExtractionMatch, Diagnostic> {
    let value_spec = request.extraction.value();
    let path = build_node_path(node);
    let tag_name = node.value().name().to_owned();
    let attributes = OnceCell::new();
    let attributes_value = || {
        attributes
            .get_or_init(|| element_attributes(node, None, false))
            .clone()
    };
    let text = OnceCell::new();
    let text_value = || {
        text.get_or_init(|| render_element_as_text(text_node, request.output.rendering.whitespace))
            .clone()
    };
    let inner_html = OnceCell::new();
    let inner_html_value = || inner_html.get_or_init(|| serialize_children(node)).clone();
    let outer_html = OnceCell::new();
    let outer_html_value = || outer_html.get_or_init(|| serialize_element(node)).clone();
    let value = match value_spec {
        ValueSpec::Text => Value::String(text_value()),
        ValueSpec::SelectedHtml => {
            return Err(error_diagnostic(
                DiagnosticCode::UnsupportedValueType,
                "selected-html is only valid for slice extraction.",
                Some(json!({
                    "strategy": "selector",
                    "value": "selected-html",
                    "path": path,
                })),
            ));
        }
        ValueSpec::InnerHtml => Value::String(inner_html_value()),
        ValueSpec::OuterHtml => Value::String(outer_html_value()),
        ValueSpec::Attribute { name } => {
            let attributes = attributes_value();
            let Some(value) = attributes.get(name.as_str()) else {
                return Err(error_diagnostic(
                    DiagnosticCode::MissingAttribute,
                    format!("Matched node is missing attribute \"{name}\"."),
                    Some(json!({
                        "attribute": name.as_str(),
                        "path": path,
                    })),
                ));
            };

            Value::String(apply_whitespace_mode(
                value,
                request.output.rendering.whitespace,
            ))
        }
        ValueSpec::Structured => json!({
            "matchIndex": match_index,
            "matchCount": match_count,
            "candidateIndex": candidate_index,
            "candidateCount": candidate_count,
            "tagName": tag_name.clone(),
            "path": path.clone(),
            "textOutput": text_value(),
            "innerHtmlOutput": inner_html_value(),
            "outerHtmlOutput": outer_html_value(),
            "attributes": attributes_value(),
        }),
    };

    let preview = build_preview(&value, request.output.preview_chars.get());
    let html = request.output.include_html.then(outer_html_value);
    let text_value = if request.output.include_text {
        Some(text_value())
    } else {
        None
    };

    Ok(ExtractionMatch {
        index: match_index,
        path: Some(path.clone()),
        value_type: value_spec.value_type(),
        value,
        html,
        text: text_value,
        preview,
        metadata: ExtractionMatchMetadata::Selector(SelectorMatchMetadata {
            candidate_count,
            candidate_index,
            path,
            tag_name,
            attributes: attributes_value(),
        }),
    })
}
