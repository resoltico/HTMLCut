// SPDX-License-Identifier: MPL-2.0
use crate::budget::WorkBudget;
use crate::dom::{ElementRef, Html, Selector};

fn source(count: usize) -> String {
    format!("<main>{}<p>A</p></main>", "<p>x</p>".repeat(count - 1))
}

#[test]
fn positional_pass_reuses_scratch_with_linear_charged_work() {
    let mut measurements = Vec::new();
    for count in [1000, 2000, 4000, 8000] {
        let html = Html::parse_document(&source(count));
        let selector = Selector::parse(&format!("p:nth-child({count})")).unwrap();
        let budget = WorkBudget::new(1_000_000);
        let matches = crate::execution::matches(&html, None, &selector, 100_000, &budget).unwrap();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].text().collect::<String>(), "A");
        let used = 1_000_000 - budget.remaining();
        assert!(
            used <= 16 * count as u32 + 100,
            "{count} siblings used {used} steps"
        );
        measurements.push(used);
    }
    for pair in measurements.windows(2) {
        assert!(pair[1] <= 2 * pair[0] + 100);
    }
}

#[test]
fn uncached_positional_walk_is_charged_even_when_it_would_match() {
    let html = Html::parse_document(&source(10000));
    let last = html
        .tree
        .root()
        .descendants()
        .filter_map(ElementRef::wrap)
        .filter(|e| e.value().name() == "p")
        .last()
        .unwrap();
    for css in ["p:nth-child(10000)", "p:nth-of-type(10000)"] {
        let selector = Selector::parse(css).unwrap();
        let budget = WorkBudget::new(100);
        assert!(
            selector
                .budgeted(&html, None, &budget)
                .unwrap()
                .matches(&last)
                .is_err()
        );
        assert!(budget.exhausted());
        assert_eq!(budget.remaining(), 0);
    }
}

#[test]
fn structural_and_relative_selector_passes_have_independent_complete_id_oracles() {
    let html = Html::parse_document(
        "<main id='m'><section id='s' class='Alpha beta'><p id='a' data-code='AB-cd' class='x'>A</p><a id='b' href='/'>B</a><p id='c' class='x y'>C</p><div id='d'><span id='e'>E</span></div><p id='f'></p></section><aside id='g'><p id='h'>H</p></aside></main>",
    );
    let cases: &[(&str, &[&str])] = &[
        ("main > section", &["s"]),
        ("section p", &["a", "c", "f"]),
        ("p + a", &["b"]),
        ("a ~ p", &["c", "f"]),
        ("section > :first-child", &["a"]),
        ("section > :last-child", &["f"]),
        ("span:only-child", &["e"]),
        ("aside p:only-of-type", &["h"]),
        ("section p:first-of-type", &["a"]),
        ("section p:last-of-type", &["f"]),
        ("section p:nth-child(odd)", &["a", "c", "f"]),
        ("section p:nth-child(even)", &[]),
        ("section p:nth-last-child(3)", &["c"]),
        ("section p:nth-of-type(2)", &["c"]),
        ("section p:nth-last-of-type(2)", &["c"]),
        ("section > :not(p)", &["b", "d"]),
        ("section > :is(a, div)", &["b", "d"]),
        ("section > :where(.x, a)", &["a", "b", "c"]),
        ("[data-code]", &["a"]),
        ("[data-code='AB-cd']", &["a"]),
        ("[data-code='ab-CD' i]", &["a"]),
        ("[data-code='ab-CD' s]", &[]),
        ("[data-code^='AB']", &["a"]),
        ("[data-code$='cd']", &["a"]),
        ("[data-code*='B-c']", &["a"]),
        ("[data-code|='AB']", &["a"]),
        ("[class~='y']", &["c"]),
        (".Alpha", &["s"]),
        (".alpha", &[]),
        ("p:empty", &["f"]),
        ("section:has(> p.y)", &["s"]),
        ("section:has(span)", &["s"]),
        ("section:has(> span)", &[]),
        ("section:has(+ aside)", &["s"]),
        ("section:has(~ aside p)", &["s"]),
        ("p:has(+ a)", &["a"]),
        ("p:has(~ div span)", &["a", "c"]),
        ("main:has(section > p.y, aside > p)", &["m"]),
        ("section:not(:has(.absent))", &["s"]),
        ("section:has(:is(p.y, span))", &["s"]),
        ("#a, #c, #h", &["a", "c", "h"]),
    ];
    for &(css, expected) in cases {
        let selector = Selector::parse(css).unwrap_or_else(|error| panic!("{css}: {error}"));
        for _ in 0..2 {
            let budget = WorkBudget::new(100_000);
            let selected = crate::execution::matches(&html, None, &selector, 100, &budget).unwrap();
            let actual = selected
                .iter()
                .map(|element| element.value().attr("id").unwrap())
                .collect::<Vec<_>>();
            assert_eq!(actual, expected, "{css}");
            assert!(!budget.exhausted());
        }
    }
}

#[test]
fn relative_search_refuses_exhausted_work_instead_of_returning_false() {
    let html = Html::parse_document(&format!(
        "<main>{}<aside id='target'><span></span></aside></main>",
        "<section><div><p>x</p></div></section>".repeat(1000)
    ));
    let root = html
        .select(&Selector::parse("main").unwrap())
        .next()
        .unwrap();
    for css in [
        "main:has(span)",
        "main:has(> aside)",
        "main:has(section ~ aside span)",
    ] {
        let selector = Selector::parse(css).unwrap();
        let budget = WorkBudget::new(100);
        assert!(
            selector
                .budgeted(&html, None, &budget)
                .unwrap()
                .matches(&root)
                .is_err(),
            "{css}"
        );
        assert!(budget.exhausted());
    }
}

#[test]
fn unsupported_browser_state_and_filtered_positional_grammar_is_refused() {
    for css in [
        "p:nth-child(2 of p)",
        "p:nth-last-child(1 of .x)",
        "a:any-link",
        "a:link",
        "a:visited",
    ] {
        assert!(Selector::parse(css).is_err(), "{css}");
    }
}

#[test]
fn mismatched_internal_scope_has_an_invariant_error_not_a_resource_error() {
    let first = Html::parse_document("<main><p>A</p></main>");
    let second = Html::parse_document("<main><p>B</p></main>");
    let scope = second
        .select(&Selector::parse("main").unwrap())
        .next()
        .unwrap();
    let budget = WorkBudget::new(1000);
    let error = crate::execution::matches(
        &first,
        Some(scope),
        &Selector::parse("p").unwrap(),
        10,
        &budget,
    )
    .unwrap_err();
    assert_eq!(error.code, crate::ErrorCode::InternalInvariant);
    assert_eq!(error.code.exit_class(), 6);
    assert_eq!(budget.remaining(), 1000);
}

#[test]
fn ancestor_sibling_negation_and_scope_paths_have_independent_id_oracles() {
    let html = Html::parse_document(
        "<main id='m'><section id='s'><p id='a'>A</p>text<!--gap--><p id='b' class='x'>B</p><div id='d'><p id='c'>C</p></div><p id='e'></p></section><aside id='z'><p id='f'>F</p></aside></main>",
    );
    let cases: &[(&str, &[&str])] = &[
        ("main section p", &["a", "b", "c", "e"]),
        ("main .absent p", &[]),
        ("main > .absent p", &[]),
        ("p + p", &["b"]),
        ("p ~ p", &["b", "e"]),
        ("p + .absent ~ p", &[]),
        ("p:not(.x)", &["a", "c", "e", "f"]),
        ("section:not(:has(.absent))", &["s"]),
        ("section:not(:has(p.x))", &[]),
        ("section:has(> p + p.x)", &["s"]),
        ("section:has(+ aside > p)", &["s"]),
        ("section:has(~ .absent > p)", &[]),
        ("section > p:nth-child(2)", &["b"]),
        ("section > p:nth-last-child(1)", &["e"]),
    ];
    for &(css, expected) in cases {
        let selector = Selector::parse(css).unwrap();
        let budget = WorkBudget::new(100_000);
        let selected = crate::execution::matches(&html, None, &selector, 100, &budget).unwrap();
        let actual: Vec<_> = selected
            .iter()
            .map(|e| e.value().attr("id").unwrap())
            .collect();
        assert_eq!(actual, expected, "{css}");
    }
    let scope = html.select(&Selector::parse("#s").unwrap()).next().unwrap();
    for (css, expected) in [
        (":scope > p", vec!["a", "b", "e"]),
        (":scope p", vec!["a", "b", "c", "e"]),
    ] {
        let budget = WorkBudget::new(100_000);
        let selector = Selector::parse(css).unwrap();
        let selected =
            crate::execution::matches(&html, Some(scope), &selector, 100, &budget).unwrap();
        let actual: Vec<_> = selected
            .iter()
            .map(|e| e.value().attr("id").unwrap())
            .collect();
        assert_eq!(actual, expected, "{css}");
    }
}

#[test]
fn warm_and_cold_matchers_agree_in_forward_reverse_and_repeated_orders() {
    let html = Html::parse_document(
        "<main id='m'><p id='a'></p><div id='d'><span id='s'></span></div><p id='b' class='x'></p><aside id='z'><p id='c'></p></aside><p id='e'></p></main>",
    );
    let elements: Vec<_> = html
        .tree
        .root()
        .descendants()
        .filter_map(ElementRef::wrap)
        .filter(|e| e.value().attr("id").is_some())
        .collect();
    let cases: &[(&str, &[&str])] = &[
        ("main > p:nth-child(3)", &["b"]),
        ("main > p:nth-last-child(1)", &["e"]),
        ("main > p:nth-of-type(2)", &["b"]),
        ("main > p:nth-last-of-type(2)", &["b"]),
        ("main:has(aside p)", &["m"]),
        ("p:has(~ aside p)", &["a", "b"]),
        ("div:not(:has(.missing))", &["d"]),
        ("main:has(> .missing)", &[]),
    ];
    for &(css, expected) in cases {
        let selector = Selector::parse(css).unwrap();
        let budget = WorkBudget::new(100_000);
        let mut warm = selector.budgeted(&html, None, &budget).unwrap();
        for order in [
            elements.clone(),
            elements.iter().rev().copied().collect(),
            elements.clone(),
        ] {
            for element in order {
                let id = element.value().attr("id").unwrap();
                let cold_budget = WorkBudget::new(100_000);
                let mut cold = selector.budgeted(&html, None, &cold_budget).unwrap();
                let expected = expected.contains(&id);
                assert_eq!(
                    cold.matches(&element).unwrap(),
                    expected,
                    "cold {css} #{id}"
                );
                assert_eq!(
                    warm.matches(&element).unwrap(),
                    expected,
                    "warm {css} #{id}"
                );
            }
        }
    }
}

#[test]
fn non_element_sibling_navigation_and_negated_search_cannot_hide_exhaustion() {
    let html = Html::parse_document(&format!(
        "<main><p id='a'>A</p>{}<p id='b'>B</p><aside><span class='late'></span></aside></main>",
        "<!--gap-->text".repeat(200)
    ));
    let last = html.select(&Selector::parse("#b").unwrap()).next().unwrap();
    for css in ["p + p", "p ~ p", "p:nth-child(2)"] {
        let selector = Selector::parse(css).unwrap();
        let budget = WorkBudget::new(30);
        let mut matcher = selector.budgeted(&html, None, &budget).unwrap();
        assert!(matcher.matches(&last).is_err(), "{css}");
        assert!(budget.exhausted(), "{css}");
        assert!(matcher.matches(&last).is_err(), "sticky {css}");
    }
    let root = html
        .select(&Selector::parse("main").unwrap())
        .next()
        .unwrap();
    for css in ["main:not(:has(.missing))", "main:not(:has(.late))"] {
        let selector = Selector::parse(css).unwrap();
        let budget = WorkBudget::new(30);
        let mut matcher = selector.budgeted(&html, None, &budget).unwrap();
        assert!(matcher.matches(&root).is_err(), "{css}");
        assert!(budget.exhausted());
        assert!(matcher.matches(&root).is_err(), "sticky {css}");
    }
}

#[test]
fn relative_descendant_search_remains_safe_at_the_supported_dom_depth() {
    // Use a DOM close to the public maximum; an explicit oracle also checks the
    // recursive upstream matcher after removal of the owned iterative traversal.
    let depth = 4000;
    let html = Html::parse_document(&format!(
        "<main>{}<span id='target'></span>{}</main>",
        "<div>".repeat(depth),
        "</div>".repeat(depth)
    ));
    let root = html
        .select(&Selector::parse("main").unwrap())
        .next()
        .unwrap();
    for (css, expected) in [("main:has(span)", true), ("main:has(.missing)", false)] {
        let selector = Selector::parse(css).unwrap();
        let budget = WorkBudget::new(100_000);
        assert_eq!(
            selector
                .budgeted(&html, None, &budget)
                .unwrap()
                .matches(&root)
                .unwrap(),
            expected,
            "{css}"
        );
        let limited = WorkBudget::new(30);
        assert!(
            selector
                .budgeted(&html, None, &limited)
                .unwrap()
                .matches(&root)
                .is_err(),
            "{css}"
        );
        assert!(limited.exhausted());
    }
}

#[test]
fn long_attribute_class_and_id_predicates_refuse_before_unbounded_comparison() {
    let tail = "x".repeat(64_000);
    let html = Html::parse_document(&format!(
        "<main><p id='needle{tail}' data-code='needle{tail}' class='needle {tail}'></p></main>"
    ));
    let element = html.select(&Selector::parse("p").unwrap()).next().unwrap();
    for (css, expected) in [
        (".needle", true),
        ("[data-code^='needle']", true),
        ("#needle", false),
    ] {
        let selector = Selector::parse(css).unwrap();
        let ample = WorkBudget::new(100_000);
        assert_eq!(
            selector
                .budgeted(&html, None, &ample)
                .unwrap()
                .matches(&element)
                .unwrap(),
            expected,
            "{css}"
        );
        let limited = WorkBudget::new(30);
        assert!(
            selector
                .budgeted(&html, None, &limited)
                .unwrap()
                .matches(&element)
                .is_err(),
            "{css}"
        );
        assert!(limited.exhausted());
    }
}
