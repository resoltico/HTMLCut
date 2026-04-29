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

use super::ExtractionRun;
use super::slice::select_candidates;

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
    let mut diagnostics = if request.normalization.rewrite_urls && effective_base_url.is_none() {
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
    let rewritten_candidates = if request.normalization.rewrite_urls {
        effective_base_url.as_deref().map(|base_url| {
            let mut rewritten = document.clone();
            rewrite_urls_in_document(&mut rewritten, base_url);
            rewritten_document = Some(rewritten);
            let rewritten_candidates = rewritten_document
                .as_ref()
                .map(|rewritten| rewritten.select(parsed_selector).collect::<Vec<_>>())
                .unwrap_or_default();
            debug_assert_eq!(rewritten_candidates.len(), candidates.len());
            rewritten_candidates
        })
    } else {
        None
    };
    let candidate_count = candidates.len();
    let (selected, selection_diagnostics) =
        select_candidates(&candidates, request.extraction.selection());
    diagnostics.extend(selection_diagnostics);
    let mut matches = Vec::new();
    let match_count = selected.len();

    for (position, selected_candidate) in selected.iter().enumerate() {
        let rendered_candidate = rewritten_candidates
            .as_ref()
            .and_then(|rewritten| rewritten.get(selected_candidate.candidate_index - 1))
            .unwrap_or(&selected_candidate.candidate);
        let built_match = build_selector_match(
            request,
            rendered_candidate,
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
    Selector::parse(selector.as_str()).map_err(|_| {
        error_diagnostic(
            DiagnosticCode::InvalidSelector,
            format!("Invalid selector: {selector}"),
            Some(json!({ "selector": selector.as_str() })),
        )
    })
}

pub(crate) fn build_selector_match(
    request: &ExtractionRequest,
    node: &ElementRef<'_>,
    match_index: usize,
    match_count: usize,
    candidate_index: usize,
    candidate_count: usize,
) -> Result<ExtractionMatch, Diagnostic> {
    let value_spec = request.extraction.value();
    let path = build_node_path(node);
    let tag_name = node.value().name().to_owned();
    let attributes = element_attributes(node, None, false);
    let needs_text = matches!(value_spec, ValueSpec::Text | ValueSpec::Structured)
        || request.output.include_text;
    let text = needs_text.then(|| render_element_as_text(node, request.normalization.whitespace));
    let needs_inner_html = matches!(value_spec, ValueSpec::InnerHtml | ValueSpec::Structured);
    let rewritten_inner_html = needs_inner_html.then(|| serialize_children(node));
    let needs_outer_html = matches!(value_spec, ValueSpec::OuterHtml | ValueSpec::Structured)
        || request.output.include_html;
    let rewritten_outer_html = needs_outer_html.then(|| serialize_element(node));
    let text_value = || {
        text.clone()
            .unwrap_or_else(|| render_element_as_text(node, request.normalization.whitespace))
    };
    let inner_html_value = || {
        rewritten_inner_html
            .clone()
            .unwrap_or_else(|| serialize_children(node))
    };
    let outer_html_value = || {
        rewritten_outer_html
            .clone()
            .unwrap_or_else(|| serialize_element(node))
    };
    let value = match value_spec {
        ValueSpec::Text => Value::String(text_value()),
        ValueSpec::InnerHtml => Value::String(inner_html_value()),
        ValueSpec::OuterHtml => Value::String(outer_html_value()),
        ValueSpec::Attribute { name } => {
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
                request.normalization.whitespace,
            ))
        }
        ValueSpec::Structured => json!({
            "matchIndex": match_index,
            "matchCount": match_count,
            "candidateIndex": candidate_index,
            "candidateCount": candidate_count,
            "tagName": tag_name.clone(),
            "path": path.clone(),
            "text": text_value(),
            "html": inner_html_value(),
            "outerHtml": outer_html_value(),
            "attributes": attributes.clone(),
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
            attributes,
        }),
    })
}
