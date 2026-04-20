use regex::RegexBuilder;
use serde_json::{Value, json};

use crate::contracts::{
    DEFAULT_REGEX_FLAGS, DelimiterPairMatchMetadata, Diagnostic, ExtractionMatch,
    ExtractionMatchMetadata, ExtractionRequest, PatternMode, SelectionSpec, SliceSpec, ValueSpec,
};
use crate::diagnostics::{
    DiagnosticCode, error_diagnostic, slice_splits_markup_diagnostic,
    unresolved_effective_base_diagnostic, warning_diagnostic,
};
use crate::document::{
    apply_whitespace_mode, build_preview, document_base_href, extract_document_title,
    first_fragment_attributes, parse_document_node, render_html_as_text, resolve_document_base_url,
    rewrite_html_urls,
};
use crate::result::Range;
use crate::source::LoadedSource;

use super::{ExtractionRun, FoundRange, SelectedCandidate, SliceCandidate, SliceStructuredValue};

pub(crate) fn run_slice_extraction(
    request: &ExtractionRequest,
    source: &LoadedSource,
) -> ExtractionRun {
    let document = parse_document_node(&source.text);
    let document_title = extract_document_title(&document);
    let effective_base_url = resolve_document_base_url(&document, source.input_base_url.as_deref());
    let mut diagnostics = if request.normalization.rewrite_urls && effective_base_url.is_none() {
        vec![unresolved_effective_base_diagnostic(
            document_base_href(&document).as_deref(),
            true,
        )]
    } else {
        Vec::new()
    };
    let slice = request
        .extraction
        .slice_spec()
        .expect("slice extraction should carry slice boundaries");

    let candidates = match extract_slice_candidates(&source.text, slice) {
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
    let slice = request
        .extraction
        .slice_spec()
        .expect("slice extraction should carry slice boundaries");
    let selected_html = rewrite_html_urls(&candidate.selected_html, effective_base_url, false);
    let outer_html = rewrite_html_urls(&candidate.outer_html, effective_base_url, false);
    let inner_html = rewrite_html_urls(&candidate.inner_html, effective_base_url, false);
    let text = render_html_as_text(&selected_html, request.normalization.whitespace);
    let value = match value_spec {
        ValueSpec::Text => Value::String(text.clone()),
        ValueSpec::InnerHtml => Value::String(selected_html.clone()),
        ValueSpec::OuterHtml => Value::String(outer_html.clone()),
        ValueSpec::Attribute { name } => {
            let attributes = first_fragment_attributes(
                &selected_html,
                effective_base_url,
                request.normalization.rewrite_urls,
            );
            let Some(value) = attributes.get(name.as_str()) else {
                let hint_include_start = !slice.include_start
                    && candidate.selected_range.start != candidate.outer_range.start;
                let message = if hint_include_start {
                    format!(
                        "Extracted fragment is missing attribute \"{name}\". If the attribute lives on the opening tag, use --include-start so the fragment keeps that tag."
                    )
                } else {
                    format!("Extracted fragment is missing attribute \"{name}\".")
                };
                return Err(error_diagnostic(
                    DiagnosticCode::MissingAttribute,
                    message,
                    Some(json!({
                        "attribute": name.as_str(),
                        "selectedRange": candidate.selected_range,
                        "hint": hint_include_start.then_some("use --include-start"),
                    })),
                ));
            };

            Value::String(apply_whitespace_mode(
                value,
                request.normalization.whitespace,
            ))
        }
        ValueSpec::Structured => serde_json::to_value(SliceStructuredValue {
            match_index,
            match_count,
            candidate_index,
            candidate_count,
            text: text.clone(),
            html: selected_html.clone(),
            inner_html: inner_html.clone(),
            outer_html: outer_html.clone(),
            selected_range: candidate.selected_range.clone(),
            inner_range: candidate.inner_range.clone(),
            outer_range: candidate.outer_range.clone(),
            include_start: slice.include_start,
            include_end: slice.include_end,
            matched_start: candidate.matched_start.clone(),
            matched_end: candidate.matched_end.clone(),
        })
        .expect("slice structured value should serialize"),
    };

    Ok(ExtractionMatch {
        index: match_index,
        path: None,
        value_type: value_spec.value_type(),
        preview: build_preview(&value, request.output.preview_chars.get()),
        value,
        html: request.output.include_html.then_some(outer_html.clone()),
        text: request.output.include_text.then_some(text),
        metadata: ExtractionMatchMetadata::DelimiterPair(DelimiterPairMatchMetadata {
            candidate_count,
            candidate_index,
            selected_range: candidate.selected_range.clone(),
            inner_range: candidate.inner_range.clone(),
            outer_range: candidate.outer_range.clone(),
            include_start: slice.include_start,
            include_end: slice.include_end,
            matched_start: candidate.matched_start.clone(),
            matched_end: candidate.matched_end.clone(),
        }),
    })
}

fn slice_markup_diagnostics(
    source_text: &str,
    selected: &[SelectedCandidate<SliceCandidate>],
) -> Vec<Diagnostic> {
    let affected_matches = selected
        .iter()
        .enumerate()
        .filter(|(_, selected_candidate)| {
            slice_splits_markup(source_text, &selected_candidate.candidate.selected_range)
        })
        .map(|(index, selected_candidate)| {
            json!({
                "matchIndex": index + 1,
                "candidateIndex": selected_candidate.candidate_index,
                "selectedRange": selected_candidate.candidate.selected_range,
            })
        })
        .collect::<Vec<_>>();

    if affected_matches.is_empty() {
        return Vec::new();
    }

    let first_range_summary = affected_matches
        .first()
        .and_then(|value| value.get("selectedRange"))
        .and_then(|value| {
            Some(format!(
                "{}..{}",
                value.get("start")?.as_u64()?,
                value.get("end")?.as_u64()?
            ))
        })
        .unwrap_or_else(|| "the selected fragment".to_owned());

    vec![slice_splits_markup_diagnostic(
        &affected_matches,
        &first_range_summary,
    )]
}

fn slice_splits_markup(source_text: &str, range: &Range) -> bool {
    position_inside_markup(source_text, range.start)
        || position_inside_markup(source_text, range.end)
}

fn position_inside_markup(source_text: &str, position: usize) -> bool {
    if position == 0 || position > source_text.len() {
        return false;
    }

    let prefix = &source_text[..position];
    match (prefix.rfind('<'), prefix.rfind('>')) {
        (Some(open), Some(close)) => open > close,
        (Some(_), None) => true,
        _ => false,
    }
}

#[cfg(test)]
pub(crate) fn position_inside_markup_for_tests(source_text: &str, position: usize) -> bool {
    position_inside_markup(source_text, position)
}

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
) -> Result<super::Finder, Diagnostic> {
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

pub(crate) fn select_candidates<T: Clone>(
    candidates: &[T],
    selection: &SelectionSpec,
) -> (Vec<SelectedCandidate<T>>, Vec<Diagnostic>) {
    if candidates.is_empty() {
        return (
            Vec::new(),
            vec![error_diagnostic(
                DiagnosticCode::NoMatch,
                "No matches were found for the extraction request.",
                None,
            )],
        );
    }

    match selection {
        SelectionSpec::All => (
            candidates
                .iter()
                .enumerate()
                .map(|(index, candidate)| SelectedCandidate {
                    candidate_index: index + 1,
                    candidate: candidate.clone(),
                })
                .collect(),
            Vec::new(),
        ),
        SelectionSpec::Single => {
            if candidates.len() > 1 {
                return (
                    Vec::new(),
                    vec![error_diagnostic(
                        DiagnosticCode::AmbiguousMatch,
                        format!(
                            "Exact-one selection requires exactly one candidate, but {} were found.",
                            candidates.len()
                        ),
                        Some(json!({
                            "candidateCount": candidates.len(),
                        })),
                    )],
                );
            }

            (
                vec![SelectedCandidate {
                    candidate_index: 1,
                    candidate: candidates[0].clone(),
                }],
                Vec::new(),
            )
        }
        SelectionSpec::First => {
            let diagnostics = if candidates.len() > 1 {
                vec![warning_diagnostic(
                    DiagnosticCode::MultipleMatches,
                    format!(
                        "Matched {} candidates while using match type first.",
                        candidates.len()
                    ),
                    Some(json!({
                        "candidateCount": candidates.len(),
                        "selectedIndex": 1,
                    })),
                )]
            } else {
                Vec::new()
            };

            (
                vec![SelectedCandidate {
                    candidate_index: 1,
                    candidate: candidates[0].clone(),
                }],
                diagnostics,
            )
        }
        SelectionSpec::Nth { index } => {
            let requested_index = index.get();
            if requested_index > candidates.len() {
                return (
                    Vec::new(),
                    vec![error_diagnostic(
                        DiagnosticCode::MatchIndexOutOfRange,
                        format!(
                            "Match index {} is out of range for {} candidates.",
                            requested_index,
                            candidates.len()
                        ),
                        Some(json!({
                            "requestedIndex": requested_index,
                            "candidateCount": candidates.len(),
                        })),
                    )],
                );
            }

            (
                vec![SelectedCandidate {
                    candidate_index: requested_index,
                    candidate: candidates[requested_index - 1].clone(),
                }],
                Vec::new(),
            )
        }
    }
}
