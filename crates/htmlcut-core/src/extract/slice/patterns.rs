use regex::RegexBuilder;
use serde_json::json;

use crate::contracts::{Diagnostic, PatternMode, SliceSpec};
use crate::diagnostics::{DiagnosticCode, error_diagnostic};
use crate::result::Range;

use super::super::{FoundRange, SliceCandidate};

#[derive(Clone, Debug)]
pub(crate) enum CompiledPatternMatcher {
    Literal(String),
    Regex(regex::Regex),
}

impl CompiledPatternMatcher {
    pub(crate) fn find(&self, source: &str, offset: usize) -> Option<FoundRange> {
        match self {
            Self::Literal(pattern) => source[offset..].find(pattern).map(|relative| FoundRange {
                start: offset + relative,
                end: offset + relative + pattern.len(),
            }),
            Self::Regex(regex) => regex.find_at(source, offset).map(|matched| FoundRange {
                start: matched.start(),
                end: matched.end(),
            }),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct CompiledSlicePatterns {
    start: CompiledPatternMatcher,
    end: CompiledPatternMatcher,
}

impl CompiledSlicePatterns {
    pub(crate) fn compile(slice: &SliceSpec) -> Result<Self, Diagnostic> {
        Ok(Self {
            start: build_finder(slice.from().as_str(), slice.mode(), slice.flags())?,
            end: build_finder(slice.to().as_str(), slice.mode(), slice.flags())?,
        })
    }
}

#[cfg(test)]
pub(crate) fn extract_slice_candidates(
    source_text: &str,
    slice: &SliceSpec,
) -> Result<Vec<SliceCandidate>, Diagnostic> {
    let patterns = CompiledSlicePatterns::compile(slice)?;
    extract_compiled_slice_candidates(source_text, slice, &patterns)
}

pub(crate) fn extract_compiled_slice_candidates(
    source_text: &str,
    slice: &SliceSpec,
    patterns: &CompiledSlicePatterns,
) -> Result<Vec<SliceCandidate>, Diagnostic> {
    let mut candidates = Vec::new();
    let mut cursor = 0usize;

    while cursor <= source_text.len() {
        let Some(start) = patterns.start.find(source_text, cursor) else {
            break;
        };

        let Some(end) = patterns.end.find(source_text, start.end) else {
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
) -> Result<CompiledPatternMatcher, Diagnostic> {
    if pattern.is_empty() {
        return Err(error_diagnostic(
            DiagnosticCode::InvalidSlicePattern,
            "Patterns must not be empty.",
            None,
        ));
    }

    match mode {
        PatternMode::Literal => Ok(CompiledPatternMatcher::Literal(pattern.to_owned())),
        PatternMode::Regex => Ok(CompiledPatternMatcher::Regex(build_regex(
            pattern,
            flags.unwrap_or_default(),
        )?)),
    }
}

pub(crate) fn build_regex(pattern: &str, flags: &str) -> Result<regex::Regex, Diagnostic> {
    let mut builder = RegexBuilder::new(pattern);

    for flag in flags.chars() {
        match flag {
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
