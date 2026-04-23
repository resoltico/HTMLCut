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
