use serde_json::json;

use crate::contracts::Diagnostic;
use crate::diagnostics::slice_splits_markup_diagnostic;
use crate::result::Range;

use super::super::{SelectedCandidate, SliceCandidate};

pub(super) fn slice_markup_diagnostics(
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

    let bytes = source_text.as_bytes();
    let mut cursor = 0usize;
    let mut state = MarkupState::Text;

    while cursor < position {
        state = match state {
            MarkupState::Text => {
                if starts_markup(bytes, cursor) {
                    if bytes[cursor..].starts_with(b"<!--") {
                        cursor += 4;
                        MarkupState::Comment
                    } else {
                        cursor += 1;
                        MarkupState::Tag { quote: None }
                    }
                } else {
                    cursor += 1;
                    MarkupState::Text
                }
            }
            MarkupState::Tag { quote: Some(quote) } => {
                if bytes[cursor] == quote {
                    cursor += 1;
                    MarkupState::Tag { quote: None }
                } else {
                    cursor += 1;
                    MarkupState::Tag { quote: Some(quote) }
                }
            }
            MarkupState::Tag { quote: None } => match bytes[cursor] {
                b'\'' | b'"' => {
                    let quote = bytes[cursor];
                    cursor += 1;
                    MarkupState::Tag { quote: Some(quote) }
                }
                b'>' => {
                    cursor += 1;
                    MarkupState::Text
                }
                _ => {
                    cursor += 1;
                    MarkupState::Tag { quote: None }
                }
            },
            MarkupState::Comment => {
                if bytes[cursor..].starts_with(b"-->") {
                    cursor += 3;
                    MarkupState::Text
                } else {
                    cursor += 1;
                    MarkupState::Comment
                }
            }
        };
    }

    !matches!(state, MarkupState::Text)
}

#[cfg(test)]
pub(crate) fn position_inside_markup_for_tests(source_text: &str, position: usize) -> bool {
    position_inside_markup(source_text, position)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MarkupState {
    Text,
    Tag { quote: Option<u8> },
    Comment,
}

fn starts_markup(bytes: &[u8], cursor: usize) -> bool {
    if bytes.get(cursor) != Some(&b'<') {
        return false;
    }

    matches!(
        bytes.get(cursor + 1),
        Some(next)
            if next.is_ascii_alphabetic()
                || matches!(next, b'/' | b'!' | b'?')
    )
}
