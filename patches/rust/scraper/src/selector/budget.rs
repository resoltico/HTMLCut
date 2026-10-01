//! Fallible selector evaluation over the maintained HTMLCut work budget.

use selectors::{
    matching::{self, SelectorCaches},
    work_budget::SelectorWorkBudget,
};

use super::Selector;
use crate::{ElementRef, Html};

/// Closed refusal reasons for one document-bound selector evaluation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectorMatchError {
    /// Finite matching work was exhausted.
    WorkLimitExceeded,
    /// A scope or candidate belongs to a different document.
    DocumentMismatch,
}

/// Matching scratch tied to one immutable selector, document scope and operation budget.
pub struct BudgetedMatcher<'selector, 'document> {
    selector: &'selector Selector,
    document: &'document Html,
    scope: Option<ElementRef<'document>>,
    budget: &'selector SelectorWorkBudget,
    caches: SelectorCaches,
}

impl std::fmt::Debug for BudgetedMatcher<'_, '_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("BudgetedMatcher")
            .field("selector", &self.selector)
            .field("scope", &self.scope)
            .field("remaining_work", &self.budget.remaining())
            .finish_non_exhaustive()
    }
}

impl Selector {
    /// Starts one scoped candidate pass with shared selector caches and work accounting.
    ///
    /// The document stays borrowed while scratch is usable:
    ///
    /// ```
    /// use scraper::{Html, Selector};
    /// use selectors::work_budget::SelectorWorkBudget;
    /// let document = Html::parse_document("<p>A</p>");
    /// let selector = Selector::parse("p").unwrap();
    /// let budget = SelectorWorkBudget::new(100);
    /// let mut matcher = selector.budgeted(&document, None, &budget).unwrap();
    /// let element = document.select(&selector).next().unwrap();
    /// assert!(matcher.matches(&element).unwrap());
    /// ```
    ///
    /// ```compile_fail
    /// use scraper::{Html, Selector};
    /// use selectors::work_budget::SelectorWorkBudget;
    /// let document = Html::parse_document("<p>A</p>");
    /// let other = Html::parse_document("<p>B</p>");
    /// let selector = Selector::parse("p").unwrap();
    /// let budget = SelectorWorkBudget::new(100);
    /// let mut matcher = selector.budgeted(&document, None, &budget).unwrap();
    /// drop(document);
    /// let element = other.select(&selector).next().unwrap();
    /// let _ = matcher.matches(&element);
    /// ```
    pub fn budgeted<'selector, 'document>(
        &'selector self,
        document: &'document Html,
        scope: Option<ElementRef<'document>>,
        budget: &'selector SelectorWorkBudget,
    ) -> Result<BudgetedMatcher<'selector, 'document>, SelectorMatchError> {
        if scope.is_some_and(|scope| !std::ptr::eq(scope.tree(), &document.tree)) {
            return Err(SelectorMatchError::DocumentMismatch);
        }
        Ok(BudgetedMatcher {
            selector: self,
            document,
            scope,
            budget,
            caches: SelectorCaches::default(),
        })
    }
}

impl<'document> BudgetedMatcher<'_, 'document> {
    /// Matches one candidate, retaining scratch only for this selector and original scope.
    pub fn matches(&mut self, element: &ElementRef<'document>) -> Result<bool, SelectorMatchError> {
        if !std::ptr::eq(element.tree(), &self.document.tree) {
            return Err(SelectorMatchError::DocumentMismatch);
        }
        if !self.budget.consume() {
            return Err(SelectorMatchError::WorkLimitExceeded);
        }
        let mut context = matching::MatchingContext::new(
            matching::MatchingMode::Normal,
            None,
            &mut self.caches,
            matching::QuirksMode::NoQuirks,
            matching::NeedsSelectorFlags::No,
            matching::MatchingForInvalidation::No,
        );
        context.scope_element = self
            .scope
            .map(|element| selectors::Element::opaque(&element));
        context.set_work_budget(Some(self.budget));
        let matches =
            self.selector.selectors.slice().iter().any(|selector| {
                matching::matches_selector(selector, 0, None, element, &mut context)
            });
        if self.budget.exhausted() {
            Err(SelectorMatchError::WorkLimitExceeded)
        } else {
            Ok(matches)
        }
    }
}

#[cfg(test)]
#[path = "tests/budget.rs"]
mod tests;
