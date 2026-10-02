//! Independent positional cache distance and bounded-work controls.
use super::*;

#[test]
fn reverse_position_cache_preserves_filtered_distances_in_every_visit_order() {
    let tree = Tree::new(vec![
        Record {
            name: "main",
            ..Record::default()
        },
        Record {
            name: "p",
            parent: Some(0),
            classes: &["x"],
            ..Record::default()
        },
        Record {
            name: "span",
            parent: Some(0),
            ..Record::default()
        },
        Record {
            name: "p",
            parent: Some(0),
            classes: &["x"],
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
            classes: &["x"],
            ..Record::default()
        },
    ]);
    for (css, expected) in [
        ("p:nth-last-child(3)", vec![3]),
        ("p:nth-last-of-type(3)", vec![3]),
        ("p:nth-last-child(2 of .x)", vec![3]),
    ] {
        let selectors = parse(css);
        for order in [
            vec![1, 2, 3, 4, 5],
            vec![5, 4, 3, 2, 1],
            vec![1, 5, 3, 2, 4],
        ] {
            let mut caches = SelectorCaches::default();
            let mut context = MatchingContext::new(
                MatchingMode::Normal,
                None,
                &mut caches,
                QuirksMode::NoQuirks,
                NeedsSelectorFlags::No,
                MatchingForInvalidation::No,
            );
            for index in order {
                assert_eq!(
                    matches_selector_list(&selectors, &tree.element(index), &mut context),
                    expected.contains(&index),
                    "{css} at {index}"
                );
            }
        }
    }
}

#[test]
fn reverse_cache_skips_nonmatching_siblings_and_uses_the_nearest_known_distance() {
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
            name: "span",
            parent: Some(0),
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
    context
        .nth_index_cache(true, true, &[])
        .insert(tree.element(1).opaque(), 3);
    context.set_work_budget(Some(&budget));
    assert_eq!(
        nth_child_index(
            &tree.element(3),
            &mut context,
            &[],
            true,
            true,
            SubjectOrPseudoElement::Yes
        ),
        2
    );
    assert_eq!(budget.remaining(), 0);
    assert!(!budget.exhausted());
}

#[test]
fn forward_cache_uses_a_known_predecessor_without_rewalking_its_ancestors() {
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
        Record {
            name: "p",
            parent: Some(0),
            ..Record::default()
        },
    ]);
    let budget = crate::work_budget::SelectorWorkBudget::new(1);
    let mut caches = SelectorCaches::default();
    let mut context = MatchingContext::new(
        MatchingMode::Normal,
        None,
        &mut caches,
        QuirksMode::NoQuirks,
        NeedsSelectorFlags::No,
        MatchingForInvalidation::No,
    );
    context
        .nth_index_cache(false, false, &[])
        .insert(tree.element(2).opaque(), 2);
    context.set_work_budget(Some(&budget));
    assert_eq!(
        nth_child_index(
            &tree.element(3),
            &mut context,
            &[],
            false,
            false,
            SubjectOrPseudoElement::Yes
        ),
        3
    );
    assert_eq!(budget.remaining(), 0);
    assert!(!budget.exhausted());
}

#[test]
fn edge_and_general_positions_have_exact_parent_invalidation_sets() {
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
        Record {
            name: "p",
            parent: Some(0),
            ..Record::default()
        },
    ]);
    for (css, index, expected) in [
        (
            "p:first-child",
            1,
            ElementSelectorFlags::HAS_EDGE_CHILD_SELECTOR,
        ),
        (
            "p:last-child",
            3,
            ElementSelectorFlags::HAS_EDGE_CHILD_SELECTOR,
        ),
        (
            "p:nth-child(3)",
            3,
            ElementSelectorFlags::HAS_SLOW_SELECTOR_LATER_SIBLINGS
                .union(ElementSelectorFlags::HAS_SLOW_SELECTOR_NTH),
        ),
        (
            "p:nth-last-child(3)",
            1,
            ElementSelectorFlags::HAS_SLOW_SELECTOR
                .union(ElementSelectorFlags::HAS_SLOW_SELECTOR_NTH),
        ),
    ] {
        tree.flags.borrow_mut().fill(0);
        let selectors = parse(css);
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
            &tree.element(index),
            &mut context
        ));
        assert_eq!(tree.flags.borrow()[0], expected.bits(), "{css}");
        assert_eq!(tree.flags.borrow()[index], 0, "flags belong to the parent");
    }
}
