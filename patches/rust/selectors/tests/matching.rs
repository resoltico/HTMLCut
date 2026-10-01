//! Independent generic matcher boundary controls, separate from application conformance.
use super::*;
use crate::parser::tests::{DummyParser, DummySelectorImpl};

fn parse(css: &str) -> SelectorList<DummySelectorImpl> {
    SelectorList::parse(
        &DummyParser::default(),
        &mut cssparser::Parser::new(css),
        crate::parser::ParseRelative::No,
    )
    .unwrap()
}

#[test]
fn invalidation_flags_are_routed_to_the_element_or_parent() {
    let local = ElementSelectorFlags::HAS_EMPTY_SELECTOR
        | ElementSelectorFlags::ANCHORS_RELATIVE_SELECTOR
        | ElementSelectorFlags::ANCHORS_RELATIVE_SELECTOR_NON_SUBJECT
        | ElementSelectorFlags::RELATIVE_SELECTOR_SEARCH_DIRECTION_SIBLING
        | ElementSelectorFlags::RELATIVE_SELECTOR_SEARCH_DIRECTION_ANCESTOR;
    let parent = ElementSelectorFlags::HAS_SLOW_SELECTOR
        | ElementSelectorFlags::HAS_SLOW_SELECTOR_LATER_SIBLINGS
        | ElementSelectorFlags::HAS_SLOW_SELECTOR_NTH
        | ElementSelectorFlags::HAS_SLOW_SELECTOR_NTH_OF
        | ElementSelectorFlags::HAS_EDGE_CHILD_SELECTOR
        | ElementSelectorFlags::MAY_HAVE_TREE_COUNTING_FUNCTION;
    assert_eq!((local | parent).for_self().bits(), local.bits());
    assert_eq!((local | parent).for_parent().bits(), parent.bits());
    assert!(local.for_parent().is_empty());
    assert!(parent.for_self().is_empty());
}

#[test]
fn ancestor_filter_rejects_missing_hashes_including_the_packed_fourth() {
    let mut filter = BloomFilter::new();
    assert!(selector_may_match(
        &AncestorHashes {
            packed_hashes: [0, 0, 0]
        },
        &filter
    ));
    let three = AncestorHashes {
        packed_hashes: [1, 2, 3],
    };
    assert!(!selector_may_match(&three, &filter));
    for hash in [1, 2, 3] {
        filter.insert_hash(hash);
    }
    assert!(selector_may_match(&three, &filter));
    let four = AncestorHashes {
        packed_hashes: [0x04000001, 2, 3],
    };
    assert!(!selector_may_match(&four, &filter));
    filter.insert_hash(4);
    assert!(selector_may_match(&four, &filter));
    assert!(selector_may_match(
        &AncestorHashes {
            packed_hashes: [1, 0, 0]
        },
        &filter
    ));
    assert!(selector_may_match(
        &AncestorHashes {
            packed_hashes: [1, 2, 0]
        },
        &filter
    ));
}

#[test]
fn backtracking_refusals_are_false_and_unknown_remains_three_valued() {
    assert_eq!(
        KleeneValue::from(SelectorMatchingResult::Matched),
        KleeneValue::True
    );
    assert_eq!(
        KleeneValue::from(SelectorMatchingResult::Unknown),
        KleeneValue::Unknown
    );
    for result in [
        SelectorMatchingResult::NotMatchedAndRestartFromClosestLaterSibling,
        SelectorMatchingResult::NotMatchedAndRestartFromClosestDescendant,
        SelectorMatchingResult::NotMatchedGlobally,
    ] {
        assert_eq!(KleeneValue::from(result), KleeneValue::False);
    }
}

#[test]
fn featureless_host_classification_keeps_logical_alternatives_distinct() {
    for (css, scope, expected) in [
        (":host", false, MatchesFeaturelessHost::Only),
        ("::before", false, MatchesFeaturelessHost::Only),
        (":scope", true, MatchesFeaturelessHost::Only),
        (":scope", false, MatchesFeaturelessHost::Never),
        (".x", false, MatchesFeaturelessHost::Never),
        (":is(:host)", false, MatchesFeaturelessHost::Only),
        (":is(:host, .x)", false, MatchesFeaturelessHost::Yes),
        (":where(.x, .y)", false, MatchesFeaturelessHost::Never),
        (":not(:host)", false, MatchesFeaturelessHost::Only),
        (":not(.x)", false, MatchesFeaturelessHost::Never),
        (
            ":is(:is(:host, .x), .y)",
            false,
            MatchesFeaturelessHost::Yes,
        ),
    ] {
        let selectors = parse(css);
        assert_eq!(
            compound_matches_featureless_host(&mut selectors.slice()[0].iter(), scope),
            expected,
            "{css}"
        );
    }
}

#[path = "element.rs"]
mod element;
use element::{Record, Tree};

#[test]
fn generic_state_attribute_and_structural_matching_has_complete_id_oracles() {
    let tree = Tree::new(vec![
        Record {
            name: "main",
            id: "root",
            ..Record::default()
        },
        Record {
            name: "p",
            id: "first",
            parent: Some(0),
            classes: &["Alpha"],
            attrs: &[("", "code", "AB-cd"), ("", "lang", "en-US")],
            hover: true,
            text: true,
            ..Record::default()
        },
        Record {
            name: "a",
            id: "link",
            parent: Some(0),
            active: true,
            link: true,
            ..Record::default()
        },
        Record {
            name: "p",
            id: "last",
            parent: Some(0),
            classes: &["x"],
            ..Record::default()
        },
    ]);
    for (css, expected) in [
        ("p", vec!["first", "last"]),
        (":hover", vec!["first"]),
        (":active", vec!["link"]),
        (":lang(en)", vec!["first"]),
        ("[code^=AB]", vec!["first"]),
        ("[code$=cd]", vec!["first"]),
        ("[code*=B-c]", vec!["first"]),
        ("[code|=AB]", vec!["first"]),
        ("[code=ab-CD i]", vec!["first"]),
        ("[code=ab-CD s]", vec![]),
        ("p:nth-child(2 of p)", vec!["last"]),
        ("p:nth-last-child(2 of p)", vec!["first"]),
        ("main:has(> p.x)", vec!["root"]),
        ("p:empty", vec!["last"]),
        ("p + a", vec!["link"]),
        ("a ~ p", vec!["last"]),
        (":is(.Alpha, a)", vec!["first", "link"]),
        (":not(p)", vec!["root", "link"]),
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
        let actual: Vec<_> = (0..tree.records.len())
            .filter(|index| matches_selector_list(&selectors, &tree.element(*index), &mut context))
            .map(|index| tree.records[index].id)
            .collect();
        assert_eq!(actual, expected, "{css}");
    }
}

#[test]
fn a_warmed_relative_filter_does_not_walk_past_a_fresh_work_budget() {
    let mut records = vec![Record {
        name: "main",
        id: "root",
        ..Record::default()
    }];
    records.push(Record {
        name: "p",
        parent: Some(0),
        classes: &["present"],
        ..Record::default()
    });
    for _ in 0..100 {
        records.push(Record {
            name: "p",
            parent: Some(0),
            ..Record::default()
        });
    }
    let tree = Tree::new(records);
    let first = parse("main:has(.present)");
    let second = parse("main:has(.absent)");
    let broad = crate::work_budget::SelectorWorkBudget::new(10000);
    let small = crate::work_budget::SelectorWorkBudget::new(16);
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
        &first,
        &tree.element(0),
        &mut context
    ));
    let before = tree.hash_visits.get();
    context.set_work_budget(Some(&small));
    assert!(!matches_selector_list(
        &second,
        &tree.element(0),
        &mut context
    ));
    assert!(
        small.exhausted(),
        "filter construction must debit its real traversal"
    );
    assert!(tree.hash_visits.get() - before <= 16);
}

#[test]
fn exhausted_relative_matching_does_not_cache_a_false_semantic_result() {
    let mut records = vec![Record {
        name: "main",
        ..Record::default()
    }];
    records.push(Record {
        name: "p",
        parent: Some(0),
        classes: &["present"],
        ..Record::default()
    });
    for _ in 0..100 {
        records.push(Record {
            name: "p",
            parent: Some(0),
            ..Record::default()
        });
    }
    records.push(Record {
        name: "p",
        parent: Some(0),
        classes: &["late"],
        ..Record::default()
    });
    let tree = Tree::new(records);
    let warm = parse("main:has(.present)");
    let target = parse("main:has(.late)");
    let broad = crate::work_budget::SelectorWorkBudget::new(10000);
    let small = crate::work_budget::SelectorWorkBudget::new(16);
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
    assert!(matches_selector_list(&warm, &tree.element(0), &mut context));
    context.set_work_budget(Some(&small));
    assert!(!matches_selector_list(
        &target,
        &tree.element(0),
        &mut context
    ));
    assert!(small.exhausted());
    context.set_work_budget(Some(&broad));
    assert!(
        matches_selector_list(&target, &tree.element(0), &mut context),
        "an exhausted evaluation is not evidence of nonmatching"
    );
}

#[test]
fn exhausted_nth_indexing_does_not_cache_a_zero_position() {
    let mut records = vec![Record {
        name: "main",
        ..Record::default()
    }];
    for _ in 0..101 {
        records.push(Record {
            name: "p",
            parent: Some(0),
            ..Record::default()
        });
    }
    let tree = Tree::new(records);
    let selector = parse("p:nth-child(101)");
    let small = crate::work_budget::SelectorWorkBudget::new(16);
    let broad = crate::work_budget::SelectorWorkBudget::new(10000);
    let mut caches = SelectorCaches::default();
    let mut context = MatchingContext::new(
        MatchingMode::Normal,
        None,
        &mut caches,
        QuirksMode::NoQuirks,
        NeedsSelectorFlags::No,
        MatchingForInvalidation::No,
    );
    context.set_work_budget(Some(&small));
    assert!(!matches_selector_list(
        &selector,
        &tree.element(101),
        &mut context
    ));
    assert!(small.exhausted());
    context.set_work_budget(Some(&broad));
    assert!(
        matches_selector_list(&selector, &tree.element(101), &mut context),
        "an exhausted positional walk has no valid index to cache"
    );
}

#[path = "matching_states.rs"]
mod states;

#[test]
fn relative_filter_construction_preserves_nested_values_and_reuses_complete_filters() {
    let tree = Tree::new(vec![
        Record {
            name: "main",
            ..Record::default()
        },
        Record {
            name: "div",
            parent: Some(0),
            ..Record::default()
        },
        Record {
            name: "span",
            parent: Some(1),
            classes: &["late"],
            ..Record::default()
        },
        Record {
            name: "p",
            parent: Some(0),
            ..Record::default()
        },
    ]);
    let mut caches = SelectorCaches::default();
    let mut context = MatchingContext::new(
        MatchingMode::Normal,
        None,
        &mut caches,
        QuirksMode::NoQuirks,
        NeedsSelectorFlags::No,
        MatchingForInvalidation::No,
    );
    let cases = [
        ("main:has(.late)", true),
        ("main:has(.missing)", false),
        ("main:has(:is(.late, .other))", true),
        ("main:has(.never)", false),
    ];
    // Cache keys borrow selector identity; keep every compiled selector alive for the pass.
    let compiled: Vec<_> = cases.iter().map(|(css, _)| parse(css)).collect();
    for ((css, expected), selectors) in cases.into_iter().zip(&compiled) {
        assert_eq!(
            matches_selector_list(selectors, &tree.element(0), &mut context),
            expected,
            "{css}"
        );
    }
    assert_eq!(
        tree.hash_visits.get(),
        3,
        "a complete descendant filter visits each descendant exactly once"
    );
}

#[path = "matching_contexts.rs"]
mod contexts;

#[test]
fn child_only_and_single_logical_filters_preserve_matches_without_descendant_leakage() {
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
            name: "div",
            parent: Some(0),
            ..Record::default()
        },
        Record {
            name: "span",
            parent: Some(2),
            classes: &["deep"],
            ..Record::default()
        },
    ]);
    let cases = [
        ("main:has(> p)", true),
        ("main:has(> p.x)", true),
        ("main:has(> .deep)", false),
        ("main:has(.x)", true),
        ("main:has(:is(.x))", true),
        ("main:has(:is(.missing))", false),
    ];
    let compiled: Vec<_> = cases.iter().map(|(css, _)| parse(css)).collect();
    let mut caches = SelectorCaches::default();
    let mut context = MatchingContext::new(
        MatchingMode::Normal,
        None,
        &mut caches,
        QuirksMode::NoQuirks,
        NeedsSelectorFlags::No,
        MatchingForInvalidation::No,
    );
    for ((css, expected), selectors) in cases.into_iter().zip(&compiled) {
        assert_eq!(
            matches_selector_list(selectors, &tree.element(0), &mut context),
            expected,
            "{css}"
        );
    }
    assert_eq!(
        tree.hash_visits.get(),
        5,
        "child and descendant filters each visit only their declared domain"
    );
}

#[path = "matching_failures.rs"]
mod failures;

#[path = "matching_diagnostics.rs"]
mod diagnostics;

#[path = "matching_attributes.rs"]
mod attributes;

#[path = "matching_budget_states.rs"]
mod budget_states;
