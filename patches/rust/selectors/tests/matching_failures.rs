//! Direct refusal and invalid-context controls for generic matcher boundaries.
use super::*;

#[test]
fn invalid_selector_representation_is_an_ordinary_nonmatch() {
    let tree = Tree::new(vec![Record {
        name: "p",
        ..Record::default()
    }]);
    let selector = Selector::<DummySelectorImpl>::new_invalid("[");
    let mut caches = SelectorCaches::default();
    let mut context = MatchingContext::new(
        MatchingMode::Normal,
        None,
        &mut caches,
        QuirksMode::NoQuirks,
        NeedsSelectorFlags::No,
        MatchingForInvalidation::No,
    );
    assert!(!matches_selector(
        &selector,
        0,
        None,
        &tree.element(0),
        &mut context
    ));
}

#[test]
#[should_panic(expected = "Used MatchingMode::ForStatelessPseudoElement")]
fn stateless_pseudo_mode_requires_a_pseudo_selector() {
    let tree = Tree::new(vec![Record {
        name: "p",
        ..Record::default()
    }]);
    let selectors = parse("p");
    let mut caches = SelectorCaches::default();
    let mut context = MatchingContext::new(
        MatchingMode::ForStatelessPseudoElement,
        None,
        &mut caches,
        QuirksMode::NoQuirks,
        NeedsSelectorFlags::No,
        MatchingForInvalidation::No,
    );
    let _ = matches_selector_list(&selectors, &tree.element(0), &mut context);
}

#[test]
fn featureless_matching_refuses_ordinary_features_and_further_ancestors() {
    let tree = Tree::new(vec![Record {
        name: "host",
        classes: &["x"],
        ..Record::default()
    }]);
    for (css, expected) in [(".x", false), (":scope", true), ("main :scope", false)] {
        let selectors = parse(css);
        let mut caches = SelectorCaches::default();
        let mut context = MatchingContext::new(
            MatchingMode::Normal,
            None,
            &mut caches,
            QuirksMode::NoQuirks,
            NeedsSelectorFlags::No,
            MatchingForInvalidation::No,
        );
        context.scope_element = Some(tree.element(0).opaque());
        context.with_featureless(true, |context| {
            assert_eq!(
                matches_selector_list(&selectors, &tree.element(0), context),
                expected,
                "{css}"
            )
        });
    }
}

#[test]
fn exhausted_subtree_search_and_relative_list_stop_before_more_facts() {
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
    let selector = parse("p");
    let relative = parse(":has(.x)");
    for list in [false, true] {
        let budget = crate::work_budget::SelectorWorkBudget::new(1);
        assert!(budget.consume());
        let mut caches = SelectorCaches::default();
        let mut context = MatchingContext::new(
            MatchingMode::Normal,
            None,
            &mut caches,
            QuirksMode::NoQuirks,
            NeedsSelectorFlags::Yes,
            MatchingForInvalidation::No,
        );
        context.set_work_budget(Some(&budget));
        let matched = if list {
            let Component::Has(selectors) = relative.slice()[0].iter().next().unwrap() else {
                panic!("relative fixture")
            };
            do_match_relative_selectors(
                selectors,
                &tree.element(0),
                &mut context,
                SubjectOrPseudoElement::No,
            )
        } else {
            matches_relative_selector_subtree(
                &selector.slice()[0],
                &tree.element(0),
                &mut context,
                SubjectOrPseudoElement::Yes,
            )
        };
        assert!(!matched);
        assert!(budget.exhausted());
        assert_eq!(tree.hash_visits.get(), 0);
    }
}

#[test]
fn sibling_relative_matching_sets_direction_flags_and_preserves_values() {
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
            name: "div",
            parent: Some(0),
            ..Record::default()
        },
        Record {
            name: "span",
            parent: Some(2),
            classes: &["x"],
            ..Record::default()
        },
    ]);
    for (css, expected) in [("p:has(+ div span)", true), ("p:has(~ .missing)", false)] {
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
        assert_eq!(
            matches_selector_list(&selectors, &tree.element(1), &mut context),
            expected
        );
    }
    assert_ne!(
        tree.flags.borrow()[1]
            & ElementSelectorFlags::RELATIVE_SELECTOR_SEARCH_DIRECTION_SIBLING.bits(),
        0
    );
}

#[test]
fn nested_part_and_slot_walks_stop_on_the_operation_budget() {
    let tree = Tree::new(vec![
        Record {
            name: "main",
            ..Record::default()
        },
        Record {
            name: "host",
            parent: Some(0),
            ..Record::default()
        },
        Record {
            name: "#shadow",
            parent: Some(1),
            ..Record::default()
        },
        Record {
            name: "inner",
            parent: Some(2),
            ..Record::default()
        },
        Record {
            name: "#shadow",
            parent: Some(3),
            ..Record::default()
        },
        Record {
            name: "p",
            parent: Some(4),
            ..Record::default()
        },
        Record {
            name: "slot",
            parent: Some(2),
            ..Record::default()
        },
        Record {
            name: "slot",
            parent: Some(4),
            slot: Some(6),
            ..Record::default()
        },
        Record {
            name: "a",
            parent: Some(3),
            slot: Some(7),
            ..Record::default()
        },
    ]);
    for slot in [false, true] {
        let budget = crate::work_budget::SelectorWorkBudget::new(1);
        if slot {
            assert!(budget.consume());
        }
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
        if slot {
            context.current_host = Some(tree.element(1).opaque());
            assert!(assigned_slot(&tree.element(8), &context).is_none());
        } else {
            assert!(host_for_part(&tree.element(5), &context).is_none());
        }
        assert!(budget.exhausted());
    }
}

#[test]
fn nth_from_end_cache_search_cannot_walk_without_work() {
    let mut records = vec![Record {
        name: "main",
        ..Record::default()
    }];
    for _ in 0..30 {
        records.push(Record {
            name: "p",
            parent: Some(0),
            ..Record::default()
        });
    }
    let tree = Tree::new(records);
    let selectors = parse("p:nth-last-child(30)");
    let broad = crate::work_budget::SelectorWorkBudget::new(1000);
    let small = crate::work_budget::SelectorWorkBudget::new(1);
    let mut caches = SelectorCaches::default();
    let mut context = MatchingContext::new(
        MatchingMode::Normal,
        None,
        &mut caches,
        QuirksMode::NoQuirks,
        NeedsSelectorFlags::No,
        MatchingForInvalidation::No,
    );
    context.set_work_budget(Some(&broad));
    assert!(matches_selector_list(
        &selectors,
        &tree.element(1),
        &mut context
    ));
    assert!(small.consume());
    context.set_work_budget(Some(&small));
    assert_eq!(
        nth_child_index(
            &tree.element(30),
            &mut context,
            &[],
            false,
            true,
            SubjectOrPseudoElement::Yes
        ),
        0
    );
    assert!(small.exhausted());
}

#[test]
#[should_panic(expected = "Combinators must separate selector compounds")]
fn a_combinator_cannot_be_submitted_as_a_simple_component() {
    let tree = Tree::new(vec![Record {
        name: "p",
        ..Record::default()
    }]);
    let mut caches = SelectorCaches::default();
    let mut context = MatchingContext::new(
        MatchingMode::Normal,
        None,
        &mut caches,
        QuirksMode::NoQuirks,
        NeedsSelectorFlags::No,
        MatchingForInvalidation::No,
    );
    let mut local = LocalMatchingContext {
        shared: &mut context,
        rightmost: SubjectOrPseudoElement::Yes,
        quirks_data: None,
    };
    let _ = matches_simple_selector(
        &Component::Combinator(Combinator::Child),
        &tree.element(0),
        &mut local,
    );
}

#[test]
fn stateless_pseudo_mode_refuses_stateful_pseudo_requirements() {
    let tree = Tree::new(vec![Record {
        name: "p",
        hover: true,
        ..Record::default()
    }]);
    let selectors = parse("p::before:hover");
    let mut caches = SelectorCaches::default();
    let mut context = MatchingContext::new(
        MatchingMode::ForStatelessPseudoElement,
        None,
        &mut caches,
        QuirksMode::NoQuirks,
        NeedsSelectorFlags::No,
        MatchingForInvalidation::No,
    );
    assert!(!matches_selector_list(
        &selectors,
        &tree.element(0),
        &mut context
    ));
}

#[test]
fn combinator_backtracking_does_not_promote_a_partial_match_to_success() {
    let tree = Tree::new(vec![
        Record {
            name: "main",
            ..Record::default()
        },
        Record {
            name: "a",
            parent: Some(0),
            ..Record::default()
        },
        Record {
            name: "div",
            parent: Some(0),
            ..Record::default()
        },
        Record {
            name: "p",
            parent: Some(0),
            ..Record::default()
        },
    ]);
    for (css, expected) in [
        ("main > a ~ p", true),
        ("div > a ~ p", false),
        ("a + p", false),
        ("main > div + p", true),
        ("missing p", false),
        ("main .missing", false),
    ] {
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
        assert_eq!(
            matches_selector_list(&selectors, &tree.element(3), &mut context),
            expected,
            "{css}"
        );
    }
}

#[test]
fn comparison_unknown_is_preserved_after_a_matching_ancestor() {
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
    for css in ["main p:has(.missing)", "main:has(.missing) p"] {
        let selectors = parse(css);
        let mut caches = SelectorCaches::default();
        let mut context = MatchingContext::new(
            MatchingMode::Normal,
            None,
            &mut caches,
            QuirksMode::NoQuirks,
            NeedsSelectorFlags::No,
            MatchingForInvalidation::YesForComparison,
        );
        assert_eq!(
            matches_selector_kleene(
                &selectors.slice()[0],
                0,
                None,
                &tree.element(1),
                &mut context
            ),
            KleeneValue::Unknown,
            "{css}"
        );
    }
}

#[test]
fn part_translation_refuses_unknown_origin_and_exhausted_export_work() {
    let tree = Tree::new(vec![
        Record {
            name: "main",
            ..Record::default()
        },
        Record {
            name: "host",
            parent: Some(0),
            ..Record::default()
        },
        Record {
            name: "#shadow",
            parent: Some(1),
            ..Record::default()
        },
        Record {
            name: "inner",
            parent: Some(2),
            exports: &[("theme", "button")],
            ..Record::default()
        },
        Record {
            name: "#shadow",
            parent: Some(3),
            ..Record::default()
        },
        Record {
            name: "p",
            parent: Some(4),
            parts: &["button"],
            ..Record::default()
        },
    ]);
    for (units, origin) in [(1, None), (2, None), (100, Some(0))] {
        let budget = crate::work_budget::SelectorWorkBudget::new(units);
        let mut caches = SelectorCaches::default();
        let mut context = MatchingContext::new(
            MatchingMode::Normal,
            None,
            &mut caches,
            QuirksMode::NoQuirks,
            NeedsSelectorFlags::No,
            MatchingForInvalidation::No,
        );
        context.current_host = origin.map(|index| tree.element(index).opaque());
        context.set_work_budget(Some(&budget));
        assert!(!matches_part(
            &tree.element(5),
            &["theme".into()],
            &mut context
        ));
        assert_eq!(budget.exhausted(), units < 100);
    }
}
