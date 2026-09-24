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
    render_selected_document_body_as_text, resolve_document_base_url, rewrite_html_urls,
};
use crate::extract::select_candidates;
use crate::source::LoadedSource;

use super::super::{
    ExecutionLimits, ExtractionRun, SliceCandidate, match_output_payload_bytes,
    output_limit_diagnostic,
};
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
    run_prepared_slice_extraction(
        request,
        &source.text,
        &document,
        source.input_base_url.as_deref(),
        slice,
        patterns,
        None,
    )
}

/// Runs one already-compiled slice strategy against exact source and an already-parsed document.
pub(crate) fn run_prepared_slice_extraction(
    request: &ExtractionRequest,
    source_text: &str,
    document: &scraper::Html,
    input_base_url: Option<&str>,
    slice: &SliceSpec,
    patterns: &CompiledSlicePatterns,
    execution_limits: Option<ExecutionLimits>,
) -> ExtractionRun {
    let document_title = extract_document_title(document);
    let effective_base_url = resolve_document_base_url(document, input_base_url);
    let mut diagnostics = if request.output.rendering.rewrite_urls && effective_base_url.is_none() {
        vec![unresolved_effective_base_diagnostic(
            document_base_href(document).as_deref(),
            true,
        )]
    } else {
        Vec::new()
    };
    let candidates = match extract_compiled_slice_candidates(
        source_text,
        slice,
        patterns,
        request.extraction.selection(),
        execution_limits.map(|limits| limits.max_candidates),
    ) {
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
    if let Some(limits) = execution_limits
        && selected.len() > limits.max_selected_matches
    {
        return ExtractionRun {
            document_title,
            effective_base_url,
            candidate_count,
            diagnostics: vec![error_diagnostic(
                DiagnosticCode::SelectedMatchLimitExceeded,
                "Delimiter-pair execution exceeded its configured selected-match limit.",
                Some(json!({ "maxSelectedMatches": limits.max_selected_matches })),
            )],
            matches: Vec::new(),
        };
    }
    diagnostics.extend(selection_diagnostics);
    diagnostics.extend(slice_markup_diagnostics(source_text, &selected));
    let mut matches = Vec::new();
    let mut total_output_bytes = 0usize;
    let match_count = selected.len();

    for (position, selected_candidate) in selected.iter().enumerate() {
        if let Some(limits) = execution_limits {
            let fragment_bytes = selected_candidate
                .candidate
                .selected_html(source_text)
                .len()
                .max(selected_candidate.candidate.inner_html(source_text).len())
                .max(selected_candidate.candidate.outer_html(source_text).len());
            if fragment_bytes > limits.max_match_output_bytes {
                return ExtractionRun {
                    document_title,
                    effective_base_url,
                    candidate_count,
                    diagnostics: vec![output_limit_diagnostic(
                        "maxMatchOutputBytes",
                        limits.max_match_output_bytes,
                        fragment_bytes,
                        position + 1,
                    )],
                    matches: Vec::new(),
                };
            }
        }
        match build_slice_match(SliceMatchInput {
            request,
            source_text,
            effective_base_url: effective_base_url.as_deref(),
            candidate: &selected_candidate.candidate,
            match_index: position + 1,
            match_count,
            candidate_index: selected_candidate.candidate_index,
            candidate_count,
        }) {
            Ok(extraction_match) => {
                if let Some(limits) = execution_limits {
                    let match_output_bytes = match_output_payload_bytes(&extraction_match);
                    if match_output_bytes > limits.max_match_output_bytes {
                        return ExtractionRun {
                            document_title,
                            effective_base_url,
                            candidate_count,
                            diagnostics: vec![output_limit_diagnostic(
                                "maxMatchOutputBytes",
                                limits.max_match_output_bytes,
                                match_output_bytes,
                                position + 1,
                            )],
                            matches: Vec::new(),
                        };
                    }
                    total_output_bytes = total_output_bytes.saturating_add(match_output_bytes);
                    if total_output_bytes > limits.max_total_output_bytes {
                        return ExtractionRun {
                            document_title,
                            effective_base_url,
                            candidate_count,
                            diagnostics: vec![output_limit_diagnostic(
                                "maxTotalOutputBytes",
                                limits.max_total_output_bytes,
                                total_output_bytes,
                                position + 1,
                            )],
                            matches: Vec::new(),
                        };
                    }
                }
                matches.push(extraction_match)
            }
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

/// Complete context required to project one already-selected delimiter candidate.
pub(crate) struct SliceMatchInput<'a> {
    pub(crate) request: &'a ExtractionRequest,
    pub(crate) source_text: &'a str,
    pub(crate) effective_base_url: Option<&'a str>,
    pub(crate) candidate: &'a SliceCandidate,
    pub(crate) match_index: usize,
    pub(crate) match_count: usize,
    pub(crate) candidate_index: usize,
    pub(crate) candidate_count: usize,
}

pub(crate) fn build_slice_match(input: SliceMatchInput<'_>) -> Result<ExtractionMatch, Diagnostic> {
    let SliceMatchInput {
        request,
        source_text,
        effective_base_url,
        candidate,
        match_index,
        match_count,
        candidate_index,
        candidate_count,
    } = input;
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
                normalized_fragment_html(
                    candidate.selected_html(source_text),
                    effective_base_url,
                    rewrite_urls,
                )
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
                normalized_fragment_html(
                    candidate.outer_html(source_text),
                    effective_base_url,
                    rewrite_urls,
                )
            })
            .clone()
    };
    let inner_html = OnceCell::new();
    let inner_html_value = || {
        inner_html
            .get_or_init(|| {
                normalized_fragment_html(
                    candidate.inner_html(source_text),
                    effective_base_url,
                    rewrite_urls,
                )
            })
            .clone()
    };
    let text_html = OnceCell::new();
    let text_html_value = || {
        text_html
            .get_or_init(|| {
                normalized_fragment_html(
                    candidate.selected_html(source_text),
                    effective_base_url,
                    true,
                )
            })
            .clone()
    };
    let text_document = OnceCell::new();
    let text_document_value =
        || text_document.get_or_init(|| parse_wrapped_fragment(&text_html_value()));
    let text = OnceCell::new();
    let text_value = || {
        text.get_or_init(|| {
            render_selected_document_body_as_text(text_document_value(), whitespace)
        })
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
            "matchedStart": candidate.matched_start(source_text),
            "matchedEnd": candidate.matched_end(source_text),
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
            matched_start: candidate.matched_start(source_text).to_owned(),
            matched_end: candidate.matched_end(source_text).to_owned(),
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
    use crate::contracts::{ExtractionSpec, SelectionSpec, SliceBoundary, SourceRequest};
    use crate::extract::match_output_payload_bytes;

    #[test]
    fn prepared_delimiter_execution_allows_exact_selected_fragment_and_output_limits() {
        let source = "STARTOneENDSTARTTwoEND";
        let slice = SliceSpec::new(
            SliceBoundary::new("START").expect("start boundary"),
            SliceBoundary::new("END").expect("end boundary"),
        );
        let request = ExtractionRequest::new(
            SourceRequest::memory("delimiter-boundaries", source),
            ExtractionSpec::slice(slice.clone()).with_selection(SelectionSpec::All),
        );
        let patterns = CompiledSlicePatterns::compile(&slice).expect("compiled patterns");
        let candidates =
            extract_compiled_slice_candidates(source, &slice, &patterns, &SelectionSpec::All, None)
                .expect("candidates");
        let document = parse_document_node(source);
        let mut fragment_request = request.clone();
        fragment_request.output.include_html = false;
        fragment_request.output.include_text = false;
        let fragment_unrestricted = run_prepared_slice_extraction(
            &fragment_request,
            source,
            &document,
            None,
            &slice,
            &patterns,
            None,
        );
        let max_fragment_bytes = candidates
            .iter()
            .map(|candidate| {
                candidate
                    .selected_html(source)
                    .len()
                    .max(candidate.inner_html(source).len())
                    .max(candidate.outer_html(source).len())
            })
            .max()
            .expect("two candidates");
        let fragment_payload_bytes = fragment_unrestricted
            .matches
            .iter()
            .map(match_output_payload_bytes)
            .max()
            .expect("two match payloads");
        assert!(fragment_payload_bytes < max_fragment_bytes);
        let fragment_bounded = run_prepared_slice_extraction(
            &fragment_request,
            source,
            &document,
            None,
            &slice,
            &patterns,
            Some(ExecutionLimits {
                max_selector_work_units: 1_000,
                max_candidates: candidates.len(),
                max_selected_matches: candidates.len(),
                max_match_output_bytes: max_fragment_bytes,
                max_total_output_bytes: usize::MAX,
            }),
        );
        assert!(fragment_bounded.diagnostics.is_empty());
        assert_eq!(fragment_bounded.matches.len(), candidates.len());

        let unrestricted = run_prepared_slice_extraction(
            &request, source, &document, None, &slice, &patterns, None,
        );
        assert!(unrestricted.diagnostics.is_empty());
        assert_eq!(unrestricted.matches.len(), 2);

        let match_payload_bytes = unrestricted
            .matches
            .iter()
            .map(match_output_payload_bytes)
            .collect::<Vec<_>>();
        let max_match_output_bytes = max_fragment_bytes.max(
            *match_payload_bytes
                .iter()
                .max()
                .expect("two match payloads"),
        );
        let max_total_output_bytes = match_payload_bytes.iter().sum();
        let bounded = run_prepared_slice_extraction(
            &request,
            source,
            &document,
            None,
            &slice,
            &patterns,
            Some(ExecutionLimits {
                max_selector_work_units: 1_000,
                max_candidates: candidates.len(),
                max_selected_matches: candidates.len(),
                max_match_output_bytes,
                max_total_output_bytes,
            }),
        );

        assert!(bounded.diagnostics.is_empty());
        assert_eq!(bounded.candidate_count, candidates.len());
        assert_eq!(bounded.matches.len(), candidates.len());
    }

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
