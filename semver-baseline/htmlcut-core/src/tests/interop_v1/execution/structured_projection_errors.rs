use super::*;

#[test]
fn structured_projection_rejects_malformed_selector_and_delimiter_matches() {
    let selector_match = selector_match();
    let delimiter_match = delimiter_match();
    let non_object_error = v1::project_structured_match_for_tests(
        &ExtractionMatch {
            value: json!("not-object"),
            ..selector_match.clone()
        },
        StrategyKind::CssSelector,
        &[],
    )
    .expect_err("non-object structured payload");
    assert_eq!(non_object_error.error_code, ErrorCode::InternalError);
    assert!(
        non_object_error
            .message
            .contains("structured core match payload")
    );

    let missing_field_error = v1::project_structured_match_for_tests(
        &ExtractionMatch {
            value: json!({"textOutput": "Hello", "innerHtmlOutput": "Hello"}),
            ..selector_match.clone()
        },
        StrategyKind::CssSelector,
        &[],
    )
    .expect_err("missing outerHtmlOutput");
    assert_eq!(missing_field_error.error_code, ErrorCode::InternalError);
    assert!(missing_field_error.message.contains("\"outerHtmlOutput\""));

    let selector_zero_index_error = v1::project_structured_match_for_tests(
        &ExtractionMatch {
            metadata: ExtractionMatchMetadata::Selector(SelectorMatchMetadata {
                candidate_count: 1,
                candidate_index: 0,
                path: "article:nth-of-type(1)".to_owned(),
                tag_name: "article".to_owned(),
                attributes: BTreeMap::new(),
            }),
            ..selector_match.clone()
        },
        StrategyKind::CssSelector,
        &[],
    )
    .expect_err("zero selector candidate index");
    assert_eq!(
        selector_zero_index_error.error_code,
        ErrorCode::InternalError
    );
    assert!(
        selector_zero_index_error
            .message
            .contains("zero candidate index")
    );

    let selector_missing_text_error = v1::project_structured_match_for_tests(
        &ExtractionMatch {
            value: json!({
                "innerHtmlOutput": "Hello",
                "outerHtmlOutput": "<article>Hello</article>"
            }),
            ..selector_match.clone()
        },
        StrategyKind::CssSelector,
        &[],
    )
    .expect_err("missing text");
    assert_eq!(
        selector_missing_text_error.error_code,
        ErrorCode::InternalError
    );
    assert!(
        selector_missing_text_error
            .message
            .contains("\"textOutput\"")
    );

    let selector_non_string_comparison_error = v1::project_structured_match_for_tests(
        &ExtractionMatch {
            value: json!({
                "textOutput": "Hello",
                "comparisonTextOutput": 7,
                "innerHtmlOutput": "Hello",
                "outerHtmlOutput": "<article>Hello</article>"
            }),
            ..selector_match.clone()
        },
        StrategyKind::CssSelector,
        &[],
    )
    .expect_err("non-string comparison text");
    assert_eq!(
        selector_non_string_comparison_error.error_code,
        ErrorCode::InternalError
    );
    assert!(
        selector_non_string_comparison_error
            .message
            .contains("comparisonTextOutput")
    );

    let delimiter_missing_selected_html_error = v1::project_structured_match_for_tests(
        &ExtractionMatch {
            value: json!({
                "textOutput": "Hello",
                "innerHtmlOutput": "Hello",
                "outerHtmlOutput": "<article>Hello</article>",
                "attributes": {}
            }),
            ..delimiter_match.clone()
        },
        StrategyKind::DelimiterPair,
        &[],
    )
    .expect_err("missing delimiter selectedHtmlOutput");
    assert_eq!(
        delimiter_missing_selected_html_error.error_code,
        ErrorCode::InternalError
    );
    assert!(
        delimiter_missing_selected_html_error
            .message
            .contains("\"selectedHtmlOutput\"")
    );

    let delimiter_missing_text_error = v1::project_structured_match_for_tests(
        &ExtractionMatch {
            value: json!({
                "selectedHtmlOutput": "<article>Hello</article>",
                "innerHtmlOutput": "Hello",
                "outerHtmlOutput": "<article>Hello</article>",
                "attributes": {}
            }),
            ..delimiter_match.clone()
        },
        StrategyKind::DelimiterPair,
        &[],
    )
    .expect_err("missing delimiter text");
    assert_eq!(
        delimiter_missing_text_error.error_code,
        ErrorCode::InternalError
    );
    assert!(
        delimiter_missing_text_error
            .message
            .contains("\"textOutput\"")
    );

    let delimiter_missing_inner_html_error = v1::project_structured_match_for_tests(
        &ExtractionMatch {
            value: json!({
                "selectedHtmlOutput": "<article>Hello</article>",
                "textOutput": "Hello",
                "outerHtmlOutput": "<article>Hello</article>",
                "attributes": {}
            }),
            ..delimiter_match.clone()
        },
        StrategyKind::DelimiterPair,
        &[],
    )
    .expect_err("missing delimiter innerHtmlOutput");
    assert_eq!(
        delimiter_missing_inner_html_error.error_code,
        ErrorCode::InternalError
    );
    assert!(
        delimiter_missing_inner_html_error
            .message
            .contains("\"innerHtmlOutput\"")
    );

    let delimiter_missing_outer_html_error = v1::project_structured_match_for_tests(
        &ExtractionMatch {
            value: json!({
                "selectedHtmlOutput": "<article>Hello</article>",
                "textOutput": "Hello",
                "innerHtmlOutput": "Hello",
                "attributes": {}
            }),
            ..delimiter_match.clone()
        },
        StrategyKind::DelimiterPair,
        &[],
    )
    .expect_err("missing delimiter outerHtmlOutput");
    assert_eq!(
        delimiter_missing_outer_html_error.error_code,
        ErrorCode::InternalError
    );
    assert!(
        delimiter_missing_outer_html_error
            .message
            .contains("\"outerHtmlOutput\"")
    );

    let zero_index_error = v1::project_structured_match_for_tests(
        &ExtractionMatch {
            metadata: ExtractionMatchMetadata::DelimiterPair(DelimiterPairMatchMetadata {
                candidate_count: 1,
                candidate_index: 0,
                selected_range: Range { start: 0, end: 22 },
                inner_range: Range { start: 9, end: 14 },
                outer_range: Range { start: 0, end: 22 },
                include_start: true,
                include_end: false,
                matched_start: "<article>".to_owned(),
                matched_end: "</article>".to_owned(),
            }),
            ..delimiter_match.clone()
        },
        StrategyKind::DelimiterPair,
        &[],
    )
    .expect_err("zero candidate index");
    assert_eq!(zero_index_error.error_code, ErrorCode::InternalError);
    assert!(zero_index_error.message.contains("zero candidate index"));
}
