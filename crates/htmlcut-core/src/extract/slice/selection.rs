use serde_json::json;

use crate::contracts::{Diagnostic, SelectionSpec};
use crate::diagnostics::{DiagnosticCode, error_diagnostic, warning_diagnostic};

use super::super::SelectedCandidate;

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
