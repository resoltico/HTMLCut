use std::cell::OnceCell;

use serde_json::{Value, json};

use crate::contracts::{
    BoundaryRetention, DelimiterPairMatchMetadata, Diagnostic, ExtractionMatch,
    ExtractionMatchMetadata, ExtractionRequest, SliceSpec, ValueSpec,
};
use crate::diagnostics::{DiagnosticCode, error_diagnostic, unresolved_effective_base_diagnostic};
use crate::document::{
    apply_whitespace_mode, build_preview, document_base_href, element_attributes,
    extract_document_title, first_body_child_element, parse_document_node, parse_wrapped_fragment,
    render_document_body_as_text, resolve_document_base_url, rewrite_html_urls,
};
use crate::extract::select_candidates;
use crate::source::LoadedSource;

use super::super::{ExtractionRun, SliceCandidate};
use super::markup::slice_markup_diagnostics;
use super::patterns::{CompiledSlicePatterns, extract_compiled_slice_candidates};

#[cfg(test)]
pub(crate) fn run_slice_extraction(
    request: &ExtractionRequest,
    source: &LoadedSource,
) -> ExtractionRun {
    let Some(slice) = request.extraction.slice_spec() else {
        return ExtractionRun {
            document_title: None,
            effective_base_url: None,
            candidate_count: 0,
            diagnostics: vec![error_diagnostic(
                DiagnosticCode::InvalidSlicePattern,
                "Slice extraction request is missing its boundaries.",
                None,
            )],
            matches: Vec::new(),
        };
    };
    let patterns = match CompiledSlicePatterns::compile(slice) {
        Ok(patterns) => patterns,
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

    run_validated_slice_extraction(request, source, slice, &patterns)
}

pub(crate) fn run_validated_slice_extraction(
    request: &ExtractionRequest,
    source: &LoadedSource,
    slice: &SliceSpec,
    patterns: &CompiledSlicePatterns,
) -> ExtractionRun {
    let document = parse_document_node(&source.text);
    let document_title = extract_document_title(&document);
    let effective_base_url = resolve_document_base_url(&document, source.input_base_url.as_deref());
    let mut diagnostics = if request.output.rendering.rewrite_urls && effective_base_url.is_none() {
        vec![unresolved_effective_base_diagnostic(
            document_base_href(&document).as_deref(),
            true,
        )]
    } else {
        Vec::new()
    };
    let candidates = match extract_compiled_slice_candidates(&source.text, slice, patterns) {
        Ok(candidates) => candidates,
        Err(diagnostic) => {
            return ExtractionRun {
                document_title,
                effective_base_url,
                candidate_count: 0,
                diagnostics: {
                    diagnostics.push(diagnostic);
                    diagnostics
                },
                matches: Vec::new(),
            };
        }
    };

    let candidate_count = candidates.len();
    let (selected, selection_diagnostics) =
        select_candidates(&candidates, request.extraction.selection());
    diagnostics.extend(selection_diagnostics);
    diagnostics.extend(slice_markup_diagnostics(&source.text, &selected));
    let mut matches = Vec::new();
    let match_count = selected.len();

    for (position, selected_candidate) in selected.iter().enumerate() {
        match build_slice_match(
            request,
            effective_base_url.as_deref(),
            &selected_candidate.candidate,
            position + 1,
            match_count,
            selected_candidate.candidate_index,
            candidate_count,
        ) {
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

pub(crate) fn build_slice_match(
    request: &ExtractionRequest,
    effective_base_url: Option<&str>,
    candidate: &SliceCandidate,
    match_index: usize,
    match_count: usize,
    candidate_index: usize,
    candidate_count: usize,
) -> Result<ExtractionMatch, Diagnostic> {
    let value_spec = request.extraction.value();
    let Some(slice) = request.extraction.slice_spec() else {
        return Err(error_diagnostic(
            DiagnosticCode::InvalidSlicePattern,
            "Slice extraction request is missing its boundaries.",
            None,
        ));
    };
    let rewrite_urls = request.output.rendering.rewrite_urls;
    let whitespace = request.output.rendering.whitespace;
    let selected_html = OnceCell::new();
    let selected_html_value = || {
        selected_html
            .get_or_init(|| {
                normalized_fragment_html(&candidate.selected_html, effective_base_url, rewrite_urls)
            })
            .clone()
    };
    let selected_document = OnceCell::new();
    let selected_document_value =
        || selected_document.get_or_init(|| parse_wrapped_fragment(&selected_html_value()));
    let outer_html = OnceCell::new();
    let outer_html_value = || {
        outer_html
            .get_or_init(|| {
                normalized_fragment_html(&candidate.outer_html, effective_base_url, rewrite_urls)
            })
            .clone()
    };
    let inner_html = OnceCell::new();
    let inner_html_value = || {
        inner_html
            .get_or_init(|| {
                normalized_fragment_html(&candidate.inner_html, effective_base_url, rewrite_urls)
            })
            .clone()
    };
    let text_html = OnceCell::new();
    let text_html_value = || {
        text_html
            .get_or_init(|| {
                normalized_fragment_html(&candidate.selected_html, effective_base_url, true)
            })
            .clone()
    };
    let text_document = OnceCell::new();
    let text_document_value =
        || text_document.get_or_init(|| parse_wrapped_fragment(&text_html_value()));
    let text = OnceCell::new();
    let text_value = || {
        text.get_or_init(|| render_document_body_as_text(text_document_value(), whitespace))
            .clone()
    };
    let attribute_value = |attribute_name: &str| -> Result<Value, Diagnostic> {
        build_attribute_value(
            request,
            slice,
            candidate,
            selected_document_value(),
            attribute_name,
        )
    };
    let attribute_map = OnceCell::new();
    let attribute_map_value = || {
        attribute_map
            .get_or_init(|| {
                first_body_child_element(selected_document_value())
                    .map(|element| element_attributes(&element, None, false))
                    .unwrap_or_default()
            })
            .clone()
    };
    let value = match value_spec {
        ValueSpec::Text => Value::String(text_value()),
        ValueSpec::SelectedHtml => Value::String(selected_html_value()),
        ValueSpec::InnerHtml => Value::String(inner_html_value()),
        ValueSpec::OuterHtml => Value::String(outer_html_value()),
        ValueSpec::Attribute { name } => attribute_value(name.as_str())?,
        ValueSpec::Structured => json!({
            "matchIndex": match_index,
            "matchCount": match_count,
            "candidateIndex": candidate_index,
            "candidateCount": candidate_count,
            "textOutput": text_value(),
            "selectedHtmlOutput": selected_html_value(),
            "innerHtmlOutput": inner_html_value(),
            "outerHtmlOutput": outer_html_value(),
            "attributes": attribute_map_value(),
            "selectedRange": candidate.selected_range.clone(),
            "innerRange": candidate.inner_range.clone(),
            "outerRange": candidate.outer_range.clone(),
            "includeStart": slice.includes_start(),
            "includeEnd": slice.includes_end(),
            "matchedStart": candidate.matched_start.clone(),
            "matchedEnd": candidate.matched_end.clone(),
        }),
    };

    Ok(ExtractionMatch {
        index: match_index,
        path: None,
        value_type: value_spec.value_type(),
        preview: build_preview(&value, request.output.preview_chars.get()),
        value,
        html: if request.output.include_html {
            Some(outer_html_value())
        } else {
            None
        },
        text: if request.output.include_text {
            Some(text_value())
        } else {
            None
        },
        metadata: ExtractionMatchMetadata::DelimiterPair(DelimiterPairMatchMetadata {
            candidate_count,
            candidate_index,
            selected_range: candidate.selected_range.clone(),
            inner_range: candidate.inner_range.clone(),
            outer_range: candidate.outer_range.clone(),
            include_start: slice.includes_start(),
            include_end: slice.includes_end(),
            matched_start: candidate.matched_start.clone(),
            matched_end: candidate.matched_end.clone(),
        }),
    })
}

fn build_attribute_value(
    request: &ExtractionRequest,
    slice: &SliceSpec,
    candidate: &SliceCandidate,
    selected_document: &scraper::Html,
    attribute_name: &str,
) -> Result<Value, Diagnostic> {
    let attributes = first_body_child_element(selected_document)
        .map(|element| element_attributes(&element, None, false))
        .unwrap_or_default();
    let Some(value) = attributes.get(attribute_name) else {
        let hint_include_start = !slice.includes_start()
            && candidate.selected_range.start != candidate.outer_range.start;
        let boundary_retention_hint = hint_include_start
            .then(|| suggested_boundary_retention_with_start(slice.boundary_retention));
        let message = if hint_include_start {
            format!(
                "Extracted fragment is missing attribute \"{attribute_name}\". If the attribute lives on the opening tag, use --boundary-retention {} so the fragment keeps that tag.",
                boundary_retention_hint.expect("start-boundary hint should exist"),
            )
        } else {
            format!("Extracted fragment is missing attribute \"{attribute_name}\".")
        };
        return Err(error_diagnostic(
            DiagnosticCode::MissingAttribute,
            message,
            Some(json!({
                "attribute": attribute_name,
                "selectedRange": candidate.selected_range,
                "hint": boundary_retention_hint
                    .map(|mode| format!("use --boundary-retention {mode}")),
            })),
        ));
    };

    Ok(Value::String(apply_whitespace_mode(
        value,
        request.output.rendering.whitespace,
    )))
}

fn normalized_fragment_html(
    fragment: &str,
    effective_base_url: Option<&str>,
    rewrite_urls: bool,
) -> String {
    if rewrite_urls {
        rewrite_html_urls(fragment, effective_base_url, false)
    } else {
        fragment.to_owned()
    }
}

fn suggested_boundary_retention_with_start(retention: BoundaryRetention) -> &'static str {
    match retention {
        BoundaryRetention::ExcludeBoth => "include-start",
        BoundaryRetention::IncludeEnd => "include-both",
        BoundaryRetention::IncludeStart | BoundaryRetention::IncludeBoth => {
            unreachable!("start-boundary hint is only used when the start boundary is excluded")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_boundary_retention_hints_cover_both_excluded_and_end_only_modes() {
        assert_eq!(
            suggested_boundary_retention_with_start(BoundaryRetention::ExcludeBoth),
            "include-start"
        );
        assert_eq!(
            suggested_boundary_retention_with_start(BoundaryRetention::IncludeEnd),
            "include-both"
        );
    }

    #[test]
    #[should_panic(
        expected = "start-boundary hint is only used when the start boundary is excluded"
    )]
    fn start_boundary_retention_hints_reject_included_start_modes() {
        let _ = suggested_boundary_retention_with_start(BoundaryRetention::IncludeStart);
    }
}
