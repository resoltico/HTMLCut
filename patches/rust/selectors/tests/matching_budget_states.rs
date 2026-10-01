//! Budget boundaries and nested mode contracts at real dispatch points.
use super::*;

#[test]
fn exhausted_sibling_and_ancestor_dispatch_stop_at_their_own_boundary() {
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
    let sibling = parse("p:has(~ p)");
    let relatives = sibling.slice()[0]
        .iter()
        .find_map(|component| {
            if let Component::Has(relatives) = component {
                Some(relatives)
            } else {
                None
            }
        })
        .expect("relative fixture");
    let ancestor = parse("main p");
    for siblings in [false, true] {
        let budget = crate::work_budget::SelectorWorkBudget::new(if siblings { 1 } else { 2 });
        if siblings {
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
        if siblings {
            assert!(!matches_relative_selector(
                &relatives[0],
                &tree.element(1),
                &mut context,
                SubjectOrPseudoElement::Yes
            ));
        } else {
            let result = matches_complex_selector_internal(
                ancestor.slice()[0].iter(),
                &tree.element(1),
                &mut context,
                SubjectOrPseudoElement::Yes,
                SubjectOrPseudoElement::Yes,
            );
            assert!(matches!(result, SelectorMatchingResult::NotMatchedGlobally));
        }
        assert!(budget.exhausted());
    }
}

#[test]
fn nested_and_non_subject_stateless_mode_preserve_real_pseudo_elements() {
    use crate::parser::tests::PseudoElement;
    let tree = Tree::new(vec![
        Record {
            name: "p",
            ..Record::default()
        },
        Record {
            name: "pseudo",
            parent: Some(0),
            pseudo: Some(PseudoElement::Before),
            ..Record::default()
        },
    ]);
    let selectors = parse("p::before");
    for nested in [false, true] {
        let mut caches = SelectorCaches::default();
        let mut context = MatchingContext::new(
            MatchingMode::ForStatelessPseudoElement,
            None,
            &mut caches,
            QuirksMode::NoQuirks,
            NeedsSelectorFlags::No,
            MatchingForInvalidation::No,
        );
        let result = if nested {
            context.nest(|context| {
                matches_complex_selector(
                    selectors.slice()[0].iter(),
                    &tree.element(1),
                    context,
                    SubjectOrPseudoElement::Yes,
                )
            })
        } else {
            matches_complex_selector(
                selectors.slice()[0].iter(),
                &tree.element(1),
                &mut context,
                SubjectOrPseudoElement::No,
            )
        };
        assert_eq!(result, KleeneValue::True);
    }
}

#[test]
fn quirk_checks_and_nth_comparison_do_not_override_filtered_false_results() {
    let tree = Tree::new(vec![Record {
        name: "p",
        hover: true,
        ..Record::default()
    }]);
    let hover = parse(":hover");
    let mut caches = SelectorCaches::default();
    let context = MatchingContext::new(
        MatchingMode::ForStatelessPseudoElement,
        None,
        &mut caches,
        QuirksMode::Quirks,
        NeedsSelectorFlags::No,
        MatchingForInvalidation::No,
    );
    assert!(hover_and_active_quirk_applies(
        &hover.slice()[0].iter(),
        &context,
        SubjectOrPseudoElement::No
    ));
    for (css, mode, expected) in [
        (
            "p:nth-child(2 of .absent)",
            MatchingForInvalidation::YesForComparison,
            KleeneValue::False,
        ),
        (
            "p:nth-child(2 of .absent)",
            MatchingForInvalidation::Yes,
            KleeneValue::False,
        ),
        (
            ":not(p:nth-child(200))",
            MatchingForInvalidation::Yes,
            KleeneValue::True,
        ),
    ] {
        let selectors = parse(css);
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
                &tree.element(0),
                &mut context
            ),
            expected,
            "{css}"
        );
    }
    let language = parse(":lang(en)");
    let mut caches = SelectorCaches::default();
    let mut context = MatchingContext::new(
        MatchingMode::Normal,
        None,
        &mut caches,
        QuirksMode::Quirks,
        NeedsSelectorFlags::No,
        MatchingForInvalidation::No,
    );
    assert!(!matches_selector_list(
        &language,
        &tree.element(0),
        &mut context
    ));
}

#[test]
fn featureless_pseudo_combinator_keeps_the_originating_host_identity() {
    use crate::parser::tests::PseudoElement;
    let tree = Tree::new(vec![
        Record {
            name: "host",
            ..Record::default()
        },
        Record {
            name: "pseudo",
            parent: Some(0),
            pseudo: Some(PseudoElement::Before),
            ..Record::default()
        },
    ]);
    let selectors = parse(":host::before");
    let mut caches = SelectorCaches::default();
    let mut context = MatchingContext::new(
        MatchingMode::Normal,
        None,
        &mut caches,
        QuirksMode::NoQuirks,
        NeedsSelectorFlags::No,
        MatchingForInvalidation::No,
    );
    context.current_host = Some(tree.element(0).opaque());
    context.with_featureless(true, |context| {
        assert!(matches_selector_list(&selectors, &tree.element(1), context));
    });
}
