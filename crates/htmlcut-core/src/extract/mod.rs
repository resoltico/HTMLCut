mod engine;
mod selection;
mod selector;
mod slice;

use crate::catalog::OperationId;
use crate::contracts::{Diagnostic, ExtractionMatch, SourceMetadata};
use crate::diagnostics::{DiagnosticCode, error_diagnostic};
use crate::result::Range;
use serde_json::Value;

#[cfg(test)]
pub(crate) use engine::validate_request;
pub(crate) use engine::{ExtractionRun, finalize_result};
pub use engine::{extract, inspect_source, preview_extraction};
pub(crate) use selection::{select_candidates, select_candidates_with_count};
#[cfg(test)]
pub(crate) use selector::build_selector_match;
#[cfg(test)]
pub(crate) use selector::run_selector_extraction;
pub(crate) use selector::{
    SelectorDomCanonicalization, run_prepared_selector_extraction,
    run_validated_selector_extraction, validate_selector_query,
};
#[cfg(test)]
pub(crate) use slice::run_slice_extraction;
pub(crate) use slice::{
    CompiledSlicePatterns, run_prepared_slice_extraction, run_validated_slice_extraction,
};
#[cfg(test)]
pub(crate) use slice::{
    SliceMatchInput, build_finder, build_regex, build_slice_match, extract_slice_candidates,
    markup_cursor_step_is_valid_for_tests, markup_position_is_in_bounds_for_tests,
    position_inside_markup_for_tests, position_inside_markup_rejects_invalid_progress_for_tests,
    position_inside_markup_rejects_out_of_bounds_progress_for_tests,
    position_inside_markup_stalled_step_count_for_tests, slice_cursor_progress_for_tests,
};

#[derive(Clone, Debug)]
pub(crate) struct SliceCandidate {
    pub(crate) selected_range: Range,
    pub(crate) inner_range: Range,
    pub(crate) outer_range: Range,
    pub(crate) matched_start_range: Range,
    pub(crate) matched_end_range: Range,
}

impl SliceCandidate {
    pub(crate) fn selected_html<'a>(&self, source: &'a str) -> &'a str {
        &source[self.selected_range.start..self.selected_range.end]
    }

    pub(crate) fn inner_html<'a>(&self, source: &'a str) -> &'a str {
        &source[self.inner_range.start..self.inner_range.end]
    }

    pub(crate) fn outer_html<'a>(&self, source: &'a str) -> &'a str {
        &source[self.outer_range.start..self.outer_range.end]
    }

    pub(crate) fn matched_start<'a>(&self, source: &'a str) -> &'a str {
        &source[self.matched_start_range.start..self.matched_start_range.end]
    }

    pub(crate) fn matched_end<'a>(&self, source: &'a str) -> &'a str {
        &source[self.matched_end_range.start..self.matched_end_range.end]
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SelectedCandidate<T> {
    pub(crate) candidate_index: usize,
    pub(crate) candidate: T,
}

#[derive(Clone, Debug)]
pub(crate) struct FinalizedExtraction {
    pub(crate) operation_id: OperationId,
    pub(crate) source: SourceMetadata,
    pub(crate) document_title: Option<String>,
    pub(crate) diagnostics: Vec<Diagnostic>,
    pub(crate) matches: Vec<ExtractionMatch>,
    pub(crate) candidate_count: usize,
}

#[derive(Clone, Copy)]
pub(crate) struct FoundRange {
    pub(crate) start: usize,
    pub(crate) end: usize,
}

/// Bounded work limits applied only by prepared interop execution.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ExecutionLimits {
    pub(crate) max_selector_work_units: u32,
    pub(crate) max_candidates: usize,
    pub(crate) max_selected_matches: usize,
    pub(crate) max_match_output_bytes: usize,
    pub(crate) max_total_output_bytes: usize,
}

pub(crate) fn match_output_payload_bytes(extraction_match: &ExtractionMatch) -> usize {
    value_payload_bytes(&extraction_match.value)
        .saturating_add(extraction_match.html.as_deref().map_or(0, str::len))
        .saturating_add(extraction_match.text.as_deref().map_or(0, str::len))
        .saturating_add(extraction_match.preview.len())
}

fn value_payload_bytes(value: &Value) -> usize {
    match value {
        Value::Null | Value::Bool(_) | Value::Number(_) => 0,
        Value::String(value) => value.len(),
        Value::Array(values) => values.iter().fold(0usize, |total, value| {
            total.saturating_add(value_payload_bytes(value))
        }),
        Value::Object(values) => values.values().fold(0usize, |total, value| {
            total.saturating_add(value_payload_bytes(value))
        }),
    }
}

pub(crate) fn output_limit_diagnostic(
    limit_name: &'static str,
    limit: usize,
    observed: usize,
    match_index: usize,
) -> Diagnostic {
    let message = match limit_name {
        "maxMatchOutputBytes" => {
            "Extraction execution exceeded its configured per-match output limit."
        }
        "maxTotalOutputBytes" => "Extraction execution exceeded its configured total output limit.",
        _ => "Extraction execution exceeded its configured output limit.",
    };
    error_diagnostic(
        DiagnosticCode::OutputLimitExceeded,
        message,
        Some(serde_json::json!({
            "limitName": limit_name,
            "limit": limit,
            "observed": observed,
            "matchIndex": match_index,
        })),
    )
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::diagnostics::DiagnosticCode;

    use super::{output_limit_diagnostic, value_payload_bytes};

    #[test]
    fn value_payload_bytes_counts_nested_utf8_string_payloads_only() {
        let value = json!({
            "title": "é",
            "nested": ["ab", null, true, 7, { "tail": "xyz" }],
        });

        assert_eq!(
            value_payload_bytes(&value),
            "é".len() + "ab".len() + "xyz".len()
        );
    }

    #[test]
    fn output_limit_diagnostics_preserve_the_specific_limit_explanation_and_evidence() {
        for (limit_name, expected_message) in [
            (
                "maxMatchOutputBytes",
                "Extraction execution exceeded its configured per-match output limit.",
            ),
            (
                "maxTotalOutputBytes",
                "Extraction execution exceeded its configured total output limit.",
            ),
            (
                "otherOutputLimit",
                "Extraction execution exceeded its configured output limit.",
            ),
        ] {
            let diagnostic = output_limit_diagnostic(limit_name, 64, 65, 3);

            assert_eq!(diagnostic.code, DiagnosticCode::OutputLimitExceeded);
            assert_eq!(diagnostic.message, expected_message);
            assert_eq!(
                diagnostic.details,
                Some(json!({
                    "limitName": limit_name,
                    "limit": 64,
                    "observed": 65,
                    "matchIndex": 3,
                }))
            );
        }
    }
}
