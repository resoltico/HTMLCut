// SPDX-License-Identifier: MPL-2.0
//! Explicit original-DOM anchor and a bounded forest of selected payload subtrees.

use std::collections::HashSet;

use ego_tree::NodeId;
use scraper::ElementRef;
use selectors::work_budget::SelectorWorkBudget;

use crate::{ErrorCode, ExtractionError};

pub(crate) struct SelectionScope<'a> {
    pub(super) anchor: ElementRef<'a>,
    pub(super) following_siblings: Vec<ElementRef<'a>>,
}

pub(super) fn selected_scopes<'a>(
    selected: &[ElementRef<'a>],
    candidates: &[ElementRef<'a>],
    following: u32,
    budget: &SelectorWorkBudget,
) -> Result<Vec<SelectionScope<'a>>, ExtractionError> {
    let candidate_ids = if following > 0 {
        super::charge(budget, candidates.len())?;
        candidates.iter().map(|node| node.id()).collect()
    } else {
        HashSet::new()
    };
    selected
        .iter()
        .enumerate()
        .map(|(index, anchor)| {
            scope(*anchor, following, &candidate_ids, budget).map_err(|mut error| {
                error.row_index = Some(index as u32 + 1);
                error
            })
        })
        .collect()
}

fn scope<'a>(
    anchor: ElementRef<'a>,
    following: u32,
    candidates: &HashSet<NodeId>,
    budget: &SelectorWorkBudget,
) -> Result<SelectionScope<'a>, ExtractionError> {
    let mut following_siblings = Vec::with_capacity(following as usize);
    let mut cursor = anchor.next_sibling();
    while following_siblings.len() < following as usize {
        let element = loop {
            let Some(node) = cursor else {
                return Err(ExtractionError::new(
                    ErrorCode::MissingRowSibling,
                    "row_scope",
                    "The declared row group lacks a following element sibling.",
                ));
            };
            super::charge(budget, 1)?;
            cursor = node.next_sibling();
            if let Some(element) = ElementRef::wrap(node) {
                break element;
            }
        };
        for node in element.descendants() {
            super::charge(budget, 1)?;
            if candidates.contains(&node.id()) {
                return Err(ExtractionError::new(
                    ErrorCode::OverlappingRowScope,
                    "row_scope",
                    "An added row subtree contains another root candidate.",
                ));
            }
        }
        following_siblings.push(element);
    }
    Ok(SelectionScope {
        anchor,
        following_siblings,
    })
}

#[cfg(test)]
mod scope_budget_tests {
    use super::*;

    #[test]
    fn anchor_only_scope_needs_no_candidate_set_or_expansion_pass() {
        let document = scraper::Html::parse_document("<article>A</article><article>B</article>");
        let selector = scraper::Selector::parse("article").unwrap();
        let roots = document.select(&selector).collect::<Vec<_>>();
        let budget = SelectorWorkBudget::new(1);
        let scopes = selected_scopes(&roots, &roots, 0, &budget).unwrap();
        assert_eq!(scopes.len(), 2);
        for (scope, anchor) in scopes.iter().zip(&roots) {
            assert_eq!(scope.anchor.id(), anchor.id());
            assert!(scope.following_siblings.is_empty());
        }
        assert_eq!(budget.remaining(), 1);
    }
}
