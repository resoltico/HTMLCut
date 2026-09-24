//! Same-snapshot selector synthesis and uniqueness proofing.

use std::num::NonZeroU32;

use scraper::{ElementRef, Selector};
use selectors::work_budget::SelectorWorkBudget;

use super::{
    CssSelectorText, MAX_SELECTOR_BYTES, SelectorProposal, SelectorStability,
    SelectorStabilityReason, inspect_element, structural_selector_path,
};

#[derive(Clone)]
struct ProposalCandidate {
    selector: String,
    stability: SelectorStability,
    reasons: Vec<SelectorStabilityReason>,
    rationale: &'static str,
}

pub(super) fn selector_proposals(
    document: &scraper::Html,
    element: &ElementRef<'_>,
    max_proposals: NonZeroU32,
    budget: &SelectorWorkBudget,
) -> (Vec<SelectorProposal>, bool) {
    selector_proposals_with(
        document,
        element,
        max_proposals,
        budget,
        uniquely_selects,
        structural_uniquely_selects,
    )
}

fn selector_proposals_with(
    document: &scraper::Html,
    element: &ElementRef<'_>,
    max_proposals: NonZeroU32,
    budget: &SelectorWorkBudget,
    mut prove_unique: impl FnMut(
        &scraper::Html,
        &Selector,
        ego_tree::NodeId,
        &SelectorWorkBudget,
    ) -> UniqueProof,
    mut prove_structural: impl FnMut(
        &scraper::Html,
        &Selector,
        ego_tree::NodeId,
        &SelectorWorkBudget,
    ) -> UniqueProof,
) -> (Vec<SelectorProposal>, bool) {
    let mut proposals = Vec::new();
    for candidate in proposal_candidates(element) {
        // `max_proposals` is non-zero and validated against the public bound of sixteen, so the
        // vector length can never approach a platform-width conversion boundary.
        if proposals.len() >= max_proposals.get() as usize {
            break;
        }
        if candidate.selector.len() > MAX_SELECTOR_BYTES {
            continue;
        }
        let selector = Selector::parse(&candidate.selector)
            .expect("HTMLCut-owned CSS serialization must yield valid selector grammar");
        match prove_unique(document, &selector, element.id(), budget) {
            UniqueProof::Proved => {
                let selector = CssSelectorText::new(candidate.selector).expect(
                    "bounded, parsed proposal selector must satisfy the public text contract",
                );
                proposals.push(SelectorProposal {
                    selector,
                    stability: candidate.stability,
                    reasons: candidate.reasons,
                    match_count: NonZeroU32::new(1).expect("one is non-zero"),
                    rationale: candidate.rationale.to_owned(),
                });
            }
            UniqueProof::NotUnique => {}
            UniqueProof::WorkTruncated => return (proposals, true),
        }
    }
    // A positional path is a fallback, not a second-choice recommendation after HTMLCut has
    // already proved a semantic, attribute, or class-based selector. Avoiding that redundant
    // proof also preserves the caller's shared work budget for later elements.
    if proposals.is_empty() {
        let structural = match structural_candidate(element, budget) {
            Ok(candidate) => candidate,
            Err(()) => return (proposals, true),
        };
        if structural.selector.len() <= MAX_SELECTOR_BYTES {
            let selector = Selector::parse(&structural.selector)
                .expect("HTMLCut-owned CSS serialization must yield valid selector grammar");
            match prove_structural(document, &selector, element.id(), budget) {
                UniqueProof::Proved => {
                    let selector = CssSelectorText::new(structural.selector).expect(
                        "bounded, parsed proposal selector must satisfy the public text contract",
                    );
                    proposals.push(SelectorProposal {
                        selector,
                        stability: structural.stability,
                        reasons: structural.reasons,
                        match_count: NonZeroU32::new(1).expect("one is non-zero"),
                        rationale: structural.rationale.to_owned(),
                    });
                }
                UniqueProof::NotUnique => {}
                UniqueProof::WorkTruncated => return (proposals, true),
            }
        }
    }
    (proposals, false)
}

enum UniqueProof {
    Proved,
    NotUnique,
    WorkTruncated,
}

fn structural_uniquely_selects(
    document: &scraper::Html,
    selector: &Selector,
    target: ego_tree::NodeId,
    budget: &SelectorWorkBudget,
) -> UniqueProof {
    let node = document
        .tree
        .get(target)
        .expect("a proposal target belongs to its prepared document");
    let element = ElementRef::wrap(node).expect("a proposal target is an element");
    let target_namespace: &str = element.value().name.ns.as_ref();
    if target_namespace != "http://www.w3.org/1999/xhtml" {
        return uniquely_selects(document, selector, target, budget);
    }
    for ancestor in node.ancestors().filter_map(ElementRef::wrap) {
        if inspect_element(budget).is_err() {
            return UniqueProof::WorkTruncated;
        }
        let ancestor_namespace: &str = ancestor.value().name.ns.as_ref();
        if ancestor_namespace != "http://www.w3.org/1999/xhtml" {
            return uniquely_selects(document, selector, target, budget);
        }
    }
    // The complete HTML ancestry chain starts at the parser's unique root and each direct-child
    // nth-of-type step is unique under its already-unique parent. Check the actual CSS projection
    // against the target before using that tree invariant as the uniqueness proof.
    match selector.matches_with_budget(&element, budget) {
        Ok(true) => UniqueProof::Proved,
        Ok(false) => UniqueProof::NotUnique,
        Err(_) => UniqueProof::WorkTruncated,
    }
}

fn uniquely_selects(
    document: &scraper::Html,
    selector: &Selector,
    target: ego_tree::NodeId,
    budget: &SelectorWorkBudget,
) -> UniqueProof {
    let mut matching_target = false;
    let mut match_count = 0_u32;
    for node in document.tree.root().descendants() {
        let Some(candidate) = ElementRef::wrap(node) else {
            continue;
        };
        if inspect_element(budget).is_err() {
            return UniqueProof::WorkTruncated;
        }
        match selector.matches_with_budget(&candidate, budget) {
            Ok(true) => {
                match_count = match_count.saturating_add(1);
                matching_target |= candidate.id() == target;
                if match_count > 1 {
                    return UniqueProof::NotUnique;
                }
            }
            Ok(false) => {}
            Err(_) => return UniqueProof::WorkTruncated,
        }
    }
    if matching_target {
        // `matching_target` is set only in the same branch that increments `match_count`, and
        // the early return above has already rejected every count above one.
        UniqueProof::Proved
    } else {
        UniqueProof::NotUnique
    }
}

#[cfg(test)]
pub(crate) fn selector_is_unique_for_tests(
    document: &scraper::Html,
    selector: &Selector,
    target: ego_tree::NodeId,
    work_units: u32,
) -> bool {
    matches!(
        uniquely_selects(
            document,
            selector,
            target,
            &SelectorWorkBudget::new(work_units),
        ),
        UniqueProof::Proved
    )
}

fn proposal_candidates(element: &ElementRef<'_>) -> Vec<ProposalCandidate> {
    let tag_name = css_identifier(element.value().name());
    let mut candidates = Vec::new();
    if let Some(id) = element
        .value()
        .attr("id")
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let generated = generated_looking(id);
        candidates.push(ProposalCandidate {
            selector: format!("#{}", css_identifier(id)),
            stability: if generated {
                SelectorStability::Structural
            } else {
                SelectorStability::Semantic
            },
            reasons: if generated {
                vec![
                    SelectorStabilityReason::GeneratedLookingIdentifier,
                    SelectorStabilityReason::UniqueOnSnapshot,
                ]
            } else {
                vec![
                    SelectorStabilityReason::UniqueId,
                    SelectorStabilityReason::UniqueOnSnapshot,
                ]
            },
            rationale: if generated {
                "A generated-looking ID is unique only on this snapshot."
            } else {
                "A stable unique ID identifies this element."
            },
        });
    }
    for (attribute, reason, stability) in [
        (
            "itemprop",
            SelectorStabilityReason::MicrodataAttribute,
            SelectorStability::StableAttribute,
        ),
        (
            "role",
            SelectorStabilityReason::SemanticAttribute,
            SelectorStability::StableAttribute,
        ),
        (
            "aria-label",
            SelectorStabilityReason::AriaAttribute,
            SelectorStability::StableAttribute,
        ),
    ] {
        if let Some(value) = element
            .value()
            .attr(attribute)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            candidates.push(ProposalCandidate {
                selector: format!("{tag_name}[{attribute}={}]", css_string(value)),
                stability,
                reasons: vec![reason, SelectorStabilityReason::UniqueOnSnapshot],
                rationale: "A semantic attribute identifies this element on this snapshot.",
            });
        }
    }
    let mut data_attributes = element
        .value()
        .attrs()
        .filter(|(name, value)| {
            name.starts_with("data-") && !value.trim().is_empty() && !generated_looking(value)
        })
        .map(|(name, value)| (name.to_owned(), value.to_owned()))
        .collect::<Vec<_>>();
    data_attributes.sort();
    data_attributes.truncate(10);
    for (name, value) in data_attributes {
        candidates.push(ProposalCandidate {
            selector: format!("{tag_name}[{name}={}]", css_string(&value)),
            stability: SelectorStability::StableAttribute,
            reasons: vec![
                SelectorStabilityReason::SemanticAttribute,
                SelectorStabilityReason::UniqueOnSnapshot,
            ],
            rationale: "A stable data attribute identifies this element on this snapshot.",
        });
    }
    let classes = element
        .value()
        .attr("class")
        .into_iter()
        .flat_map(str::split_whitespace)
        .filter(|class| !class.is_empty())
        .take(2)
        .collect::<Vec<_>>();
    if !classes.is_empty() {
        let generated = classes.iter().any(|class| generated_looking(class));
        let class_suffix = classes
            .iter()
            .map(|class| format!(".{}", css_identifier(class)))
            .collect::<String>();
        candidates.push(ProposalCandidate {
            selector: format!("{tag_name}{class_suffix}"),
            stability: if generated {
                SelectorStability::Structural
            } else {
                SelectorStability::ClassBased
            },
            reasons: if generated {
                vec![
                    SelectorStabilityReason::GeneratedLookingClass,
                    SelectorStabilityReason::UniqueOnSnapshot,
                ]
            } else {
                vec![
                    SelectorStabilityReason::StableClass,
                    SelectorStabilityReason::UniqueOnSnapshot,
                ]
            },
            rationale: "A class combination identifies this element on this snapshot.",
        });
    }
    candidates.sort_by(|left, right| {
        left.stability
            .cmp(&right.stability)
            .then_with(|| left.selector.len().cmp(&right.selector.len()))
            .then_with(|| left.selector.cmp(&right.selector))
    });
    candidates.dedup_by(|left, right| left.selector == right.selector);
    candidates
}

fn structural_candidate(
    element: &ElementRef<'_>,
    budget: &SelectorWorkBudget,
) -> Result<ProposalCandidate, ()> {
    let selector = structural_selector_path(element, budget).map_err(|_| ())?;
    Ok(ProposalCandidate {
        selector,
        stability: SelectorStability::Positional,
        reasons: vec![
            SelectorStabilityReason::StructuralPath,
            SelectorStabilityReason::NthDependency,
            SelectorStabilityReason::UniqueOnSnapshot,
        ],
        rationale: "An exact positional path identifies this element on this snapshot.",
    })
}

fn css_identifier(value: &str) -> String {
    let mut serialized = String::new();
    cssparser::serialize_identifier(value, &mut serialized)
        .expect("string-backed CSS identifier serialization cannot fail");
    serialized
}

fn css_string(value: &str) -> String {
    let mut serialized = String::new();
    cssparser::serialize_string(value, &mut serialized)
        .expect("string-backed CSS string serialization cannot fail");
    serialized
}

fn generated_looking(value: &str) -> bool {
    let digits = value.bytes().filter(u8::is_ascii_digit).count();
    let hex = value.bytes().filter(u8::is_ascii_hexdigit).count();
    let long_enough = value.len() >= 16;
    let digit_heavy = digits >= 6;
    let hexadecimal = hex == value.len();

    // These independent signals are pure and cheap. Evaluate each one so the classification is
    // an auditable conjunction of the documented length and composition criteria, rather than a
    // control-flow-dependent sequence of partial checks.
    long_enough & (digit_heavy | hexadecimal)
}

#[cfg(test)]
mod tests;
