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
    position_inside_markup_with_step(source_text, position, |cursor, width| {
        cursor.checked_add(width)
    })
}

fn position_inside_markup_with_step(
    source_text: &str,
    position: usize,
    advance: impl Fn(usize, usize) -> Option<usize>,
) -> bool {
    if !markup_position_is_in_bounds(position, source_text.len()) {
        return false;
    }

    let bytes = source_text.as_bytes();
    let mut cursor = 0usize;
    let mut state = MarkupState::Text;

    for _ in 0..bytes.len() {
        if cursor >= position {
            break;
        }
        let (next_state, width) = match state {
            MarkupState::Text => {
                if starts_markup(bytes, cursor) {
                    if bytes[cursor..].starts_with(b"<!--") {
                        (MarkupState::Comment, 4)
                    } else {
                        (MarkupState::Tag { quote: None }, 1)
                    }
                } else {
                    (MarkupState::Text, 1)
                }
            }
            MarkupState::Tag { quote: Some(quote) } => {
                if bytes[cursor] == quote {
                    (MarkupState::Tag { quote: None }, 1)
                } else {
                    (MarkupState::Tag { quote: Some(quote) }, 1)
                }
            }
            MarkupState::Tag { quote: None } => match bytes[cursor] {
                b'\'' | b'"' => {
                    let quote = bytes[cursor];
                    (MarkupState::Tag { quote: Some(quote) }, 1)
                }
                b'>' => (MarkupState::Text, 1),
                _ => (MarkupState::Tag { quote: None }, 1),
            },
            MarkupState::Comment => {
                if bytes[cursor..].starts_with(b"-->") {
                    (MarkupState::Text, 3)
                } else {
                    (MarkupState::Comment, 1)
                }
            }
        };
        // Cursor movement is an internal safety invariant: a malformed scanner step must fail
        // closed instead of turning a diagnostic probe into an unbounded loop.
        let Some(next_cursor) = advance(cursor, width) else {
            return false;
        };
        if !markup_cursor_step_is_valid(cursor, next_cursor, bytes.len()) {
            return false;
        }
        cursor = next_cursor;
        state = next_state;
    }

    !matches!(state, MarkupState::Text)
}

fn markup_position_is_in_bounds(position: usize, length: usize) -> bool {
    position > 0 && position <= length
}

fn markup_cursor_step_is_valid(cursor: usize, next_cursor: usize, length: usize) -> bool {
    next_cursor > cursor && next_cursor <= length
}

#[cfg(test)]
pub(crate) fn position_inside_markup_for_tests(source_text: &str, position: usize) -> bool {
    position_inside_markup(source_text, position)
}

#[cfg(test)]
pub(crate) fn markup_position_is_in_bounds_for_tests(position: usize, length: usize) -> bool {
    markup_position_is_in_bounds(position, length)
}

#[cfg(test)]
pub(crate) fn markup_cursor_step_is_valid_for_tests(
    cursor: usize,
    next_cursor: usize,
    length: usize,
) -> bool {
    markup_cursor_step_is_valid(cursor, next_cursor, length)
}

#[cfg(test)]
pub(crate) fn position_inside_markup_rejects_invalid_progress_for_tests(
    source_text: &str,
    position: usize,
    overflow: bool,
) -> bool {
    position_inside_markup_with_step(source_text, position, |cursor, _| {
        (!overflow).then_some(cursor)
    })
}

#[cfg(test)]
pub(crate) fn position_inside_markup_rejects_out_of_bounds_progress_for_tests(
    source_text: &str,
    position: usize,
) -> bool {
    position_inside_markup_with_step(source_text, position, |_, _| Some(source_text.len() + 1))
}

#[cfg(test)]
pub(crate) fn position_inside_markup_stalled_step_count_for_tests(
    source_text: &str,
    position: usize,
) -> (bool, usize) {
    let steps = std::cell::Cell::new(0usize);
    let result = position_inside_markup_with_step(source_text, position, |cursor, _| {
        steps.set(steps.get() + 1);
        Some(cursor)
    });
    (result, steps.get())
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
