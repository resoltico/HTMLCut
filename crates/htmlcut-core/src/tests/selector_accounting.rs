use scraper::{ElementRef, Html, Selector};
use selectors::work_budget::SelectorWorkBudget;

fn source(count: usize) -> String {
    format!("<main>{}<p>A</p></main>", "<p>x</p>".repeat(count - 1))
}

#[test]
fn positional_pass_reuses_scratch_with_linear_charged_work() {
    let mut measurements = Vec::new();
    for count in [1000, 2000, 4000, 8000] {
        let html = Html::parse_document(&source(count));
        let selector = Selector::parse(&format!("p:nth-child({count})")).unwrap();
        let budget = SelectorWorkBudget::new(1_000_000);
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
        let budget = SelectorWorkBudget::new(100);
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
            let budget = SelectorWorkBudget::new(100_000);
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
        let budget = SelectorWorkBudget::new(100);
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
    let budget = SelectorWorkBudget::new(1000);
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
