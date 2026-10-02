//! Generic state/pseudo-element contracts, not application browser capabilities.
use super::*;
#[test]
fn generic_host_slot_part_and_pseudo_contracts_have_complete_id_oracles() {
    use crate::parser::tests::PseudoElement;
    let tree = Tree::new(vec![
        Record {
            name: "main",
            id: "root",
            ..Record::default()
        },
        Record {
            name: "host",
            id: "host",
            parent: Some(0),
            classes: &["x"],
            ..Record::default()
        },
        Record {
            name: "#shadow",
            parent: Some(1),
            ..Record::default()
        },
        Record {
            name: "p",
            id: "part",
            parent: Some(2),
            parts: &["button"],
            ..Record::default()
        },
        Record {
            name: "slot",
            id: "slot",
            parent: Some(2),
            ..Record::default()
        },
        Record {
            name: "a",
            id: "light",
            parent: Some(1),
            slot: Some(4),
            classes: &["slotted"],
            ..Record::default()
        },
        Record {
            name: "pseudo",
            id: "before",
            parent: Some(3),
            pseudo: Some(PseudoElement::Before),
            ..Record::default()
        },
    ]);
    for (css, host, mode, candidates, expected) in [
        (
            ":host",
            Some(1),
            MatchingMode::Normal,
            vec![0, 1, 3, 4, 5],
            vec!["host"],
        ),
        (
            ":host(.x)",
            Some(1),
            MatchingMode::Normal,
            vec![0, 1, 3],
            vec!["host"],
        ),
        (
            ":host(.missing)",
            Some(1),
            MatchingMode::Normal,
            vec![1],
            vec![],
        ),
        (":host", None, MatchingMode::Normal, vec![1], vec![]),
        (
            "slot::slotted(.slotted)",
            Some(1),
            MatchingMode::Normal,
            vec![3, 4, 5],
            vec!["light"],
        ),
        (
            "::part(button)",
            Some(1),
            MatchingMode::Normal,
            vec![1, 3, 4, 5],
            vec!["part"],
        ),
        (
            "::part(missing)",
            Some(1),
            MatchingMode::Normal,
            vec![3],
            vec![],
        ),
        (
            "p::before",
            None,
            MatchingMode::Normal,
            vec![3, 6],
            vec!["before"],
        ),
        ("p::after", None, MatchingMode::Normal, vec![6], vec![]),
        (
            "p::before",
            None,
            MatchingMode::ForStatelessPseudoElement,
            vec![3],
            vec!["part"],
        ),
    ] {
        let selectors = parse(css);
        let mut caches = SelectorCaches::default();
        let mut context = MatchingContext::new(
            mode,
            None,
            &mut caches,
            QuirksMode::NoQuirks,
            NeedsSelectorFlags::Yes,
            MatchingForInvalidation::No,
        );
        context.current_host = host.map(|index| tree.element(index).opaque());
        let actual: Vec<_> = candidates
            .into_iter()
            .filter(|index| matches_selector_list(&selectors, &tree.element(*index), &mut context))
            .map(|index| tree.records[index].id)
            .collect();
        assert_eq!(actual, expected, "{css}");
    }
}

#[test]
fn local_name_rejection_and_compound_boundaries_do_not_conflate_match_states() {
    let tree = Tree::new(vec![Record {
        name: "p",
        id: "target",
        classes: &["x"],
        ..Record::default()
    }]);
    for (css, rejected) in [
        ("p.x", false),
        ("div.x", true),
        (":is(div, p).x", false),
        (":where(div, aside)", true),
        (".missing", false),
    ] {
        let selectors = parse(css);
        assert_eq!(
            early_reject_by_local_name(&selectors.slice()[0], 0, &tree.element(0)),
            rejected,
            "{css}"
        );
    }
    for (css, expected) in [
        ("p.x", CompoundSelectorMatchingResult::FullyMatched),
        ("p.missing", CompoundSelectorMatchingResult::NotMatched),
    ] {
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
        assert!(matches!(
            (
                matches_compound_selector_from(
                    &selectors.slice()[0],
                    0,
                    &mut context,
                    &tree.element(0)
                ),
                expected
            ),
            (
                CompoundSelectorMatchingResult::FullyMatched,
                CompoundSelectorMatchingResult::FullyMatched
            ) | (
                CompoundSelectorMatchingResult::NotMatched,
                CompoundSelectorMatchingResult::NotMatched
            )
        ));
    }
}

#[test]
fn invalidation_preserves_unknown_and_conservative_relative_results() {
    let tree = Tree::new(vec![Record {
        name: "main",
        ..Record::default()
    }]);
    let selectors = parse("main:has(.missing)");
    for (invalidation, expected) in [
        (MatchingForInvalidation::No, KleeneValue::False),
        (MatchingForInvalidation::Yes, KleeneValue::True),
        (
            MatchingForInvalidation::YesForComparison,
            KleeneValue::Unknown,
        ),
    ] {
        let mut caches = SelectorCaches::default();
        let mut context = MatchingContext::new(
            MatchingMode::Normal,
            None,
            &mut caches,
            QuirksMode::NoQuirks,
            NeedsSelectorFlags::No,
            invalidation,
        );
        assert_eq!(
            matches_selector_kleene(
                &selectors.slice()[0],
                0,
                None,
                &tree.element(0),
                &mut context
            ),
            expected
        );
        assert_eq!(
            matches_selector(
                &selectors.slice()[0],
                0,
                None,
                &tree.element(0),
                &mut context
            ),
            expected.to_bool(true)
        );
    }
}

#[test]
fn ancestor_bloom_and_local_name_checks_refuse_before_matching_facts() {
    let tree = Tree::new(vec![Record {
        name: "p",
        ..Record::default()
    }]);
    let selectors = parse("p");
    let hashes = AncestorHashes {
        packed_hashes: [1, 0, 0],
    };
    let mut filter = BloomFilter::new();
    let mut caches = SelectorCaches::default();
    let mut context = MatchingContext::new(
        MatchingMode::Normal,
        Some(&filter),
        &mut caches,
        QuirksMode::NoQuirks,
        NeedsSelectorFlags::No,
        MatchingForInvalidation::No,
    );
    assert!(!matches_selector(
        &selectors.slice()[0],
        0,
        Some(&hashes),
        &tree.element(0),
        &mut context
    ));
    filter.insert_hash(1);
    let mut context = MatchingContext::new(
        MatchingMode::Normal,
        Some(&filter),
        &mut caches,
        QuirksMode::NoQuirks,
        NeedsSelectorFlags::No,
        MatchingForInvalidation::No,
    );
    assert!(matches_selector(
        &selectors.slice()[0],
        0,
        Some(&hashes),
        &tree.element(0),
        &mut context
    ));
    assert!(matches_selector(
        &selectors.slice()[0],
        0,
        None,
        &tree.element(0),
        &mut context
    ));
}

#[test]
fn stateless_pseudo_policy_can_accept_or_refuse_the_requested_pseudo() {
    use crate::parser::tests::PseudoElement;
    let tree = Tree::new(vec![Record {
        name: "p",
        ..Record::default()
    }]);
    let selectors = parse("p::before");
    let accept = |pseudo: &PseudoElement| *pseudo == PseudoElement::Before;
    let reject = |_: &PseudoElement| false;
    for policy in [&accept as &dyn Fn(&PseudoElement) -> bool, &reject] {
        let mut caches = SelectorCaches::default();
        let mut context = MatchingContext::new(
            MatchingMode::ForStatelessPseudoElement,
            None,
            &mut caches,
            QuirksMode::NoQuirks,
            NeedsSelectorFlags::No,
            MatchingForInvalidation::No,
        );
        context.pseudo_element_matching_fn = Some(policy);
        assert_eq!(
            matches_selector(
                &selectors.slice()[0],
                0,
                None,
                &tree.element(0),
                &mut context
            ),
            policy(&PseudoElement::Before)
        );
    }
}

#[test]
fn hover_active_quirks_distinguish_links_compounds_and_nested_matching() {
    let tree = Tree::new(vec![
        Record {
            name: "p",
            id: "plain",
            hover: true,
            active: true,
            ..Record::default()
        },
        Record {
            name: "a",
            id: "link",
            hover: true,
            active: true,
            link: true,
            ..Record::default()
        },
    ]);
    for (css, expected) in [
        (":hover", vec!["link"]),
        (":active", vec!["link"]),
        (":hover:active", vec!["link"]),
        ("p:hover", vec!["plain"]),
        (":is(:hover)", vec!["plain", "link"]),
    ] {
        let selectors = parse(css);
        let mut caches = SelectorCaches::default();
        let mut context = MatchingContext::new(
            MatchingMode::Normal,
            None,
            &mut caches,
            QuirksMode::Quirks,
            NeedsSelectorFlags::No,
            MatchingForInvalidation::No,
        );
        let actual: Vec<_> = (0..2)
            .filter(|index| matches_selector_list(&selectors, &tree.element(*index), &mut context))
            .map(|index| tree.records[index].id)
            .collect();
        assert_eq!(actual, expected, "{css}");
    }
    let selectors = parse("p:hover::before");
    let mut caches = SelectorCaches::default();
    let mut context = MatchingContext::new(
        MatchingMode::ForStatelessPseudoElement,
        None,
        &mut caches,
        QuirksMode::Quirks,
        NeedsSelectorFlags::No,
        MatchingForInvalidation::No,
    );
    assert!(matches_selector_list(
        &selectors,
        &tree.element(0),
        &mut context
    ));
}

#[test]
fn nested_parts_and_flattened_slots_follow_the_requested_origin_scope() {
    let tree = Tree::new(vec![
        Record {
            name: "main",
            id: "root",
            ..Record::default()
        },
        Record {
            name: "host",
            id: "outer",
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
            id: "inner",
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
            id: "part",
            parent: Some(4),
            parts: &["button"],
            ..Record::default()
        },
        Record {
            name: "slot",
            id: "outer-slot",
            parent: Some(2),
            ..Record::default()
        },
        Record {
            name: "slot",
            id: "inner-slot",
            parent: Some(4),
            slot: Some(6),
            ..Record::default()
        },
        Record {
            name: "a",
            id: "light",
            parent: Some(3),
            slot: Some(7),
            classes: &["light"],
            ..Record::default()
        },
    ]);
    for (css, host, candidates, expected) in [
        ("host::part(theme)", None, vec![5], vec!["part"]),
        ("host::part(button)", None, vec![5], vec![]),
        ("inner::part(button)", Some(1), vec![5], vec!["part"]),
        ("inner::part(theme)", Some(1), vec![5], vec![]),
        ("slot::slotted(.light)", Some(1), vec![7, 8], vec!["light"]),
        ("slot::slotted(.missing)", Some(1), vec![8], vec![]),
    ] {
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
        context.current_host = host.map(|index| tree.element(index).opaque());
        let actual: Vec<_> = candidates
            .into_iter()
            .filter(|index| matches_selector_list(&selectors, &tree.element(*index), &mut context))
            .map(|index| tree.records[index].id)
            .collect();
        assert_eq!(actual, expected, "{css}");
    }
}
