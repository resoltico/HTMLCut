use regex::RegexBuilder;
use serde_json::json;

use crate::contracts::{DEFAULT_REGEX_FLAGS, Diagnostic, PatternMode, SliceSpec};
use crate::diagnostics::{DiagnosticCode, error_diagnostic};
use crate::result::Range;

use super::super::{Finder, FoundRange, SliceCandidate};

pub(crate) fn extract_slice_candidates(
    source_text: &str,
    slice: &SliceSpec,
) -> Result<Vec<SliceCandidate>, Diagnostic> {
    let start_finder = build_finder(slice.from().as_str(), slice.mode(), slice.flags())?;
    let end_finder = build_finder(slice.to().as_str(), slice.mode(), slice.flags())?;
    let mut candidates = Vec::new();
    let mut cursor = 0usize;

    while cursor <= source_text.len() {
        let Some(start) = start_finder(source_text, cursor) else {
            break;
        };

        let Some(end) = end_finder(source_text, start.end) else {
            return Err(error_diagnostic(
                DiagnosticCode::NoMatch,
                format!(
                    "End pattern was not found after offset {}: {}",
                    start.start,
                    slice.to()
                ),
                Some(json!({
                    "from": slice.from().as_str(),
                    "to": slice.to().as_str(),
                    "offset": start.start,
                })),
            ));
        };

        let inner_range = Range {
            start: start.end,
            end: end.start,
        };
        let outer_range = Range {
            start: start.start,
            end: end.end,
        };
        let selected_range = Range {
            start: if slice.include_start {
                start.start
            } else {
                start.end
            },
            end: if slice.include_end {
                end.end
            } else {
                end.start
            },
        };
        let candidate = SliceCandidate {
            inner_html: source_text[inner_range.start..inner_range.end].to_owned(),
            outer_html: source_text[outer_range.start..outer_range.end].to_owned(),
            selected_html: source_text[selected_range.start..selected_range.end].to_owned(),
            selected_range,
            inner_range,
            outer_range,
            matched_start: source_text[start.start..start.end].to_owned(),
            matched_end: source_text[end.start..end.end].to_owned(),
        };

        let next_cursor = if candidate.outer_range.end > candidate.outer_range.start {
            candidate.outer_range.end
        } else {
            candidate.outer_range.start + 1
        };
        candidates.push(candidate);
        cursor = next_cursor;
    }

    if candidates.is_empty() {
        return Err(error_diagnostic(
            DiagnosticCode::NoMatch,
            format!("Start pattern was not found: {}", slice.from()),
            Some(json!({
                "from": slice.from().as_str(),
                "to": slice.to().as_str(),
            })),
        ));
    }

    Ok(candidates)
}

pub(crate) fn build_finder(
    pattern: &str,
    mode: PatternMode,
    flags: Option<&str>,
) -> Result<Finder, Diagnostic> {
    if pattern.is_empty() {
        return Err(error_diagnostic(
            DiagnosticCode::InvalidSlicePattern,
            "Patterns must not be empty.",
            None,
        ));
    }

    match mode {
        PatternMode::Literal => {
            let pattern = pattern.to_owned();
            Ok(Box::new(move |source, offset| {
                source[offset..].find(&pattern).map(|relative| FoundRange {
                    start: offset + relative,
                    end: offset + relative + pattern.len(),
                })
            }))
        }
        PatternMode::Regex => {
            let regex = build_regex(pattern, flags.unwrap_or(DEFAULT_REGEX_FLAGS))?;
            Ok(Box::new(move |source, offset| {
                regex.find_at(source, offset).map(|matched| FoundRange {
                    start: matched.start(),
                    end: matched.end(),
                })
            }))
        }
    }
}

pub(crate) fn build_regex(pattern: &str, flags: &str) -> Result<regex::Regex, Diagnostic> {
    let mut builder = RegexBuilder::new(pattern);

    for flag in flags.chars() {
        match flag {
            'g' | 'u' => {}
            'i' => {
                builder.case_insensitive(true);
            }
            'm' => {
                builder.multi_line(true);
            }
            's' => {
                builder.dot_matches_new_line(true);
            }
            'U' => {
                builder.swap_greed(true);
            }
            'x' => {
                builder.ignore_whitespace(true);
            }
            unsupported => {
                return Err(error_diagnostic(
                    DiagnosticCode::InvalidSlicePattern,
                    format!("Unsupported regex flag: {unsupported}"),
                    Some(json!({ "flags": flags })),
                ));
            }
        }
    }

    builder.build().map_err(|error| {
        error_diagnostic(
            DiagnosticCode::InvalidSlicePattern,
            format!("Invalid regular expression: {error}"),
            Some(json!({ "pattern": pattern, "flags": flags })),
        )
    })
}
