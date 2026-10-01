//! Namespace and explicit scope facts at the generic matching boundary.
use super::*;

#[test]
fn namespace_and_attribute_namespace_constraints_use_declared_element_facts() {
    let tree = Tree::new(vec![
        Record {
            name: "p",
            id: "plain",
            attrs: &[("", "code", "AB")],
            ..Record::default()
        },
        Record {
            name: "p",
            id: "foreign",
            namespace: "urn:fixture",
            attrs: &[("urn:fixture", "code", "AB")],
            ..Record::default()
        },
    ]);
    let parser = DummyParser::default().with_namespace("f", "urn:fixture");
    for (css, expected) in [
        ("*|p", vec!["plain", "foreign"]),
        ("|p", vec!["plain"]),
        ("f|p", vec!["foreign"]),
        ("[code]", vec!["plain"]),
        ("[*|code]", vec!["plain", "foreign"]),
        ("[f|code]", vec!["foreign"]),
        ("[f|code=AB]", vec!["foreign"]),
        ("[f|code=ab s]", vec![]),
    ] {
        let selectors = SelectorList::parse(
            &parser,
            &mut cssparser::Parser::new(css),
            crate::parser::ParseRelative::No,
        )
        .unwrap();
        let mut caches = SelectorCaches::default();
        let mut context = MatchingContext::new(
            MatchingMode::Normal,
            None,
            &mut caches,
            QuirksMode::NoQuirks,
            NeedsSelectorFlags::No,
            MatchingForInvalidation::No,
        );
        let actual: Vec<_> = (0..2)
            .filter(|index| matches_selector_list(&selectors, &tree.element(*index), &mut context))
            .map(|index| tree.records[index].id)
            .collect();
        assert_eq!(actual, expected, "{css}");
    }
    let parser = DummyParser::default_with_namespace("urn:fixture".into());
    let selectors = SelectorList::parse(
        &parser,
        &mut cssparser::Parser::new("p"),
        crate::parser::ParseRelative::No,
    )
    .unwrap();
    let mut caches = SelectorCaches::default();
    let mut context = MatchingContext::new(
        MatchingMode::Normal,
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
    assert!(matches_selector_list(
        &selectors,
        &tree.element(1),
        &mut context
    ));
}

#[test]
fn name_and_automatic_case_policies_keep_foreign_case_significant() {
    let tree = Tree::new(vec![
        Record {
            name: "p",
            ..Record::default()
        },
        Record {
            name: "P",
            namespace: "urn:fixture",
            ..Record::default()
        },
    ]);
    assert_eq!(*select_name(&tree.element(0), &"P", &"p"), "p");
    assert_eq!(*select_name(&tree.element(1), &"P", &"p"), "P");
    assert_eq!(*select_name(&tree.element(1), &"p", &"p"), "p");
    let policy = ParsedCaseSensitivity::AsciiCaseInsensitiveIfInHtmlElementInHtmlDocument;
    assert!(matches!(
        to_unconditional_case_sensitivity(policy, &tree.element(0)),
        CaseSensitivity::AsciiCaseInsensitive
    ));
    assert!(matches!(
        to_unconditional_case_sensitivity(policy, &tree.element(1)),
        CaseSensitivity::CaseSensitive
    ));
}

#[test]
fn parent_and_scope_selectors_respect_root_explicit_scope_and_comparison_modes() {
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
    for css in ["&", ":scope", ":root"] {
        let selectors = parse(css);
        for scope in [None, Some(1)] {
            let mut caches = SelectorCaches::default();
            let mut context = MatchingContext::new(
                MatchingMode::Normal,
                None,
                &mut caches,
                QuirksMode::NoQuirks,
                NeedsSelectorFlags::No,
                MatchingForInvalidation::No,
            );
            context.scope_element = scope.map(|index| tree.element(index).opaque());
            for index in 0..2 {
                assert_eq!(
                    matches_selector_list(&selectors, &tree.element(index), &mut context),
                    index
                        == if css == ":root" {
                            0
                        } else {
                            scope.unwrap_or(0)
                        },
                    "{css}"
                );
            }
        }
    }
    let selectors = parse(":scope");
    for (mode, expected) in [
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
            mode,
        );
        assert_eq!(
            matches_selector_kleene(
                &selectors.slice()[0],
                0,
                None,
                &tree.element(1),
                &mut context
            ),
            expected
        );
    }
    let mut caches = SelectorCaches::default();
    let mut context = MatchingContext::new_for_revalidation(
        None,
        &mut caches,
        QuirksMode::NoQuirks,
        NeedsSelectorFlags::No,
    );
    assert!(matches_selector_list(
        &selectors,
        &tree.element(1),
        &mut context
    ));
}

#[test]
fn partial_compounds_report_the_next_boundary_and_allow_non_subject_matching() {
    let tree = Tree::new(vec![
        Record {
            name: "main",
            classes: &["x"],
            ..Record::default()
        },
        Record {
            name: "p",
            parent: Some(0),
            classes: &["y"],
            ..Record::default()
        },
    ]);
    let selectors = parse("main.x > p.y");
    let selector = &selectors.slice()[0];
    let mut caches = SelectorCaches::default();
    let mut context = MatchingContext::new(
        MatchingMode::Normal,
        None,
        &mut caches,
        QuirksMode::NoQuirks,
        NeedsSelectorFlags::Yes,
        MatchingForInvalidation::No,
    );
    assert!(matches!(
        matches_compound_selector_from(selector, 0, &mut context, &tree.element(0)),
        CompoundSelectorMatchingResult::Matched {
            next_combinator_offset: 2
        }
    ));
    assert!(matches!(
        matches_compound_selector_from(selector, 3, &mut context, &tree.element(1)),
        CompoundSelectorMatchingResult::FullyMatched
    ));
    let hashes = AncestorHashes {
        packed_hashes: [1, 0, 0],
    };
    assert!(matches_selector(
        selector,
        3,
        Some(&hashes),
        &tree.element(0),
        &mut context
    ));
}

#[test]
fn nth_invalidation_and_ignored_positions_do_not_invent_indices() {
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
            ignore_nth: true,
            ..Record::default()
        },
    ]);
    for css in [
        "p:nth-child(2)",
        "p:nth-last-child(2)",
        "p:first-child",
        "p:last-child",
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
        assert!(!matches_selector_list(
            &selectors,
            &tree.element(2),
            &mut context
        ));
    }
    let selectors = parse("p:nth-child(200)");
    for (mode, expected) in [
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
            NeedsSelectorFlags::Yes,
            mode,
        );
        assert_eq!(
            matches_selector_kleene(
                &selectors.slice()[0],
                0,
                None,
                &tree.element(1),
                &mut context
            ),
            expected
        );
    }
    let selectors = parse("p:nth-child(1 of .missing)");
    let mut caches = SelectorCaches::default();
    let mut context = MatchingContext::new(
        MatchingMode::Normal,
        None,
        &mut caches,
        QuirksMode::NoQuirks,
        NeedsSelectorFlags::Yes,
        MatchingForInvalidation::No,
    );
    assert!(!matches_selector_list(
        &selectors,
        &tree.element(1),
        &mut context
    ));
}

#[test]
fn nth_and_relative_caches_reuse_complete_results_and_reject_nested_relative_context() {
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
            name: "p",
            parent: Some(0),
            ..Record::default()
        },
    ]);
    let selectors = parse("p:nth-child(2)");
    let mut caches = SelectorCaches::default();
    let mut context = MatchingContext::new(
        MatchingMode::Normal,
        None,
        &mut caches,
        QuirksMode::NoQuirks,
        NeedsSelectorFlags::Yes,
        MatchingForInvalidation::No,
    );
    for _ in 0..3 {
        assert!(matches_selector_list(
            &selectors,
            &tree.element(2),
            &mut context
        ));
    }
    let relative = parse("main:has(.x)");
    for _ in 0..3 {
        assert!(matches_selector_list(
            &relative,
            &tree.element(0),
            &mut context
        ));
    }
    context.nest_for_relative_selector(tree.element(0).opaque(), |context| {
        assert!(!matches_selector_list(&relative, &tree.element(0), context));
    });
    assert!(matches_selector_list(
        &relative,
        &tree.element(0),
        &mut context
    ));
}

#[test]
fn edge_nth_and_filtered_nth_route_invalidation_to_the_parent() {
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
            name: "p",
            parent: Some(0),
            classes: &["x"],
            ..Record::default()
        },
    ]);
    for (css, index, flag) in [
        (
            "p:first-child",
            1,
            ElementSelectorFlags::HAS_EDGE_CHILD_SELECTOR,
        ),
        (
            "p:last-child",
            2,
            ElementSelectorFlags::HAS_EDGE_CHILD_SELECTOR,
        ),
        (
            "p:nth-child(2)",
            2,
            ElementSelectorFlags::HAS_SLOW_SELECTOR_NTH,
        ),
        (
            "p:nth-child(2 of .x)",
            2,
            ElementSelectorFlags::HAS_SLOW_SELECTOR_NTH_OF,
        ),
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
        assert!(matches_selector_list(
            &selectors,
            &tree.element(index),
            &mut context
        ));
        assert_ne!(tree.flags.borrow()[0] & flag.bits(), 0, "{css}");
    }
}

#[test]
fn universal_and_shadow_boundary_matching_keep_featureless_hosts_scoped() {
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
            name: "p",
            parent: Some(2),
            ..Record::default()
        },
    ]);
    for (css, expected) in [
        ("*", true),
        ("*|*", true),
        (":host > p", true),
        ("main p", false),
        (".missing > p", false),
        ("main :host > p", false),
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
        context.current_host = Some(tree.element(1).opaque());
        assert_eq!(
            matches_selector_list(&selectors, &tree.element(3), &mut context),
            expected,
            "{css}"
        );
    }
}
