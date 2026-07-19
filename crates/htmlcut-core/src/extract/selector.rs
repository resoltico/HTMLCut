use std::cell::OnceCell;
use std::collections::BTreeSet;

use ego_tree::NodeId;
use scraper::{ElementRef, Html, Node, Selector};
use serde_json::{Value, json};

use crate::contracts::{
    Diagnostic, ExtractionMatch, ExtractionMatchMetadata, ExtractionRequest, SelectorMatchMetadata,
    SelectorQuery, ValueSpec,
};
use crate::diagnostics::{DiagnosticCode, error_diagnostic, unresolved_effective_base_diagnostic};
use crate::document::{
    apply_whitespace_mode, build_node_path, build_preview, document_base_href, element_attributes,
    extract_document_title, extract_element_plain_text, parse_document_node,
    render_element_as_text, resolve_document_base_url, rewrite_urls_in_document,
    serialize_children, serialize_element,
};
use crate::interop::v1::INVALID_SELECTOR_MESSAGE;
use crate::selector_parse::selector_parse_details;
use crate::source::LoadedSource;

use super::{ExtractionRun, select_candidates};

/// Canonicalization policy applied only to a detached selector-match clone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SelectorDomCanonicalization {
    ignored_attributes: BTreeSet<String>,
    strip_whitespace_nodes: bool,
}

impl SelectorDomCanonicalization {
    /// Builds one selector-only detached-clone canonicalization policy.
    pub(crate) fn new(
        ignored_attributes: impl IntoIterator<Item = String>,
        strip_whitespace_nodes: bool,
    ) -> Self {
        Self {
            ignored_attributes: ignored_attributes.into_iter().collect(),
            strip_whitespace_nodes,
        }
    }

    fn ignores_attribute(&self, name: &str) -> bool {
        self.ignored_attributes
            .iter()
            .any(|ignored| ignored.eq_ignore_ascii_case(name))
    }
}

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

    run_validated_selector_extraction(request, source, &parsed_selector, None)
}

pub(crate) fn run_validated_selector_extraction(
    request: &ExtractionRequest,
    source: &LoadedSource,
    parsed_selector: &Selector,
    dom_canonicalization: Option<&SelectorDomCanonicalization>,
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
    // Clone the same DOM that supplies raw text projection before adding detached selected
    // subtrees. Candidate matching remains exclusively anchored to `document` above.
    let mut canonicalization_document = dom_canonicalization.map(|_| {
        text_projection_document
            .as_ref()
            .cloned()
            .unwrap_or_else(|| document.clone())
    });
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
        let (comparison_text_output, comparison_plain_text_output) =
            match (dom_canonicalization, canonicalization_document.as_mut()) {
                (Some(canonicalization), Some(canonicalization_document)) => {
                    let (rendered, plain) = project_canonicalized_selected_clone(
                        canonicalization_document,
                        selected_candidate.candidate.id(),
                        canonicalization,
                        request.output.rendering.whitespace,
                    );
                    (Some(rendered), Some(plain))
                }
                _ => (None, None),
            };
        let built_match = build_selector_match_with_comparison(
            request,
            &rendered_candidate,
            &text_projection_candidate,
            SelectorMatchDetails {
                match_index: position + 1,
                match_count,
                candidate_index: selected_candidate.candidate_index,
                candidate_count,
                comparison_text_output,
                comparison_plain_text_output,
            },
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
    Selector::parse_with_location(selector.as_str()).map_err(|error| {
        error_diagnostic(
            DiagnosticCode::InvalidSelector,
            INVALID_SELECTOR_MESSAGE,
            Some(selector_parse_details(&error)),
        )
    })
}

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

struct SelectorMatchDetails {
    match_index: usize,
    match_count: usize,
    candidate_index: usize,
    candidate_count: usize,
    comparison_text_output: Option<String>,
    comparison_plain_text_output: Option<String>,
}

fn build_selector_match_with_comparison(
    request: &ExtractionRequest,
    node: &ElementRef<'_>,
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
    let plain_text = OnceCell::new();
    let plain_text_value = || {
        plain_text
            .get_or_init(|| {
                extract_element_plain_text(text_node, request.output.rendering.whitespace)
            })
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
        ValueSpec::Structured => {
            let mut structured = json!({
                "matchIndex": match_index,
                "matchCount": match_count,
                "candidateIndex": candidate_index,
                "candidateCount": candidate_count,
                "tagName": tag_name.clone(),
                "path": path.clone(),
                "textOutput": text_value(),
                "plainTextOutput": plain_text_value(),
                "innerHtmlOutput": inner_html_value(),
                "outerHtmlOutput": outer_html_value(),
                "attributes": attributes_value(),
            });
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

fn project_canonicalized_selected_clone(
    document: &mut Html,
    selected_node_id: NodeId,
    canonicalization: &SelectorDomCanonicalization,
    whitespace: crate::WhitespaceMode,
) -> (String, String) {
    let detached_clone_id = document
        .tree
        .get_mut(selected_node_id)
        .expect("selected node IDs must survive an HTML clone")
        .clone_subtree()
        .id();
    canonicalize_detached_subtree(document, detached_clone_id, canonicalization);
    let detached_clone = document
        .tree
        .get(detached_clone_id)
        .and_then(ElementRef::wrap)
        .expect("cloned selected element must remain an element");
    (
        render_element_as_text(&detached_clone, whitespace),
        extract_element_plain_text(&detached_clone, whitespace),
    )
}

fn canonicalize_detached_subtree(
    document: &mut Html,
    detached_root_id: NodeId,
    canonicalization: &SelectorDomCanonicalization,
) {
    let mut node_ids = vec![detached_root_id];
    node_ids.extend(
        document
            .tree
            .get(detached_root_id)
            .expect("detached clone root must survive canonicalization")
            .descendants()
            .map(|node| node.id()),
    );

    for node_id in node_ids {
        let remove_node = document
            .tree
            .get(node_id)
            .is_some_and(|node| match node.value() {
                Node::Text(text) => {
                    canonicalization.strip_whitespace_nodes && text.trim().is_empty()
                }
                _ => false,
            });
        if remove_node {
            document
                .tree
                .get_mut(node_id)
                .expect("cloned node must survive canonicalization")
                .detach();
            continue;
        }

        let mut node = document
            .tree
            .get_mut(node_id)
            .expect("cloned node must survive canonicalization");
        let Node::Element(element) = node.value() else {
            continue;
        };
        if !canonicalization.ignored_attributes.is_empty() {
            element.retain_attributes(|name| !canonicalization.ignores_attribute(name));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::select_first;

    #[test]
    fn detached_clone_canonicalization_never_mutates_the_selected_source_subtree() {
        let mut document = parse_document_node(
            "<article z=\"1\" data-nonce=\"volatile\" a=\"2\"><!-- transient --><span>Guide</span>  \n</article>",
        );
        let selected_node_id = select_first(&document, "article")
            .expect("selected article")
            .id();
        let detached_clone_id = document
            .tree
            .get_mut(selected_node_id)
            .expect("selected node")
            .clone_subtree()
            .id();
        let canonicalization = SelectorDomCanonicalization::new(["data-nonce".to_owned()], true);

        canonicalize_detached_subtree(&mut document, detached_clone_id, &canonicalization);

        let original = document
            .tree
            .get(selected_node_id)
            .and_then(ElementRef::wrap)
            .expect("original selected element");
        let canonical = document
            .tree
            .get(detached_clone_id)
            .and_then(ElementRef::wrap)
            .expect("detached canonical clone");
        assert!(serialize_element(&original).contains("data-nonce=\"volatile\""));
        assert!(serialize_element(&original).contains("<!-- transient -->"));
        assert_eq!(canonical.value().attr("data-nonce"), None);
        assert_eq!(canonical.value().attr("a"), Some("2"));
        assert_eq!(canonical.value().attr("z"), Some("1"));
        let canonical_html = serialize_element(&canonical);
        assert!(!canonical_html.contains("  \n"));
    }
}
