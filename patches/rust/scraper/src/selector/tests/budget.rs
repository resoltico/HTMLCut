//! Document ownership, cache reuse and typed work-limit controls.

use selectors::work_budget::SelectorWorkBudget;

use super::{Selector, SelectorMatchError};
use crate::ElementRef;

#[test]
fn one_matcher_refuses_candidates_from_a_different_document() {
    let first = crate::Html::parse_document("<main><p>A</p><p>B</p></main>");
    let second = crate::Html::parse_document("<main><p>C</p><p>D</p></main>");
    let scope = first
        .select(&Selector::parse("main").unwrap())
        .next()
        .unwrap();
    let candidate = second
        .select(&Selector::parse("p").unwrap())
        .last()
        .unwrap();
    let selector = Selector::parse("p:nth-child(2)").unwrap();
    let budget = SelectorWorkBudget::new(1000);
    let mut matcher = selector.budgeted(&first, Some(scope), &budget).unwrap();
    assert!(matcher.matches(&candidate).is_err());
}

#[test]
fn matcher_debug_identifies_scope_and_remaining_work() {
    let document = crate::Html::parse_document("<main><p>A</p></main>");
    let main = document
        .select(&Selector::parse("main").unwrap())
        .next()
        .unwrap();
    let selector = Selector::parse("p").unwrap();
    let budget = SelectorWorkBudget::new(10);
    let diagnostic = format!(
        "{:?}",
        selector.budgeted(&document, Some(main), &budget).unwrap()
    );
    assert!(diagnostic.contains("BudgetedMatcher"));
    assert!(diagnostic.contains("remaining_work: 10"));
    assert!(diagnostic.contains("scope: Some"));
    assert_eq!(budget.remaining(), 10);
}

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
            .budgeted(&document, None, &SelectorWorkBudget::new(10))
            .unwrap()
            .matches(&article),
        Ok(true)
    );
    assert_eq!(
        Selector::parse("aside")
            .expect("selector")
            .budgeted(&document, None, &SelectorWorkBudget::new(10))
            .unwrap()
            .matches(&article),
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
        selector
            .budgeted(&document, None, &budget)
            .unwrap()
            .matches(&element),
        Err(SelectorMatchError::WorkLimitExceeded)
    );
}

#[test]
fn mixed_scope_is_refused_before_work_or_cache_initialization() {
    let first = crate::Html::parse_document("<main><p>A</p></main>");
    let second = crate::Html::parse_document("<main><p>B</p></main>");
    let scope = second
        .select(&Selector::parse("main").unwrap())
        .next()
        .unwrap();
    let selector = Selector::parse("p").unwrap();
    let budget = SelectorWorkBudget::new(100);
    assert_eq!(
        selector.budgeted(&first, Some(scope), &budget).err(),
        Some(SelectorMatchError::DocumentMismatch)
    );
    assert_eq!(budget.remaining(), 100);
    assert!(!budget.exhausted());
}

#[test]
fn fresh_document_and_scope_passes_do_not_share_positional_scratch() {
    for html in [
        "<main><section id='a'><p id='first'></p><p id='second'></p></section><section id='b'><p id='only'></p></section></main>",
        "<main><section id='b'><p id='only'></p></section><section id='a'><p id='first'></p><p id='second'></p></section></main>",
    ] {
        let document = crate::Html::parse_document(html);
        let selector = Selector::parse(":scope > p:nth-child(2)").unwrap();
        for (scope_id, expected) in [("a", vec!["second"]), ("b", vec![]), ("a", vec!["second"])] {
            let scope = document
                .select(&Selector::parse(&format!("#{scope_id}")).unwrap())
                .next()
                .unwrap();
            let budget = SelectorWorkBudget::new(1000);
            let mut matcher = selector.budgeted(&document, Some(scope), &budget).unwrap();
            let selected: Vec<_> = document
                .select(&Selector::parse("p").unwrap())
                .filter(|element| matcher.matches(element).unwrap())
                .map(|element| element.value().attr("id").unwrap())
                .collect();
            assert_eq!(selected, expected);
            assert!(!budget.exhausted());
        }
    }
}
