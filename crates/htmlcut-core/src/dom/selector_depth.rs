// SPDX-License-Identifier: MPL-2.0
//! Bounded, iterative admission of parsed matching paths before recursive execution.
use super::StaticSelectors;
use crate::{ExtractionError, limits::MAX_SELECTOR_MATCHING_DEPTH};
use selectors::parser::{Component, SelectorList};

pub(super) fn admit(list: &SelectorList<StaticSelectors>) -> Result<(), ExtractionError> {
    // Components are in right-to-left matching order. A nested branch inherits
    // only its containing compound's active depth, not later leftward compounds.
    // Borrowed work items avoid recursion in this admission walk itself.
    let mut pending = list.slice().iter().map(|s| (s, 0)).collect::<Vec<_>>();
    while let Some((selector, inherited)) = pending.pop() {
        let mut depth = inherited + 1;
        check(depth)?;
        for component in selector.iter_raw_match_order() {
            match component {
                Component::Combinator(_) => {
                    depth += 1;
                    check(depth)?;
                }
                Component::Is(branches)
                | Component::Where(branches)
                | Component::Negation(branches) => {
                    pending.extend(branches.slice().iter().map(|s| (s, depth)));
                }
                Component::Has(branches) => {
                    // Relative selectors include their synthetic anchor and
                    // combinator; matching visits that parsed sequence too.
                    pending.extend(branches.iter().map(|r| (&r.selector, depth)));
                }
                Component::Host(Some(branch)) | Component::Slotted(branch) => {
                    pending.push((branch, depth));
                }
                Component::NthOf(branches) => {
                    pending.extend(branches.selectors().iter().map(|s| (s, depth)));
                }
                Component::LocalName(_)
                | Component::ID(_)
                | Component::Class(_)
                | Component::AttributeInNoNamespaceExists { .. }
                | Component::AttributeInNoNamespace { .. }
                | Component::AttributeOther(_)
                | Component::ExplicitUniversalType
                | Component::ExplicitAnyNamespace
                | Component::ExplicitNoNamespace
                | Component::DefaultNamespace(_)
                | Component::Namespace(_, _)
                | Component::Root
                | Component::Empty
                | Component::Scope
                | Component::ImplicitScope
                | Component::ParentSelector
                | Component::Nth(_)
                | Component::NonTSPseudoClass(_)
                | Component::Part(_)
                | Component::Host(None)
                | Component::Invalid(_)
                | Component::PseudoElement(_)
                | Component::RelativeSelectorAnchor => {}
            }
        }
    }
    Ok(())
}

fn check(depth: usize) -> Result<(), ExtractionError> {
    if depth > MAX_SELECTOR_MATCHING_DEPTH {
        return Err(ExtractionError::resource(
            "compilation",
            "selector_matching_depth",
            MAX_SELECTOR_MATCHING_DEPTH as u64,
        ));
    }
    Ok(())
}
