use super::*;

#[test]
fn request_compilation_maps_selection_strategy_and_flags() {
    let selector_source =
        HtmlInput::new("inline", "<article>Hello</article>").expect("selector source");
    let selector_request = v1::compile_request_for_tests(&selector_source, &selector_plan());
    assert_eq!(
        selector_request.extraction.strategy(),
        ExtractionStrategy::Selector
    );
    assert_eq!(
        selector_request.output.rendering.whitespace,
        WhitespaceMode::Normalize
    );
    assert!(!selector_request.output.rendering.rewrite_urls);
    let first_request = v1::compile_request_for_tests(
        &selector_source,
        &Plan::new(
            PlanStrategy::css_selector(css_selector("article")),
            Selection::first(),
            Output::text(),
            Rendering::new(TextWhitespace::Normalize, false),
        ),
    );
    assert!(matches!(
        first_request.extraction.selection(),
        SelectionSpec::First
    ));

    let delimiter_source =
        HtmlInput::new("inline", "<article>Hello</article>").expect("delimiter source");
    let delimiter_request = v1::compile_request_for_tests(&delimiter_source, &delimiter_plan());
    assert_eq!(
        delimiter_request.extraction.strategy(),
        ExtractionStrategy::Slice
    );
    assert_eq!(
        delimiter_request.output.rendering.whitespace,
        WhitespaceMode::Rendered
    );
    assert!(delimiter_request.output.rendering.rewrite_urls);
    assert_eq!(
        v1::compile_regex_flags_for_tests(&[
            RegexFlag::CaseInsensitive,
            RegexFlag::MultiLine,
            RegexFlag::DotMatchesNewLine,
            RegexFlag::SwapGreed,
            RegexFlag::IgnoreWhitespace,
        ]),
        "imsUx"
    );
}
