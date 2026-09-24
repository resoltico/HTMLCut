//! Materialization of one selector match and its detached projection.

use ego_tree::NodeId;
use scraper::{ElementRef, Html};
use serde_json::{Value, json};
use std::cell::OnceCell;

use crate::contracts::{
    Diagnostic, ExtractionMatch, ExtractionMatchMetadata, ExtractionRequest, SelectorMatchMetadata,
    ValueSpec,
};
use crate::diagnostics::{DiagnosticCode, error_diagnostic};
use crate::document::{
    apply_whitespace_mode, build_node_path, build_preview, element_attributes,
    extract_element_plain_text, render_element_as_text, rewrite_urls_in_document,
    serialize_children, serialize_element,
};

#[cfg(test)]
pub(crate) fn build_selector_match(
    request: &ExtractionRequest,
    node: &ElementRef<'_>,
    text_node: &ElementRef<'_>,
    match_index: usize,
    match_count: usize,
    candidate_index: usize,
    candidate_count: usize,
) -> Result<ExtractionMatch, Diagnostic> {
    build_selector_match_with_comparison(
        request,
        node,
        node,
        text_node,
        SelectorMatchDetails {
            match_index,
            match_count,
            candidate_index,
            candidate_count,
            comparison_text_output: None,
            comparison_plain_text_output: None,
        },
    )
}

pub(super) struct SelectorMatchDetails {
    pub(super) match_index: usize,
    pub(super) match_count: usize,
    pub(super) candidate_index: usize,
    pub(super) candidate_count: usize,
    pub(super) comparison_text_output: Option<String>,
    pub(super) comparison_plain_text_output: Option<String>,
}

pub(super) fn build_selector_match_with_comparison(
    request: &ExtractionRequest,
    metadata_node: &ElementRef<'_>,
    projection_node: &ElementRef<'_>,
    text_node: &ElementRef<'_>,
    details: SelectorMatchDetails,
) -> Result<ExtractionMatch, Diagnostic> {
    let SelectorMatchDetails {
        match_index,
        match_count,
        candidate_index,
        candidate_count,
        comparison_text_output,
        comparison_plain_text_output,
    } = details;
    let value_spec = request.extraction.value();
    let path = build_node_path(metadata_node);
    let tag_name = metadata_node.value().name().to_owned();
    let attributes = OnceCell::new();
    let attributes_value = || {
        attributes
            .get_or_init(|| element_attributes(projection_node, None, false))
            .clone()
    };
    let text = OnceCell::new();
    let text_value = || {
        text.get_or_init(|| render_element_as_text(text_node, request.output.rendering.whitespace))
            .clone()
    };
    let plain_text = OnceCell::new();
    let plain_text_value = || {
        plain_text
            .get_or_init(|| {
                extract_element_plain_text(text_node, request.output.rendering.whitespace)
            })
            .clone()
    };
    let inner_html = OnceCell::new();
    let inner_html_value = || {
        inner_html
            .get_or_init(|| serialize_children(projection_node))
            .clone()
    };
    let outer_html = OnceCell::new();
    let outer_html_value = || {
        outer_html
            .get_or_init(|| serialize_element(projection_node))
            .clone()
    };
    let value = match value_spec {
        ValueSpec::Text => Value::String(text_value()),
        ValueSpec::SelectedHtml => {
            return Err(error_diagnostic(
                DiagnosticCode::UnsupportedValueType,
                "selected-html is only valid for slice extraction.",
                Some(json!({"strategy":"selector","value":"selected-html","path":path})),
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
                    Some(json!({"attribute":name.as_str(),"path":path})),
                ));
            };
            Value::String(apply_whitespace_mode(
                value,
                request.output.rendering.whitespace,
            ))
        }
        ValueSpec::Structured => {
            let mut structured = json!({"matchIndex":match_index,"matchCount":match_count,"candidateIndex":candidate_index,"candidateCount":candidate_count,"tagName":tag_name.clone(),"path":path.clone(),"textOutput":text_value(),"plainTextOutput":plain_text_value(),"innerHtmlOutput":inner_html_value(),"outerHtmlOutput":outer_html_value(),"attributes":attributes_value()});
            if let Some(comparison_text_output) = &comparison_text_output {
                structured["comparisonTextOutput"] = Value::String(comparison_text_output.clone());
            }
            if let Some(comparison_plain_text_output) = &comparison_plain_text_output {
                structured["comparisonPlainTextOutput"] =
                    Value::String(comparison_plain_text_output.clone());
            }
            structured
        }
    };
    let preview = build_preview(&value, request.output.preview_chars.get());
    let html = request.output.include_html.then(outer_html_value);
    let text_value = request.output.include_text.then(text_value);
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

pub(super) fn cloned_rewritten_selected_fragment(
    document: &Html,
    selected_node_id: NodeId,
    base_url: &str,
) -> Option<Html> {
    let mut fragment = document.clone_subtree_as_fragment(selected_node_id)?;
    rewrite_urls_in_document(&mut fragment, base_url);
    Some(fragment)
}

pub(super) fn fragment_root_element(fragment: &Html) -> Option<ElementRef<'_>> {
    fragment.tree.root().children().find_map(ElementRef::wrap)
}
