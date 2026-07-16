use super::*;

#[test]
fn interop_execution_helpers_cover_compile_projection_and_error_paths() {
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

    let selector_match = ExtractionMatch {
        index: 1,
        path: Some("article:nth-of-type(1)".to_owned()),
        value_type: ValueType::Structured,
        value: json!({
            "textOutput": "Hello",
            "innerHtmlOutput": "Hello",
            "outerHtmlOutput": "<article>Hello</article>"
        }),
        html: Some("<article>Hello</article>".to_owned()),
        text: Some("Hello".to_owned()),
        preview: "Hello".to_owned(),
        metadata: ExtractionMatchMetadata::Selector(SelectorMatchMetadata {
            candidate_count: 1,
            candidate_index: 1,
            path: "article:nth-of-type(1)".to_owned(),
            tag_name: "article".to_owned(),
            attributes: BTreeMap::new(),
        }),
    };
    assert!(
        v1::project_structured_match_for_tests(&selector_match, StrategyKind::CssSelector, &[])
            .is_ok()
    );

    let delimiter_match = ExtractionMatch {
        index: 1,
        path: None,
        value_type: ValueType::Structured,
        value: json!({
            "textOutput": "Hello",
            "selectedHtmlOutput": "<article>Hello</article>",
            "innerHtmlOutput": "Hello",
            "outerHtmlOutput": "<article>Hello</article>",
            "attributes": {}
        }),
        html: Some("<article>Hello</article>".to_owned()),
        text: Some("Hello".to_owned()),
        preview: "Hello".to_owned(),
        metadata: ExtractionMatchMetadata::DelimiterPair(DelimiterPairMatchMetadata {
            candidate_count: 1,
            candidate_index: 1,
            selected_range: Range { start: 0, end: 22 },
            inner_range: Range { start: 9, end: 14 },
            outer_range: Range { start: 0, end: 22 },
            include_start: true,
            include_end: false,
            matched_start: "<article>".to_owned(),
            matched_end: "</article>".to_owned(),
        }),
    };
    assert!(
        v1::project_structured_match_for_tests(&delimiter_match, StrategyKind::DelimiterPair, &[])
            .is_ok()
    );

    let adapted_text = v1::adapt_successful_extraction_for_tests(
        &selector_source
            .clone()
            .with_input_base_url(http_url("https://example.com/start.html")),
        &selector_plan(),
        successful_selector_extraction(
            vec![selector_core_match(1, 1, 1)],
            1,
            Some("https://example.com/base.html"),
        ),
    )
    .expect("adapted text result");
    assert_eq!(adapted_text.selected_matches.len(), 1);
    assert_eq!(adapted_text.output.kind(), OutputKind::Text);
    assert_eq!(
        adapted_text.selected_matches[0].output_value,
        Value::String("Hello".to_owned())
    );
    assert_eq!(
        adapted_text
            .source
            .effective_base_url
            .as_ref()
            .map(DisplayedHttpUrl::as_str),
        Some("https://example.com/base.html")
    );

    let adapted_comparison_text = v1::adapt_successful_extraction_for_tests(
        &selector_source,
        &selector_plan(),
        successful_selector_extraction(
            vec![ExtractionMatch {
                value: json!({
                    "textOutput": "Raw evidence",
                    "comparisonTextOutput": "Canonical comparison",
                    "innerHtmlOutput": "Raw evidence",
                    "outerHtmlOutput": "<article>Raw evidence</article>"
                }),
                ..selector_core_match(1, 1, 1)
            }],
            1,
            None,
        ),
    )
    .expect("comparison text projection");
    assert_eq!(
        adapted_comparison_text.selected_matches[0].text_output,
        "Raw evidence"
    );
    assert_eq!(
        adapted_comparison_text.selected_matches[0]
            .comparison_text_output
            .as_deref(),
        Some("Canonical comparison")
    );
    assert_eq!(
        adapted_comparison_text.selected_matches[0].output_value,
        Value::String("Canonical comparison".to_owned())
    );

    let adapted_comparison_structured = v1::adapt_successful_extraction_for_tests(
        &selector_source,
        &Plan::new(
            PlanStrategy::css_selector(css_selector("article")),
            Selection::single(),
            Output::structured(),
            Rendering::new(TextWhitespace::Normalize, false),
        ),
        successful_selector_extraction(
            vec![ExtractionMatch {
                value: json!({
                    "textOutput": "Raw evidence",
                    "comparisonTextOutput": "Canonical comparison",
                    "innerHtmlOutput": "Raw evidence",
                    "outerHtmlOutput": "<article>Raw evidence</article>"
                }),
                ..selector_core_match(1, 1, 1)
            }],
            1,
            None,
        ),
    )
    .expect("comparison structured projection");
    let selected = &adapted_comparison_structured.selected_matches[0];
    assert_eq!(
        selected.comparison_text_output.as_deref(),
        Some("Canonical comparison")
    );
    assert_eq!(
        selected.output_value,
        json!({
            "textOutput": "Raw evidence",
            "innerHtmlOutput": "Raw evidence",
            "outerHtmlOutput": "<article>Raw evidence</article>"
        })
    );

    let adapted_inner = v1::adapt_successful_extraction_for_tests(
        &selector_source,
        &Plan::new(
            PlanStrategy::css_selector(css_selector("article")),
            Selection::single(),
            Output::inner_html(),
            Rendering::new(TextWhitespace::Normalize, false),
        ),
        successful_selector_extraction(vec![selector_core_match(1, 1, 1)], 1, None),
    )
    .expect("adapted inner-html result");
    assert_eq!(
        adapted_inner.selected_matches[0].output_value,
        Value::String("Hello".to_owned())
    );

    let adapted_outer = v1::adapt_successful_extraction_for_tests(
        &selector_source,
        &Plan::new(
            PlanStrategy::css_selector(css_selector("article")),
            Selection::single(),
            Output::outer_html(),
            Rendering::new(TextWhitespace::Normalize, false),
        ),
        successful_selector_extraction(vec![selector_core_match(1, 1, 1)], 1, None),
    )
    .expect("adapted outer-html result");
    assert!(
        adapted_outer.selected_matches[0]
            .output_value
            .as_str()
            .is_some_and(|html| html.contains("<article"))
    );

    let adapted_all = v1::adapt_successful_extraction_for_tests(
        &selector_source,
        &Plan::new(
            PlanStrategy::css_selector(css_selector("article")),
            Selection::all(),
            Output::text(),
            Rendering::new(TextWhitespace::Normalize, false),
        ),
        successful_selector_extraction(
            vec![selector_core_match(1, 1, 2), selector_core_match(2, 2, 2)],
            2,
            None,
        ),
    )
    .expect("adapted all-selection result");
    assert_eq!(adapted_all.selected_matches.len(), 2);
    assert_eq!(adapted_all.selection_mode, SelectionMode::All);
    assert_eq!(adapted_all.selected_matches[0].candidate_index.get(), 1);
    assert_eq!(adapted_all.selected_matches[1].candidate_index.get(), 2);

    let adapted_projection_error = v1::adapt_successful_extraction_for_tests(
        &selector_source,
        &selector_plan(),
        successful_selector_extraction(
            vec![ExtractionMatch {
                value: json!({"textOutput": "Hello", "outerHtmlOutput": "<article>Hello</article>"}),
                ..selector_core_match(1, 1, 1)
            }],
            1,
            None,
        ),
    )
    .expect_err("projection failure should surface as interop error");
    assert_eq!(
        adapted_projection_error.error_code,
        ErrorCode::InternalError
    );
    assert!(
        adapted_projection_error
            .message
            .contains("\"innerHtmlOutput\"")
    );

    let adapted_url_error = v1::adapt_successful_extraction_for_tests(
        &selector_source,
        &selector_plan(),
        successful_selector_extraction(vec![selector_core_match(1, 1, 1)], 1, Some("not a url")),
    )
    .expect_err("invalid effective base URL should surface as interop error");
    assert_eq!(adapted_url_error.error_code, ErrorCode::InternalError);
    assert!(adapted_url_error.message.contains("invalid URL"));

    let no_match_adapter_error = v1::adapt_successful_extraction_for_tests(
        &selector_source,
        &selector_plan(),
        successful_selector_extraction(Vec::new(), 0, None),
    )
    .expect_err("missing selected match");
    assert_eq!(no_match_adapter_error.error_code, ErrorCode::InternalError);
    assert!(
        no_match_adapter_error
            .message
            .contains("did not produce a selected match")
    );

    let multi_match_adapter_error = v1::adapt_successful_extraction_for_tests(
        &selector_source,
        &selector_plan(),
        successful_selector_extraction(
            vec![selector_core_match(1, 1, 2), selector_core_match(2, 2, 2)],
            2,
            None,
        ),
    )
    .expect_err("multiple selected matches");
    assert_eq!(
        multi_match_adapter_error.error_code,
        ErrorCode::InternalError
    );
    assert!(
        multi_match_adapter_error
            .message
            .contains("invalid interop result")
    );

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

    let invalid_url_error =
        v1::parse_optional_url_for_tests(Some("not a url"), "effective_base_url", &[])
            .expect_err("invalid url");
    assert_eq!(invalid_url_error.error_code, ErrorCode::InternalError);
    assert!(invalid_url_error.message.contains("invalid URL"));
    assert_eq!(
        v1::parse_optional_url_for_tests(None, "effective_base_url", &[]).expect("none url"),
        None
    );

    let no_primary_error = v1::core_execution_error_for_tests(&selector_plan(), &[]);
    assert_eq!(no_primary_error.error_code, ErrorCode::InternalError);
    assert!(
        no_primary_error
            .message
            .contains("without an error diagnostic")
    );

    let no_match_error = v1::core_execution_error_for_tests(
        &selector_plan(),
        &[Diagnostic {
            level: DiagnosticLevel::Error,
            code: DiagnosticCode::NoMatch,
            message: "no match".to_owned(),
            details: None,
        }],
    );
    assert_eq!(no_match_error.error_code, ErrorCode::NoMatch);
    assert_eq!(
        no_match_error.details["core_details"]["candidateCount"],
        json!(0)
    );

    let ambiguous_error = v1::core_execution_error_for_tests(
        &selector_plan(),
        &[Diagnostic {
            level: DiagnosticLevel::Error,
            code: DiagnosticCode::AmbiguousMatch,
            message: "ambiguous".to_owned(),
            details: Some(json!({"candidateCount": 2})),
        }],
    );
    assert_eq!(ambiguous_error.error_code, ErrorCode::AmbiguousMatch);
    assert!(ambiguous_error.details.contains_key("core_details"));

    let scalar_details_error = v1::core_execution_error_for_tests(
        &selector_plan(),
        &[Diagnostic {
            level: DiagnosticLevel::Error,
            code: DiagnosticCode::NoMatch,
            message: "no match with scalar details".to_owned(),
            details: Some(json!("legacy scalar detail")),
        }],
    );
    assert_eq!(
        scalar_details_error.details["core_details"]["diagnostic_details"],
        json!("legacy scalar detail")
    );
    assert_eq!(
        scalar_details_error.details["core_details"]["candidateCount"],
        json!(0)
    );

    let missing_attribute_core_error = v1::core_execution_error_for_tests(
        &Plan::new(
            PlanStrategy::css_selector(css_selector("article")),
            Selection::single(),
            Output::attribute(output_attribute_name("href")),
            Rendering::new(TextWhitespace::Normalize, false),
        ),
        &[Diagnostic {
            level: DiagnosticLevel::Error,
            code: DiagnosticCode::MissingAttribute,
            message: "missing attribute".to_owned(),
            details: None,
        }],
    );
    assert_eq!(
        missing_attribute_core_error.error_code,
        ErrorCode::MissingAttribute
    );

    let invalid_request_error = v1::core_execution_error_for_tests(
        &selector_plan(),
        &[Diagnostic {
            level: DiagnosticLevel::Error,
            code: DiagnosticCode::InvalidSelector,
            message: "CSS selector is invalid.".to_owned(),
            details: Some(selector_parse_details(1, 5, "invalid_attribute_selector")),
        }],
    );
    assert_eq!(invalid_request_error.error_code, ErrorCode::PlanInvalid);

    let unexpected_code_error = v1::core_execution_error_for_tests(
        &selector_plan(),
        &[Diagnostic {
            level: DiagnosticLevel::Error,
            code: DiagnosticCode::MultipleMatches,
            message: "weird".to_owned(),
            details: None,
        }],
    );
    assert_eq!(unexpected_code_error.error_code, ErrorCode::InternalError);

    let adapter_error = v1::internal_adapter_error_for_tests(
        "adapter failure",
        BTreeMap::from([("field".to_owned(), Value::from("effective_base_url"))]),
        Vec::new(),
    );
    assert_eq!(adapter_error.error_code, ErrorCode::InternalError);
    assert_eq!(adapter_error.error_digest_sha256.len(), 64);

    let fallback_error = v1::internal_adapter_error_with_plan_digest_for_tests(
        "not-a-digest",
        "adapter failure",
        BTreeMap::from([("field".to_owned(), Value::from("effective_base_url"))]),
        Vec::new(),
    );
    assert_eq!(fallback_error.error_code, ErrorCode::InternalError);
    assert_eq!(
        fallback_error.plan_digest_sha256,
        "0000000000000000000000000000000000000000000000000000000000000000"
    );
    assert_eq!(fallback_error.error_digest_sha256.len(), 64);
    assert!(
        fallback_error
            .message
            .contains("could not finalize its interop error payload")
    );
    assert!(fallback_error.validate().is_ok());

    let plan_digest_error = v1::plan_digest_error_for_tests(
        &selector_plan(),
        ContractError::InvalidDigest {
            field: "plan_digest_sha256",
            received: "not-a-digest".to_owned(),
        },
    );
    assert_eq!(plan_digest_error.error_code, ErrorCode::InternalError);
    assert_eq!(
        plan_digest_error.plan_digest_sha256,
        "0000000000000000000000000000000000000000000000000000000000000000"
    );
    assert!(
        plan_digest_error
            .message
            .contains("could not compute the interop plan digest")
    );

    let recoverable_error = v1::finalize_error_for_tests(InteropError {
        schema_name: "htmlcut.not-real".to_owned(),
        ..InteropError::new(
            TEST_PLAN_DIGEST_SHA256,
            ErrorCode::InternalError,
            "adapter failure",
            Some(StrategyKind::CssSelector),
            BTreeMap::new(),
            Vec::new(),
        )
    });
    assert_eq!(recoverable_error.schema_name, v1::ERROR_SCHEMA_NAME);
    assert_eq!(recoverable_error.error_code, ErrorCode::InternalError);
    assert_eq!(recoverable_error.error_digest_sha256.len(), 64);
    assert!(recoverable_error.validate().is_ok());

    let non_hex_digest_error = v1::internal_adapter_error_with_plan_digest_for_tests(
        "zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz",
        "adapter failure",
        BTreeMap::from([("field".to_owned(), Value::from("effective_base_url"))]),
        Vec::new(),
    );
    assert_eq!(
        non_hex_digest_error.plan_digest_sha256,
        "0000000000000000000000000000000000000000000000000000000000000000"
    );

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

    let _typed: Box<InteropError> = Box::new(adapter_error);
}

#[test]
fn error_finalization_replaces_rejected_payloads_with_bounded_typed_evidence() {
    let valid_selector_parse = selector_parse_details(1, 1, "invalid_attribute_selector");
    let oversized_root = InteropError::new(
        TEST_PLAN_DIGEST_SHA256,
        ErrorCode::InternalError,
        "x".repeat(1025),
        Some(StrategyKind::CssSelector),
        BTreeMap::new(),
        Vec::new(),
    );
    let oversized_diagnostic = InteropError::new(
        TEST_PLAN_DIGEST_SHA256,
        ErrorCode::NoMatch,
        "No matches were found.",
        Some(StrategyKind::CssSelector),
        BTreeMap::new(),
        vec![InteropDiagnostic {
            level: InteropDiagnosticLevel::Error,
            code: InteropDiagnosticCode::NoMatch,
            message: "x".repeat(1025),
            details: None,
        }],
    );
    let missing_selector_parse =
        invalid_selector_interop_error(json!({}), valid_selector_parse.clone());
    let malformed_selector_parse = invalid_selector_interop_error(
        json!({"selector_parse": {"line": 1}}),
        valid_selector_parse.clone(),
    );
    let non_object_selector_parse = invalid_selector_interop_error(
        json!({"selector_parse": false}),
        valid_selector_parse.clone(),
    );
    let zero_line_selector_parse = invalid_selector_interop_error(
        selector_parse_details(0, 1, "invalid_attribute_selector"),
        valid_selector_parse.clone(),
    );
    let zero_column_selector_parse = invalid_selector_interop_error(
        selector_parse_details(1, 0, "invalid_attribute_selector"),
        valid_selector_parse.clone(),
    );
    let unknown_class_selector_parse = invalid_selector_interop_error(
        selector_parse_details(1, 1, "not_a_class"),
        valid_selector_parse.clone(),
    );
    let mismatched_selector_parse = invalid_selector_interop_error(
        valid_selector_parse,
        selector_parse_details(2, 1, "invalid_attribute_selector"),
    );

    for rejected in [
        oversized_root,
        oversized_diagnostic,
        missing_selector_parse,
        malformed_selector_parse,
        non_object_selector_parse,
        zero_line_selector_parse,
        zero_column_selector_parse,
        unknown_class_selector_parse,
        mismatched_selector_parse,
    ] {
        let finalized = v1::finalize_error_for_tests(rejected);
        assert_eq!(finalized.error_code, ErrorCode::InternalError);
        assert!(finalized.diagnostics.is_empty());
        assert!(finalized.validate().is_ok());
        assert!(finalized.details["interop_contract_rejection"]["code"].is_string());
        assert!(
            finalized.details["interop_contract_rejection"]["rejected_diagnostic_count"]
                .is_number()
        );
        assert!(
            finalized.details["interop_contract_rejection"]["rejected_diagnostic_code_counts"]
                .is_object()
        );
    }
}

#[test]
fn error_finalization_classifies_every_rejected_contract_family() {
    let selector_parse = selector_parse_details(1, 1, "invalid_attribute_selector");
    let valid_selector_error =
        invalid_selector_interop_error(selector_parse.clone(), selector_parse)
            .with_computed_digest()
            .expect("valid selector error");

    let mut duplicate_diagnostic = valid_selector_error.clone();
    duplicate_diagnostic
        .diagnostics
        .push(duplicate_diagnostic.diagnostics[0].clone());
    let mut missing_core_diagnostic = valid_selector_error.clone();
    missing_core_diagnostic
        .details
        .remove("core_diagnostic_code");
    let invalid_plan_digest = InteropError::new(
        "not-a-sha256-digest",
        ErrorCode::InternalError,
        "Invalid plan digest.",
        None,
        BTreeMap::new(),
        Vec::new(),
    );

    for (rejected, expected_code) in [
        (
            duplicate_diagnostic,
            "invalid_selector_diagnostic_cardinality",
        ),
        (missing_core_diagnostic, "invalid_selector_core_diagnostic"),
        (invalid_plan_digest, "invalid_interop_contract"),
    ] {
        let finalized = v1::finalize_error_for_tests(rejected);
        assert_eq!(
            finalized.details["interop_contract_rejection"]["code"],
            expected_code
        );
        assert!(finalized.validate().is_ok());
    }
}

#[test]
fn error_finalization_has_a_valid_last_resort_when_the_sanitized_fallback_is_invalid() {
    let invalid_fallback = InteropError::new(
        "not-a-sha256-digest",
        ErrorCode::InternalError,
        "HTMLCut could not finalize its interop error payload.",
        None,
        BTreeMap::new(),
        Vec::new(),
    );

    let finalized = v1::finalize_sanitized_fallback_for_tests(invalid_fallback);
    assert_eq!(finalized.error_code, ErrorCode::InternalError);
    assert_eq!(
        finalized.details["interop_contract_rejection"]["code"],
        "fallback_finalization_failed"
    );
    assert!(finalized.validate().is_ok());
}
