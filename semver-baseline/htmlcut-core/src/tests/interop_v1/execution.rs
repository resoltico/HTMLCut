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
        selector_request.normalization.whitespace,
        WhitespaceMode::Normalize
    );
    assert!(!selector_request.normalization.rewrite_urls);
    let first_request = v1::compile_request_for_tests(
        &selector_source,
        &Plan::new(
            PlanStrategy::css_selector(selector_query("article")),
            Selection::first(),
            Output::new(OutputKind::Text),
            Normalization::new(TextWhitespace::Normalize, false),
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
        delimiter_request.normalization.whitespace,
        WhitespaceMode::Preserve
    );
    assert!(delimiter_request.normalization.rewrite_urls);
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
            "html": "<article>Hello</article>",
            "text": "Hello",
            "outerHtml": "<article>Hello</article>"
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
            "html": "<article>Hello</article>",
            "text": "Hello",
            "innerHtml": "Hello",
            "outerHtml": "<article>Hello</article>"
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
            .with_input_base_url(Url::parse("https://example.com/start.html").expect("url")),
        &selector_plan(),
        successful_selector_extraction(
            vec![selector_core_match(1, 1, 1)],
            1,
            Some("https://example.com/base.html"),
        ),
    )
    .expect("adapted text result");
    assert_eq!(adapted_text.selected_matches.len(), 1);
    assert_eq!(
        adapted_text.selected_matches[0].value_kind,
        OutputKind::Text
    );
    assert_eq!(adapted_text.selected_matches[0].value, "Hello");
    assert_eq!(
        adapted_text
            .source
            .effective_base_url
            .as_ref()
            .map(Url::as_str),
        Some("https://example.com/base.html")
    );

    let adapted_inner = v1::adapt_successful_extraction_for_tests(
        &selector_source,
        &Plan::new(
            PlanStrategy::css_selector(selector_query("article")),
            Selection::single(),
            Output::new(OutputKind::InnerHtml),
            Normalization::new(TextWhitespace::Normalize, false),
        ),
        successful_selector_extraction(vec![selector_core_match(1, 1, 1)], 1, None),
    )
    .expect("adapted inner-html result");
    assert!(adapted_inner.selected_matches[0].value.contains("<article"));

    let adapted_outer = v1::adapt_successful_extraction_for_tests(
        &selector_source,
        &Plan::new(
            PlanStrategy::css_selector(selector_query("article")),
            Selection::single(),
            Output::new(OutputKind::OuterHtml),
            Normalization::new(TextWhitespace::Normalize, false),
        ),
        successful_selector_extraction(vec![selector_core_match(1, 1, 1)], 1, None),
    )
    .expect("adapted outer-html result");
    assert!(adapted_outer.selected_matches[0].value.contains("<article"));

    let adapted_all = v1::adapt_successful_extraction_for_tests(
        &selector_source,
        &Plan::new(
            PlanStrategy::css_selector(selector_query("article")),
            Selection::all(),
            Output::new(OutputKind::Text),
            Normalization::new(TextWhitespace::Normalize, false),
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
                value: json!({"text": "Hello", "outerHtml": "<article>Hello</article>"}),
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
    assert!(adapted_projection_error.message.contains("\"html\""));

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
            value: json!({"html": "<article>Hello</article>", "text": "Hello"}),
            ..selector_match.clone()
        },
        StrategyKind::CssSelector,
        &[],
    )
    .expect_err("missing outerHtml");
    assert_eq!(missing_field_error.error_code, ErrorCode::InternalError);
    assert!(missing_field_error.message.contains("\"outerHtml\""));

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
                "html": "<article>Hello</article>",
                "outerHtml": "<article>Hello</article>"
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
    assert!(selector_missing_text_error.message.contains("\"text\""));

    let delimiter_missing_html_error = v1::project_structured_match_for_tests(
        &ExtractionMatch {
            value: json!({
                "text": "Hello",
                "innerHtml": "Hello",
                "outerHtml": "<article>Hello</article>"
            }),
            ..delimiter_match.clone()
        },
        StrategyKind::DelimiterPair,
        &[],
    )
    .expect_err("missing delimiter html");
    assert_eq!(
        delimiter_missing_html_error.error_code,
        ErrorCode::InternalError
    );
    assert!(delimiter_missing_html_error.message.contains("\"html\""));

    let delimiter_missing_text_error = v1::project_structured_match_for_tests(
        &ExtractionMatch {
            value: json!({
                "html": "<article>Hello</article>",
                "innerHtml": "Hello",
                "outerHtml": "<article>Hello</article>"
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
    assert!(delimiter_missing_text_error.message.contains("\"text\""));

    let delimiter_missing_inner_html_error = v1::project_structured_match_for_tests(
        &ExtractionMatch {
            value: json!({
                "html": "<article>Hello</article>",
                "text": "Hello",
                "outerHtml": "<article>Hello</article>"
            }),
            ..delimiter_match.clone()
        },
        StrategyKind::DelimiterPair,
        &[],
    )
    .expect_err("missing delimiter innerHtml");
    assert_eq!(
        delimiter_missing_inner_html_error.error_code,
        ErrorCode::InternalError
    );
    assert!(
        delimiter_missing_inner_html_error
            .message
            .contains("\"innerHtml\"")
    );

    let delimiter_missing_outer_html_error = v1::project_structured_match_for_tests(
        &ExtractionMatch {
            value: json!({
                "html": "<article>Hello</article>",
                "text": "Hello",
                "innerHtml": "Hello"
            }),
            ..delimiter_match.clone()
        },
        StrategyKind::DelimiterPair,
        &[],
    )
    .expect_err("missing delimiter outerHtml");
    assert_eq!(
        delimiter_missing_outer_html_error.error_code,
        ErrorCode::InternalError
    );
    assert!(
        delimiter_missing_outer_html_error
            .message
            .contains("\"outerHtml\"")
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

    let invalid_request_error = v1::core_execution_error_for_tests(
        &selector_plan(),
        &[Diagnostic {
            level: DiagnosticLevel::Error,
            code: DiagnosticCode::InvalidSelector,
            message: "invalid".to_owned(),
            details: None,
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
        fallback_error.error_digest_sha256,
        "0000000000000000000000000000000000000000000000000000000000000000"
    );
    assert!(
        fallback_error
            .message
            .contains("could not finalize its interop error payload")
    );

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

    let _typed: Box<InteropError> = Box::new(adapter_error);
}
