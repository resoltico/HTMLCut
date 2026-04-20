use scraper::{ElementRef, Selector};
use serde_json::{Value, json};

use crate::contracts::{
    Diagnostic, ExtractionMatch, ExtractionMatchMetadata, ExtractionRequest, SelectorMatchMetadata,
    ValueSpec,
};
use crate::diagnostics::{DiagnosticCode, error_diagnostic, unresolved_effective_base_diagnostic};
use crate::document::{
    apply_whitespace_mode, build_node_path, build_preview, document_base_href, element_attributes,
    extract_document_title, parse_document_node, render_html_as_text, resolve_document_base_url,
    rewrite_html_urls, serialize_children, serialize_element,
};
use crate::source::LoadedSource;

use super::slice::select_candidates;
use super::{ExtractionRun, SelectorStructuredValue};

pub(crate) fn run_selector_extraction(
    request: &ExtractionRequest,
    source: &LoadedSource,
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
    let selector = request
        .extraction
        .selector_query()
        .expect("selector extraction should carry a selector");
    let parsed_selector = match Selector::parse(selector.as_str()) {
        Ok(selector) => selector,
        Err(_) => {
            return ExtractionRun {
                document_title: None,
                effective_base_url,
                candidate_count: 0,
                diagnostics: {
                    diagnostics.push(error_diagnostic(
                        DiagnosticCode::InvalidSelector,
                        format!("Invalid selector: {selector}"),
                        Some(json!({ "selector": selector.as_str() })),
                    ));
                    diagnostics
                },
                matches: Vec::new(),
            };
        }
    };
    let document_title = extract_document_title(&document);

    let candidates: Vec<ElementRef<'_>> = document.select(&parsed_selector).collect();
    let candidate_count = candidates.len();
    let (selected, selection_diagnostics) =
        select_candidates(&candidates, request.extraction.selection());
    diagnostics.extend(selection_diagnostics);
    let mut matches = Vec::new();
    let match_count = selected.len();

    for (position, selected_candidate) in selected.iter().enumerate() {
        let built_match = build_selector_match(
            request,
            effective_base_url.as_deref(),
            &selected_candidate.candidate,
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

pub(crate) fn build_selector_match(
    request: &ExtractionRequest,
    effective_base_url: Option<&str>,
    node: &ElementRef<'_>,
    match_index: usize,
    match_count: usize,
    candidate_index: usize,
    candidate_count: usize,
) -> Result<ExtractionMatch, Diagnostic> {
    let value_spec = request.extraction.value();
    let path = build_node_path(node);
    let tag_name = node.value().name().to_owned();
    let attributes =
        element_attributes(node, effective_base_url, request.normalization.rewrite_urls);
    let inner_html = serialize_children(node);
    let outer_html = serialize_element(node);
    let rewritten_inner_html = rewrite_html_urls(&inner_html, effective_base_url, false);
    let rewritten_outer_html = rewrite_html_urls(&outer_html, effective_base_url, false);
    let text = render_html_as_text(&rewritten_inner_html, request.normalization.whitespace);
    let value = match value_spec {
        ValueSpec::Text => Value::String(text.clone()),
        ValueSpec::InnerHtml => Value::String(rewritten_inner_html.clone()),
        ValueSpec::OuterHtml => Value::String(rewritten_outer_html.clone()),
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
        ValueSpec::Structured => serde_json::to_value(SelectorStructuredValue {
            match_index,
            match_count,
            candidate_index,
            candidate_count,
            tag_name: tag_name.clone(),
            path: path.clone(),
            text: text.clone(),
            html: rewritten_inner_html.clone(),
            outer_html: rewritten_outer_html.clone(),
            attributes: attributes.clone(),
        })
        .expect("selector structured value should serialize"),
    };

    let preview = build_preview(&value, request.output.preview_chars.get());
    let html = if request.output.include_html {
        Some(rewritten_outer_html)
    } else {
        None
    };
    let text_value = if request.output.include_text {
        Some(text.clone())
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
