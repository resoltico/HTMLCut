//! Fallible selector evaluation over the maintained HTMLCut work budget.

use selectors::{
    matching::{self, SelectorCaches},
    work_budget::SelectorWorkBudget,
};

use super::Selector;
use crate::ElementRef;

/// Failure returned when one selector evaluation exhausts its explicit work budget.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SelectorWorkLimitExceeded;

impl Selector {
    /// Returns whether this selector matches one element within an explicit work budget.
    pub fn matches_with_budget(
        &self,
        element: &ElementRef,
        budget: &SelectorWorkBudget,
    ) -> Result<bool, SelectorWorkLimitExceeded> {
        let mut caches = SelectorCaches::default();
        let mut context = matching::MatchingContext::new(
            matching::MatchingMode::Normal,
            None,
            &mut caches,
            matching::QuirksMode::NoQuirks,
            matching::NeedsSelectorFlags::No,
            matching::MatchingForInvalidation::No,
        );
        context.set_work_budget(Some(budget));
        let matches =
            self.selectors.slice().iter().any(|selector| {
                matching::matches_selector(selector, 0, None, element, &mut context)
            });
        if budget.exhausted() {
            Err(SelectorWorkLimitExceeded)
        } else {
            Ok(matches)
        }
    }
}

#[cfg(test)]
mod tests {
    use selectors::work_budget::SelectorWorkBudget;

    use super::{Selector, SelectorWorkLimitExceeded};
    use crate::ElementRef;

    #[test]
    fn budgeted_matching_returns_match_and_non_match_before_exhaustion() {
        let document = crate::Html::parse_document("<main><article>One</article></main>");
        let article = document
            .tree
            .root()
            .descendants()
            .filter_map(ElementRef::wrap)
            .find(|element| element.value().name() == "article")
            .expect("article element");

        assert_eq!(
            Selector::parse("article")
                .expect("selector")
                .matches_with_budget(&article, &SelectorWorkBudget::new(10)),
            Ok(true)
        );
        assert_eq!(
            Selector::parse("aside")
                .expect("selector")
                .matches_with_budget(&article, &SelectorWorkBudget::new(10)),
            Ok(false)
        );
    }

    #[test]
    fn budgeted_matching_reports_exhaustion_instead_of_non_match() {
        let document = crate::Html::parse_document("<main><article>One</article></main>");
        let element = document
            .tree
            .root()
            .descendants()
            .filter_map(ElementRef::wrap)
            .find(|element| element.value().name() == "article")
            .expect("article element");
        let selector = Selector::parse("article").expect("selector");
        let budget = SelectorWorkBudget::new(1);

        assert_eq!(
            selector.matches_with_budget(&element, &budget),
            Err(SelectorWorkLimitExceeded)
        );
    }
}
