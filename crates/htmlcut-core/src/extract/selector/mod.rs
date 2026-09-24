use scraper::{ElementRef, Html, Selector};
use selectors::work_budget::SelectorWorkBudget;
use serde_json::json;
use std::collections::BTreeSet;

use crate::contracts::{Diagnostic, ExtractionRequest, SelectorQuery};
use crate::diagnostics::{DiagnosticCode, error_diagnostic, unresolved_effective_base_diagnostic};
#[cfg(test)]
use crate::document::serialize_element;
use crate::document::{
    document_base_href, extract_document_title, parse_document_node, resolve_document_base_url,
};
use crate::interop::v2::INVALID_SELECTOR_MESSAGE;
use crate::selector_parse::selector_parse_details;
use crate::source::LoadedSource;

use super::{
    ExecutionLimits, ExtractionRun, match_output_payload_bytes, output_limit_diagnostic,
    select_candidates, select_candidates_with_count,
};

mod canonicalization;
mod match_projection;

const SINGLE_SELECTION_RETAINED_CANDIDATE_CAPACITY: usize = 2;

fn should_retain_single_selection_candidate(retained_candidate_count: usize) -> bool {
    retained_candidate_count < SINGLE_SELECTION_RETAINED_CANDIDATE_CAPACITY
}

#[cfg(test)]
use canonicalization::canonicalize_detached_subtree;
use canonicalization::project_canonicalized_selected_clone;
#[cfg(test)]
pub(crate) use match_projection::build_selector_match;
use match_projection::{
    SelectorMatchDetails, build_selector_match_with_comparison, cloned_rewritten_selected_fragment,
    fragment_root_element,
};

/// Canonicalization policy applied only to a detached selector-match clone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SelectorDomCanonicalization {
    ignored_attributes: BTreeSet<String>,
    strip_whitespace_nodes: bool,
}

impl SelectorDomCanonicalization {
    /// Builds one selector-only detached-clone canonicalization policy.
    pub(crate) fn new(
        ignored_attributes: impl IntoIterator<Item = String>,
        strip_whitespace_nodes: bool,
    ) -> Self {
        Self {
            ignored_attributes: ignored_attributes.into_iter().collect(),
            strip_whitespace_nodes,
        }
    }

    fn ignores_attribute(&self, name: &str) -> bool {
        self.ignored_attributes
            .iter()
            .any(|ignored| ignored.eq_ignore_ascii_case(name))
    }
}

#[cfg(test)]
pub(crate) fn run_selector_extraction(
    request: &ExtractionRequest,
    source: &LoadedSource,
) -> ExtractionRun {
    let Some(selector) = request.extraction.selector_query() else {
        return ExtractionRun {
            document_title: None,
            effective_base_url: None,
            candidate_count: 0,
            diagnostics: vec![error_diagnostic(
                DiagnosticCode::InvalidSelector,
                "Selector extraction request is missing its selector.",
                None,
            )],
            matches: Vec::new(),
        };
    };
    let parsed_selector = match validate_selector_query(selector) {
        Ok(selector) => selector,
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

    run_validated_selector_extraction(request, source, &parsed_selector, None)
}

pub(crate) fn run_validated_selector_extraction(
    request: &ExtractionRequest,
    source: &LoadedSource,
    parsed_selector: &Selector,
    dom_canonicalization: Option<&SelectorDomCanonicalization>,
) -> ExtractionRun {
    let document = parse_document_node(&source.text);
    run_prepared_selector_extraction(
        request,
        &document,
        source.input_base_url.as_deref(),
        parsed_selector,
        dom_canonicalization,
        None,
    )
}

/// Runs one already-compiled selector against one already-parsed document.
pub(crate) fn run_prepared_selector_extraction(
    request: &ExtractionRequest,
    document: &Html,
    input_base_url: Option<&str>,
    parsed_selector: &Selector,
    dom_canonicalization: Option<&SelectorDomCanonicalization>,
    execution_limits: Option<ExecutionLimits>,
) -> ExtractionRun {
    let effective_base_url = resolve_document_base_url(document, input_base_url);
    let mut diagnostics = if request.output.rendering.rewrite_urls && effective_base_url.is_none() {
        vec![unresolved_effective_base_diagnostic(
            document_base_href(document).as_deref(),
            true,
        )]
    } else {
        Vec::new()
    };
    let document_title = extract_document_title(document);

    let (candidates, candidate_count) = match execution_limits {
        None => {
            let candidates = document.select(parsed_selector).collect::<Vec<_>>();
            let candidate_count = candidates.len();
            (candidates, candidate_count)
        }
        Some(limits) => {
            let maximum = limits.max_selector_work_units;
            let budget = SelectorWorkBudget::new(maximum);
            let mut candidates = Vec::new();
            let mut candidate_count = 0usize;
            for node in document
                .tree
                .root()
                .descendants()
                .filter_map(ElementRef::wrap)
            {
                match parsed_selector.matches_with_budget(&node, &budget) {
                    Ok(true) => {
                        candidate_count += 1;
                        if candidate_count > limits.max_candidates {
                            return ExtractionRun {
                                document_title,
                                effective_base_url,
                                candidate_count,
                                diagnostics: vec![error_diagnostic(
                                    DiagnosticCode::CandidateLimitExceeded,
                                    "Selector execution exceeded its configured candidate limit.",
                                    Some(json!({
                                        "maxCandidates": limits.max_candidates,
                                    })),
                                )],
                                matches: Vec::new(),
                            };
                        }

                        match request.extraction.selection() {
                            crate::SelectionSpec::All => {
                                if candidates.len() >= limits.max_selected_matches {
                                    return ExtractionRun {
                                        document_title,
                                        effective_base_url,
                                        candidate_count,
                                        diagnostics: vec![error_diagnostic(
                                            DiagnosticCode::SelectedMatchLimitExceeded,
                                            "Selector execution exceeded its configured selected-match limit.",
                                            Some(json!({
                                                "maxSelectedMatches": limits.max_selected_matches,
                                            })),
                                        )],
                                        matches: Vec::new(),
                                    };
                                }
                                candidates.push(node);
                            }
                            crate::SelectionSpec::Single => {
                                if should_retain_single_selection_candidate(candidates.len()) {
                                    candidates.push(node);
                                }
                            }
                            crate::SelectionSpec::First => {
                                if candidates.is_empty() {
                                    candidates.push(node);
                                }
                            }
                            crate::SelectionSpec::Nth { index } => {
                                if candidate_count <= index.get() {
                                    candidates.push(node);
                                }
                            }
                        }
                    }
                    Ok(false) => {}
                    Err(_) => {
                        return ExtractionRun {
                            document_title,
                            effective_base_url,
                            candidate_count,
                            diagnostics: vec![error_diagnostic(
                                DiagnosticCode::SelectorWorkLimitExceeded,
                                "Selector execution exceeded its configured work budget.",
                                Some(json!({ "maxSelectorWorkUnits": maximum })),
                            )],
                            matches: Vec::new(),
                        };
                    }
                }
            }
            (candidates, candidate_count)
        }
    };
    let (selected, selection_diagnostics) = match execution_limits {
        Some(_) => select_candidates_with_count(
            &candidates,
            candidate_count,
            request.extraction.selection(),
        ),
        None => select_candidates(&candidates, request.extraction.selection()),
    };
    diagnostics.extend(selection_diagnostics);
    let mut matches = Vec::new();
    let mut total_output_bytes = 0usize;
    let match_count = selected.len();

    for (position, selected_candidate) in selected.iter().enumerate() {
        let selected_fragment = effective_base_url.as_deref().and_then(|base_url| {
            cloned_rewritten_selected_fragment(
                document,
                selected_candidate.candidate.id(),
                base_url,
            )
        });
        let built_match = if let Some(selected_fragment) = selected_fragment.as_ref() {
            let projection_candidate = fragment_root_element(selected_fragment)
                .expect("selected detached fragment must preserve its root element");
            let mut canonicalization_fragment =
                dom_canonicalization.map(|_| selected_fragment.clone());
            let (comparison_text_output, comparison_plain_text_output) =
                match (dom_canonicalization, canonicalization_fragment.as_mut()) {
                    (Some(canonicalization), Some(canonicalization_fragment)) => {
                        let selected_id = fragment_root_element(canonicalization_fragment)
                            .expect("selected detached fragment must preserve its root element")
                            .id();
                        let (rendered, plain) = project_canonicalized_selected_clone(
                            canonicalization_fragment,
                            selected_id,
                            canonicalization,
                            request.output.rendering.whitespace,
                        );
                        (Some(rendered), Some(plain))
                    }
                    _ => (None, None),
                };
            build_selector_match_with_comparison(
                request,
                &selected_candidate.candidate,
                if request.output.rendering.rewrite_urls {
                    &projection_candidate
                } else {
                    &selected_candidate.candidate
                },
                &projection_candidate,
                SelectorMatchDetails {
                    match_index: position + 1,
                    match_count,
                    candidate_index: selected_candidate.candidate_index,
                    candidate_count,
                    comparison_text_output,
                    comparison_plain_text_output,
                },
            )
        } else {
            let mut canonicalization_fragment = dom_canonicalization.and_then(|_| {
                document.clone_subtree_as_fragment(selected_candidate.candidate.id())
            });
            let (comparison_text_output, comparison_plain_text_output) =
                match (dom_canonicalization, canonicalization_fragment.as_mut()) {
                    (Some(canonicalization), Some(canonicalization_fragment)) => {
                        let selected_id = fragment_root_element(canonicalization_fragment)
                            .expect("selected detached fragment must preserve its root element")
                            .id();
                        let (rendered, plain) = project_canonicalized_selected_clone(
                            canonicalization_fragment,
                            selected_id,
                            canonicalization,
                            request.output.rendering.whitespace,
                        );
                        (Some(rendered), Some(plain))
                    }
                    _ => (None, None),
                };
            build_selector_match_with_comparison(
                request,
                &selected_candidate.candidate,
                &selected_candidate.candidate,
                &selected_candidate.candidate,
                SelectorMatchDetails {
                    match_index: position + 1,
                    match_count,
                    candidate_index: selected_candidate.candidate_index,
                    candidate_count,
                    comparison_text_output,
                    comparison_plain_text_output,
                },
            )
        };

        match built_match {
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

pub(crate) fn validate_selector_query(selector: &SelectorQuery) -> Result<Selector, Diagnostic> {
    Selector::parse_with_location(selector.as_str()).map_err(|error| {
        error_diagnostic(
            DiagnosticCode::InvalidSelector,
            INVALID_SELECTOR_MESSAGE,
            Some(selector_parse_details(&error)),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::{ExtractionSpec, SourceRequest};
    use crate::document::select_first;

    #[test]
    fn single_selection_retains_only_the_two_candidates_needed_to_detect_ambiguity() {
        assert!(should_retain_single_selection_candidate(0));
        assert!(should_retain_single_selection_candidate(1));
        assert!(!should_retain_single_selection_candidate(2));
        assert!(!should_retain_single_selection_candidate(3));
    }

    #[test]
    fn detached_clone_canonicalization_never_mutates_the_selected_source_subtree() {
        let mut document = parse_document_node(
            "<article z=\"1\" data-nonce=\"volatile\" a=\"2\"><!-- transient --><span>Guide</span>  \n</article>",
        );
        let selected_node_id = select_first(&document, "article")
            .expect("selected article")
            .id();
        let detached_clone_id = document
            .tree
            .get_mut(selected_node_id)
            .expect("selected node")
            .clone_subtree()
            .id();
        let canonicalization = SelectorDomCanonicalization::new(["data-nonce".to_owned()], true);

        canonicalize_detached_subtree(&mut document, detached_clone_id, &canonicalization);

        let original = document
            .tree
            .get(selected_node_id)
            .and_then(ElementRef::wrap)
            .expect("original selected element");
        let canonical = document
            .tree
            .get(detached_clone_id)
            .and_then(ElementRef::wrap)
            .expect("detached canonical clone");
        assert!(serialize_element(&original).contains("data-nonce=\"volatile\""));
        assert!(serialize_element(&original).contains("<!-- transient -->"));
        assert_eq!(canonical.value().attr("data-nonce"), None);
        assert_eq!(canonical.value().attr("a"), Some("2"));
        assert_eq!(canonical.value().attr("z"), Some("1"));
        let canonical_html = serialize_element(&canonical);
        assert!(!canonical_html.contains("  \n"));
    }

    #[test]
    fn prepared_selector_candidate_limit_allows_the_boundary_match_then_reports_the_next_one() {
        let request = ExtractionRequest::new(
            SourceRequest::memory(
                "candidate-boundary",
                "<main><article>one</article><article>two</article></main>",
            ),
            ExtractionSpec::selector(SelectorQuery::new("article").expect("selector query")),
        );
        let document =
            parse_document_node("<main><article>one</article><article>two</article></main>");
        let selector = Selector::parse("article").expect("parsed selector");
        let run = run_prepared_selector_extraction(
            &request,
            &document,
            None,
            &selector,
            None,
            Some(ExecutionLimits {
                max_selector_work_units: 1_000,
                max_candidates: 1,
                max_selected_matches: 8,
                max_match_output_bytes: 1_024,
                max_total_output_bytes: 4_096,
            }),
        );

        assert_eq!(run.candidate_count, 2);
        assert!(run.matches.is_empty());
        assert_eq!(
            run.diagnostics.first().map(|diagnostic| diagnostic.code),
            Some(DiagnosticCode::CandidateLimitExceeded)
        );
        assert_eq!(
            run.diagnostics
                .first()
                .and_then(|diagnostic| diagnostic.details.as_ref()),
            Some(&json!({ "maxCandidates": 1 }))
        );
    }

    #[test]
    fn prepared_selector_output_limits_allow_exact_per_match_and_total_payloads() {
        let request = ExtractionRequest::new(
            SourceRequest::memory(
                "output-boundary",
                "<main><article>one</article><article>two</article></main>",
            ),
            ExtractionSpec::selector(SelectorQuery::new("article").expect("selector query"))
                .with_selection(crate::SelectionSpec::All),
        );
        let document =
            parse_document_node("<main><article>one</article><article>two</article></main>");
        let selector = Selector::parse("article").expect("parsed selector");
        let unrestricted =
            run_prepared_selector_extraction(&request, &document, None, &selector, None, None);
        assert!(unrestricted.diagnostics.is_empty());
        assert_eq!(unrestricted.matches.len(), 2);

        let match_payload_bytes = unrestricted
            .matches
            .iter()
            .map(super::super::match_output_payload_bytes)
            .collect::<Vec<_>>();
        let max_match_output_bytes = *match_payload_bytes.iter().max().expect("two matches");
        let max_total_output_bytes = match_payload_bytes.iter().sum();
        let bounded = run_prepared_selector_extraction(
            &request,
            &document,
            None,
            &selector,
            None,
            Some(ExecutionLimits {
                max_selector_work_units: 1_000,
                max_candidates: 8,
                max_selected_matches: 8,
                max_match_output_bytes,
                max_total_output_bytes,
            }),
        );

        assert!(bounded.diagnostics.is_empty());
        assert_eq!(bounded.candidate_count, 2);
        assert_eq!(bounded.matches.len(), 2);
    }
}
