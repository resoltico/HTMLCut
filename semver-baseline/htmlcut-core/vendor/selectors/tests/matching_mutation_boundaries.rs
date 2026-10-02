//! Complete value and work controls for paths exposed by exact-source mutants.
use super::*;

#[test]
fn subtree_unwind_reaches_later_descendants_without_escaping_the_anchor() {
    let tree = Tree::new(vec![
        Record {
            name: "main",
            ..Record::default()
        },
        Record {
            name: "section",
            parent: Some(0),
            ..Record::default()
        },
        Record {
            name: "div",
            parent: Some(1),
            ..Record::default()
        },
        Record {
            name: "p",
            parent: Some(2),
            ..Record::default()
        },
        Record {
            name: "span",
            classes: &["inside"],
            parent: Some(1),
            ..Record::default()
        },
        Record {
            name: "aside",
            classes: &["outside"],
            parent: Some(0),
            ..Record::default()
        },
    ]);
    for (css, expected) in [(".inside", true), (".outside", false)] {
        let selector = parse(css);
        let mut caches = SelectorCaches::default();
        let mut context = MatchingContext::new(
            MatchingMode::Normal,
            None,
            &mut caches,
            QuirksMode::NoQuirks,
            NeedsSelectorFlags::No,
            MatchingForInvalidation::No,
        );
        assert_eq!(
            matches_relative_selector_subtree(
                &selector.slice()[0],
                &tree.element(1),
                &mut context,
                SubjectOrPseudoElement::Yes
            ),
            expected,
            "{css}"
        );
    }
}

#[test]
fn leaf_unwind_requires_its_own_work_unit() {
    let tree = Tree::new(vec![
        Record {
            name: "main",
            ..Record::default()
        },
        Record {
            name: "p",
            parent: Some(0),
            ..Record::default()
        },
    ]);
    let selector = parse(".absent");
    // One unit visits the child, one enters its complex selector, then leaf unwind needs one.
    let budget = crate::work_budget::SelectorWorkBudget::new(2);
    let mut caches = SelectorCaches::default();
    let mut context = MatchingContext::new(
        MatchingMode::Normal,
        None,
        &mut caches,
        QuirksMode::NoQuirks,
        NeedsSelectorFlags::No,
        MatchingForInvalidation::No,
    );
    context.set_work_budget(Some(&budget));
    assert!(!matches_relative_selector_subtree(
        &selector.slice()[0],
        &tree.element(0),
        &mut context,
        SubjectOrPseudoElement::Yes
    ));
    assert!(budget.exhausted());
}

#[test]
fn only_child_requires_both_first_and_last_positions() {
    let tree = Tree::new(vec![
        Record {
            name: "main",
            ..Record::default()
        },
        Record {
            name: "p",
            parent: Some(0),
            ..Record::default()
        },
        Record {
            name: "p",
            parent: Some(0),
            ..Record::default()
        },
    ]);
    let selectors = parse(":only-child");
    let mut caches = SelectorCaches::default();
    let mut context = MatchingContext::new(
        MatchingMode::Normal,
        None,
        &mut caches,
        QuirksMode::NoQuirks,
        NeedsSelectorFlags::No,
        MatchingForInvalidation::No,
    );
    for index in [1, 2] {
        assert!(!matches_selector_list(
            &selectors,
            &tree.element(index),
            &mut context
        ));
    }
}

#[test]
fn relative_filter_has_independent_rejection_and_budget_oracles() {
    use crate::relative_selector::filter::RelativeSelectorFilterMap;
    let tree = Tree::new(vec![
        Record {
            name: "main",
            ..Record::default()
        },
        Record {
            name: "p",
            classes: &["present"],
            parent: Some(0),
            ..Record::default()
        },
        Record {
            name: "p",
            parent: Some(0),
            ..Record::default()
        },
    ]);
    let selectors = parse(":has(.absent)");
    let Component::Has(relative) = selectors.slice()[0].iter().next().unwrap() else {
        panic!("relative fixture")
    };
    let mut filters = RelativeSelectorFilterMap::default();
    assert!(!filters.fast_reject(&tree.element(0), &relative[0], QuirksMode::NoQuirks, None));
    assert!(filters.fast_reject(&tree.element(0), &relative[0], QuirksMode::NoQuirks, None));
    assert_eq!(tree.hash_visits.get(), 2);
    tree.hash_visits.set(0);
    let mut filters = RelativeSelectorFilterMap::default();
    let budget = crate::work_budget::SelectorWorkBudget::new(1);
    assert!(!filters.fast_reject(
        &tree.element(0),
        &relative[0],
        QuirksMode::NoQuirks,
        Some(&budget)
    ));
    assert!(!filters.fast_reject(
        &tree.element(0),
        &relative[0],
        QuirksMode::NoQuirks,
        Some(&budget)
    ));
    assert!(budget.exhausted());
    assert_eq!(tree.hash_visits.get(), 1);
    // An incomplete filter must not be installed; a fresh budget can rebuild the whole filter.
    let broad = crate::work_budget::SelectorWorkBudget::new(10);
    assert!(filters.fast_reject(
        &tree.element(0),
        &relative[0],
        QuirksMode::NoQuirks,
        Some(&broad)
    ));
    assert_eq!(tree.hash_visits.get(), 3);
    assert_eq!(broad.remaining(), 8);
}

#[test]
fn subject_and_ancestor_relative_anchors_receive_distinct_flags() {
    let tree = Tree::new(vec![
        Record {
            name: "main",
            ..Record::default()
        },
        Record {
            name: "p",
            parent: Some(0),
            ..Record::default()
        },
    ]);
    let selectors = parse(":has(p)");
    let Component::Has(relative) = selectors.slice()[0].iter().next().unwrap() else {
        panic!("relative fixture")
    };
    for (subject, expected) in [
        (
            SubjectOrPseudoElement::Yes,
            ElementSelectorFlags::ANCHORS_RELATIVE_SELECTOR,
        ),
        (
            SubjectOrPseudoElement::No,
            ElementSelectorFlags::ANCHORS_RELATIVE_SELECTOR_NON_SUBJECT,
        ),
    ] {
        tree.flags.borrow_mut().fill(0);
        let mut caches = SelectorCaches::default();
        let mut context = MatchingContext::new(
            MatchingMode::Normal,
            None,
            &mut caches,
            QuirksMode::NoQuirks,
            NeedsSelectorFlags::Yes,
            MatchingForInvalidation::No,
        );
        context.nest_for_relative_selector(tree.element(0).opaque(), |context| {
            assert!(do_match_relative_selectors(
                relative,
                &tree.element(0),
                context,
                subject
            ));
        });
        assert_eq!(
            tree.flags.borrow()[0],
            expected
                .union(ElementSelectorFlags::RELATIVE_SELECTOR_SEARCH_DIRECTION_ANCESTOR)
                .bits()
        );
    }
}

#[test]
fn ancestor_combinators_do_not_receive_sibling_invalidation_flags() {
    let tree = Tree::new(vec![
        Record {
            name: "main",
            ..Record::default()
        },
        Record {
            name: "p",
            parent: Some(0),
            ..Record::default()
        },
        Record {
            name: "p",
            parent: Some(0),
            ..Record::default()
        },
    ]);
    for (css, needs, expected) in [
        ("main p", NeedsSelectorFlags::Yes, 0),
        (
            "p + p",
            NeedsSelectorFlags::Yes,
            ElementSelectorFlags::HAS_SLOW_SELECTOR_LATER_SIBLINGS.bits(),
        ),
        ("p + p", NeedsSelectorFlags::No, 0),
    ] {
        tree.flags.borrow_mut().fill(0);
        let selectors = parse(css);
        let mut caches = SelectorCaches::default();
        let mut context = MatchingContext::new(
            MatchingMode::Normal,
            None,
            &mut caches,
            QuirksMode::NoQuirks,
            needs,
            MatchingForInvalidation::No,
        );
        assert!(matches_selector_list(
            &selectors,
            &tree.element(2),
            &mut context
        ));
        assert_eq!(tree.flags.borrow()[0], expected, "{css}");
        assert_eq!(tree.flags.borrow()[2], 0, "parent-scoped sibling flags");
    }
}

#[test]
fn ancestor_has_is_non_subject_after_an_ordinary_combinator() {
    let tree = Tree::new(vec![
        Record {
            name: "main",
            ..Record::default()
        },
        Record {
            name: "p",
            parent: Some(0),
            ..Record::default()
        },
    ]);
    let selectors = parse("main:has(p) > p");
    let mut caches = SelectorCaches::default();
    let mut context = MatchingContext::new(
        MatchingMode::Normal,
        None,
        &mut caches,
        QuirksMode::NoQuirks,
        NeedsSelectorFlags::Yes,
        MatchingForInvalidation::No,
    );
    assert!(matches_selector_list(
        &selectors,
        &tree.element(1),
        &mut context
    ));
    assert_eq!(
        tree.flags.borrow()[0],
        ElementSelectorFlags::ANCHORS_RELATIVE_SELECTOR_NON_SUBJECT
            .union(ElementSelectorFlags::RELATIVE_SELECTOR_SEARCH_DIRECTION_ANCESTOR)
            .bits()
    );
}

#[path = "matching_position_boundaries.rs"]
mod positions;
