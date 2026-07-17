use super::*;

#[test]
fn adapter_rejects_invalid_delimiter_shapes_and_unsupported_outputs() {
    let selector_source = selector_source();
    let delimiter_match = delimiter_match();
    let delimiter_missing_attributes_error = v1::project_structured_match_for_tests(
        &ExtractionMatch {
            value: json!({
                "selectedHtmlOutput": "<article>Hello</article>",
                "textOutput": "Hello",
                "innerHtmlOutput": "Hello",
                "outerHtmlOutput": "<article>Hello</article>"
            }),
            ..delimiter_match.clone()
        },
        StrategyKind::DelimiterPair,
        &[],
    )
    .expect_err("missing delimiter attributes");
    assert_eq!(
        delimiter_missing_attributes_error.error_code,
        ErrorCode::InternalError
    );
    assert!(
        delimiter_missing_attributes_error
            .message
            .contains("\"attributes\"")
    );

    let delimiter_non_string_attribute_error = v1::project_structured_match_for_tests(
        &ExtractionMatch {
            value: json!({
                "selectedHtmlOutput": "<article>Hello</article>",
                "textOutput": "Hello",
                "innerHtmlOutput": "Hello",
                "outerHtmlOutput": "<article>Hello</article>",
                "attributes": {"data-id": 7}
            }),
            ..delimiter_match.clone()
        },
        StrategyKind::DelimiterPair,
        &[],
    )
    .expect_err("non-string delimiter attribute");
    assert_eq!(
        delimiter_non_string_attribute_error.error_code,
        ErrorCode::InternalError
    );
    assert!(
        delimiter_non_string_attribute_error
            .message
            .contains("non-string attribute value")
    );

    let selected_html_projection_error = v1::adapt_successful_extraction_for_tests(
        &selector_source,
        &Plan::new(
            PlanStrategy::css_selector(css_selector("article")),
            Selection::single(),
            Output::selected_html(),
            Rendering::new(TextWhitespace::Normalize, false),
        ),
        successful_selector_extraction(vec![selector_core_match(1, 1, 1)], 1, None),
    )
    .expect_err("selector selected_html projection should fail");
    assert_eq!(
        selected_html_projection_error.error_code,
        ErrorCode::InternalError
    );
    assert!(
        selected_html_projection_error
            .message
            .contains("selected_html")
    );

    let missing_attribute_error = v1::adapt_successful_extraction_for_tests(
        &selector_source,
        &Plan::new(
            PlanStrategy::css_selector(css_selector("article")),
            Selection::single(),
            Output::attribute(output_attribute_name("href")),
            Rendering::new(TextWhitespace::Normalize, false),
        ),
        successful_selector_extraction(vec![selector_core_match(1, 1, 1)], 1, None),
    )
    .expect_err("missing selector attribute should map to an interop error");
    assert_eq!(
        missing_attribute_error.error_code,
        ErrorCode::MissingAttribute
    );
    assert!(
        missing_attribute_error
            .message
            .contains("missing attribute")
    );
}
