//! Independent expectations for matching scratch, nesting and budget restoration.
use super::*;
use crate::parser::tests::DummySelectorImpl;

#[test]
fn link_modes_and_document_quirks_are_explicit() {
    for (mode, visited, unvisited) in [
        (VisitedHandlingMode::AllLinksUnvisited, false, true),
        (VisitedHandlingMode::AllLinksVisitedAndUnvisited, true, true),
        (VisitedHandlingMode::RelevantLinkVisited, true, false),
    ] {
        assert_eq!(mode.matches_visited(), visited);
        assert_eq!(mode.matches_unvisited(), unvisited);
    }
    assert_eq!(
        QuirksMode::Quirks.classes_and_ids_case_sensitivity(),
        CaseSensitivity::AsciiCaseInsensitive
    );
    for mode in [QuirksMode::NoQuirks, QuirksMode::LimitedQuirks] {
        assert_eq!(
            mode.classes_and_ids_case_sensitivity(),
            CaseSensitivity::CaseSensitive
        );
    }
}

#[test]
fn nested_context_restores_scope_negation_featureless_and_visited_modes() {
    let mut caches = SelectorCaches::default();
    let mut context = MatchingContext::<DummySelectorImpl>::new(
        MatchingMode::ForStatelessPseudoElement,
        None,
        &mut caches,
        QuirksMode::NoQuirks,
        NeedsSelectorFlags::Yes,
        MatchingForInvalidation::Yes,
    );
    assert!(context.needs_selector_flags());
    assert!(context.matching_for_invalidation());
    assert!(!context.matching_for_revalidation());
    assert_eq!(context.matching_for_invalidation_comparison(), Some(false));
    context.for_invalidation_comparison(|inner| {
        assert_eq!(inner.matching_for_invalidation_comparison(), Some(true));
    });
    assert_eq!(context.matching_for_invalidation_comparison(), Some(false));
    assert!(!context.is_nested());
    assert!(!context.in_negation());
    context.nest_for_negation(|inner| {
        assert!(inner.is_nested());
        assert!(inner.in_negation());
        inner.nest_for_negation(|double| {
            assert!(double.is_nested());
            assert!(!double.in_negation());
        });
        assert!(inner.in_negation());
    });
    assert!(!context.is_nested());
    assert!(!context.in_negation());
    assert!(!context.featureless());
    context.with_featureless(true, |inner| {
        assert!(inner.featureless());
        inner.with_featureless(false, |nested| assert!(!nested.featureless()));
        assert!(inner.featureless());
    });
    assert!(!context.featureless());
    context.with_visited_handling_mode(VisitedHandlingMode::RelevantLinkVisited, |inner| {
        assert_eq!(
            inner.visited_handling(),
            VisitedHandlingMode::RelevantLinkVisited
        );
    });
    assert_eq!(
        context.visited_handling(),
        VisitedHandlingMode::AllLinksUnvisited
    );
    let anchor_identity = 0_u8;
    let anchor = OpaqueElement::new(&anchor_identity);
    assert_eq!(context.relative_selector_anchor(), None);
    context.nest_for_relative_selector(anchor, |inner| {
        assert!(inner.is_nested());
        assert_eq!(inner.relative_selector_anchor(), Some(anchor));
    });
    assert_eq!(context.relative_selector_anchor(), None);
    assert!(!context.is_nested());
    context.nest_for_scope_condition(Some(anchor), |inner| {
        assert_eq!(inner.scope_element, Some(anchor));
        assert_eq!(inner.matching_mode(), MatchingMode::Normal);
    });
    assert_eq!(context.scope_element, None);
    assert_eq!(
        context.matching_mode(),
        MatchingMode::ForStatelessPseudoElement
    );
}

#[test]
fn revalidation_and_budgeting_preserve_their_distinct_contracts() {
    let mut caches = SelectorCaches::default();
    let mut context = MatchingContext::<DummySelectorImpl>::new_for_revalidation(
        None,
        &mut caches,
        QuirksMode::Quirks,
        NeedsSelectorFlags::No,
    );
    assert!(context.matching_for_revalidation());
    assert!(!context.matching_for_invalidation());
    assert_eq!(context.matching_for_invalidation_comparison(), None);
    assert!(!context.needs_selector_flags());
    assert_eq!(
        context.classes_and_ids_case_sensitivity(),
        CaseSensitivity::AsciiCaseInsensitive
    );
    assert!(context.consume_work());
    let budget = SelectorWorkBudget::new(1);
    context.set_work_budget(Some(&budget));
    assert!(context.consume_work());
    assert!(!context.consume_work());
    assert!(budget.exhausted());
    assert!(!context.consume_work());
    context.set_work_budget(None);
    assert!(context.consume_work());
}

#[test]
fn explicit_visited_context_keeps_mode_flags_and_quiet_link_policy() {
    let mut caches = SelectorCaches::default();
    let context = MatchingContext::<DummySelectorImpl>::new_for_visited(
        MatchingMode::Normal,
        None,
        &mut caches,
        VisitedHandlingMode::AllLinksVisitedAndUnvisited,
        QuirksMode::LimitedQuirks,
        NeedsSelectorFlags::No,
        MatchingForInvalidation::YesForComparison,
    );
    assert_eq!(
        context.visited_handling(),
        VisitedHandlingMode::AllLinksVisitedAndUnvisited
    );
    assert_eq!(context.matching_mode(), MatchingMode::Normal);
    assert_eq!(context.matching_for_invalidation_comparison(), Some(true));
    assert!(context.matching_for_invalidation());
    assert!(!context.matching_for_revalidation());
    assert!(!context.needs_selector_flags());
    assert_eq!(
        context.classes_and_ids_case_sensitivity(),
        CaseSensitivity::CaseSensitive
    );
}
