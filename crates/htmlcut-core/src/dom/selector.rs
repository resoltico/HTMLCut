// SPDX-License-Identifier: MPL-2.0
//! Upstream CSS matching with product traversal accounting.
use crate::budget::WorkBudget;
use crate::dom::{ElementRef, Html};
use selectors::{
    matching,
    parser::{ParseRelative, SelectorList},
};
#[path = "selector_depth.rs"]
mod depth;
#[derive(Debug, Clone)]
pub(crate) struct Selector(SelectorList<scraper::selector::Simple>);
#[derive(Clone, Copy, Debug)]
pub(crate) enum SelectorMatchError {
    WorkLimitExceeded,
    DocumentMismatch,
}
pub(crate) struct BudgetedMatcher<'s, 'd> {
    selector: &'s Selector,
    document: &'d Html,
    scope: Option<ElementRef<'d>>,
    budget: &'s WorkBudget,
    caches: matching::SelectorCaches,
}
impl Selector {
    #[cfg(test)]
    pub fn test_matches(&self, e: &ElementRef<'_>) -> bool {
        let mut caches = matching::SelectorCaches::default();
        self.matches_upstream(e, None, &mut caches)
    }
    pub fn parse(value: &str) -> Result<Self, crate::ExtractionError> {
        let mut input = cssparser::ParserInput::new(value);
        let mut parser = cssparser::Parser::new(&mut input);
        let list = SelectorList::parse(&scraper::selector::Parser, &mut parser, ParseRelative::No)
            .map_err(|_| {
                crate::ExtractionError::new(
                    crate::ErrorCode::InvalidSelector,
                    "compilation",
                    "The CSS selector is invalid or unsupported.",
                )
            })?;
        depth::admit(&list)?;
        Ok(Self(list))
    }
    fn matches_upstream(
        &self,
        e: &ElementRef<'_>,
        scope: Option<ElementRef<'_>>,
        caches: &mut matching::SelectorCaches,
    ) -> bool {
        let element = scraper::ElementRef::wrap(e.node).unwrap();
        let mut context = matching::MatchingContext::new(
            matching::MatchingMode::Normal,
            None,
            caches,
            matching::QuirksMode::NoQuirks,
            matching::NeedsSelectorFlags::No,
            matching::MatchingForInvalidation::No,
        );
        context.scope_element =
            scope.map(|s| selectors::Element::opaque(&scraper::ElementRef::wrap(s.node).unwrap()));
        self.0
            .slice()
            .iter()
            .any(|s| matching::matches_selector(s, 0, None, &element, &mut context))
    }
    pub fn budgeted<'s, 'd>(
        &'s self,
        document: &'d Html,
        scope: Option<ElementRef<'d>>,
        budget: &'s WorkBudget,
    ) -> Result<BudgetedMatcher<'s, 'd>, SelectorMatchError> {
        if scope.is_some_and(|s| !std::ptr::eq(s.tree(), &document.tree)) {
            return Err(SelectorMatchError::DocumentMismatch);
        }
        Ok(BudgetedMatcher {
            selector: self,
            document,
            scope,
            budget,
            caches: Default::default(),
        })
    }
}
impl<'d> BudgetedMatcher<'_, 'd> {
    pub fn matches(&mut self, element: &ElementRef<'d>) -> Result<bool, SelectorMatchError> {
        if !std::ptr::eq(element.tree(), &self.document.tree) {
            return Err(SelectorMatchError::DocumentMismatch);
        }
        if !self.budget.consume() {
            return Err(SelectorMatchError::WorkLimitExceeded);
        }
        Ok(self
            .selector
            .matches_upstream(element, self.scope, &mut self.caches))
    }
}
