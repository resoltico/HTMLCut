use super::*;

#[test]
fn interop_result_validation_covers_strategy_specific_payload_invariants() {
    let source = ResultSource {
        input_base_url: None,
        effective_base_url: None,
        document_title: None,
    };

    let mut selector_with_selected_html = InteropResult::new(
        ResultExecution::new(
            TEST_PLAN_DIGEST_SHA256,
            StrategyKind::CssSelector,
            SelectionMode::Single,
            Output::text(),
            1,
        ),
        source.clone(),
        selector_selected_matches(),
        Vec::new(),
    )
    .with_computed_digest()
    .expect("digest");
    selector_with_selected_html.selected_matches[0].selected_html_output =
        Some("<article>Hello</article>".to_owned());
    let selector_selected_html_error = selector_with_selected_html
        .validate()
        .expect_err("selector matches must not publish selected_html_output");
    assert!(matches!(
        selector_selected_html_error,
        ContractError::UnexpectedSelectedHtmlOutput
    ));

    let mut delimiter_without_selected_html = InteropResult::new(
        ResultExecution::new(
            TEST_PLAN_DIGEST_SHA256,
            StrategyKind::DelimiterPair,
            SelectionMode::Single,
            Output::selected_html(),
            1,
        ),
        source.clone(),
        vec![delimiter_selected_match_with(1, 1)],
        Vec::new(),
    )
    .with_computed_digest()
    .expect("digest");
    delimiter_without_selected_html.selected_matches[0].selected_html_output = None;
    let delimiter_selected_html_error = delimiter_without_selected_html
        .validate()
        .expect_err("delimiter matches require selected_html_output");
    assert!(matches!(
        delimiter_selected_html_error,
        ContractError::MissingSelectedHtmlOutput
    ));

    let mut delimiter_with_comparison_text = InteropResult::new(
        ResultExecution::new(
            TEST_PLAN_DIGEST_SHA256,
            StrategyKind::DelimiterPair,
            SelectionMode::Single,
            Output::selected_html(),
            1,
        ),
        source.clone(),
        vec![delimiter_selected_match_with(1, 1)],
        Vec::new(),
    )
    .with_computed_digest()
    .expect("digest");
    delimiter_with_comparison_text.selected_matches[0].comparison_text_output =
        Some("comparison text is CSS-only".to_owned());
    let delimiter_comparison_text_error = delimiter_with_comparison_text
        .validate()
        .expect_err("delimiter matches must not publish comparison_text_output");
    assert!(matches!(
        delimiter_comparison_text_error,
        ContractError::UnexpectedComparisonTextOutput
    ));

    let mut non_object_structured = InteropResult::new(
        ResultExecution::new(
            TEST_PLAN_DIGEST_SHA256,
            StrategyKind::CssSelector,
            SelectionMode::Single,
            Output::structured(),
            1,
        ),
        source.clone(),
        selector_selected_matches(),
        Vec::new(),
    );
    non_object_structured.selected_matches[0].output_value = json!("not an object");
    let structured_error = non_object_structured
        .validate()
        .expect_err("structured output must stay object-shaped");
    assert!(matches!(
        structured_error,
        ContractError::NonObjectStructuredOutputValue
    ));

    let mut non_string_text = InteropResult::new(
        ResultExecution::new(
            TEST_PLAN_DIGEST_SHA256,
            StrategyKind::CssSelector,
            SelectionMode::Single,
            Output::text(),
            1,
        ),
        source,
        selector_selected_matches(),
        Vec::new(),
    )
    .with_computed_digest()
    .expect("digest");
    non_string_text.selected_matches[0].output_value = json!({"text": "Hello"});
    let non_string_error = non_string_text
        .validate()
        .expect_err("non-structured output must stay string-shaped");
    assert!(matches!(
        non_string_error,
        ContractError::NonStringOutputValue {
            output_kind: OutputKind::Text
        }
    ));
}

#[test]
fn interop_validation_enforces_bounded_messages_and_closed_selector_parse_details() {
    let selector_parse = selector_parse_details(1, 1, "invalid_attribute_selector");
    let valid_error = invalid_selector_interop_error(selector_parse.clone(), selector_parse)
        .with_computed_digest()
        .expect("valid invalid-selector error");
    assert!(valid_error.validate().is_ok());

    let mut noncanonical_root_message = valid_error.clone();
    noncanonical_root_message.message = "Invalid selector: operator input".to_owned();
    assert!(matches!(
        noncanonical_root_message.digest_sha256(),
        Err(ContractError::InvalidSelectorMessage { carrier: "message" })
    ));

    let mut noncanonical_diagnostic_message = valid_error.clone();
    noncanonical_diagnostic_message.diagnostics[0].message =
        "Invalid selector: operator input".to_owned();
    assert!(matches!(
        noncanonical_diagnostic_message.digest_sha256(),
        Err(ContractError::InvalidSelectorMessage {
            carrier: "diagnostic.message"
        })
    ));

    let mut missing = valid_error.clone();
    missing.diagnostics[0].details = None;
    assert!(matches!(
        missing.digest_sha256(),
        Err(ContractError::MissingSelectorParseDetails {
            carrier: "diagnostic.details"
        })
    ));

    let mut malformed = valid_error.clone();
    malformed.diagnostics[0].details = Some(json!({"selector_parse": {"line": 1}}));
    assert!(matches!(
        malformed.digest_sha256(),
        Err(ContractError::MalformedSelectorParseDetails {
            carrier: "diagnostic.details"
        })
    ));

    let mut replaced_required_field = valid_error.clone();
    replaced_required_field.diagnostics[0].details = Some(json!({
        "selector_parse": {
            "line": 1,
            "column_utf16": 1,
            "unexpected": "invalid_attribute_selector",
        }
    }));
    assert!(matches!(
        replaced_required_field.digest_sha256(),
        Err(ContractError::MalformedSelectorParseDetails {
            carrier: "diagnostic.details"
        })
    ));

    let mut non_object = valid_error.clone();
    non_object.diagnostics[0].details = Some(json!({"selector_parse": "invalid"}));
    assert!(matches!(
        non_object.digest_sha256(),
        Err(ContractError::NonObjectSelectorParseDetails {
            carrier: "diagnostic.details"
        })
    ));

    let mut non_object_details_carrier = valid_error.clone();
    non_object_details_carrier.diagnostics[0].details = Some(json!(false));
    assert!(matches!(
        non_object_details_carrier.digest_sha256(),
        Err(ContractError::MalformedSelectorParseDetails {
            carrier: "diagnostic.details"
        })
    ));

    let mut non_integer_position = valid_error.clone();
    non_integer_position.diagnostics[0].details = Some(json!({
        "selector_parse": {
            "line": "one",
            "column_utf16": 1,
            "parse_error_class": "invalid_attribute_selector",
        }
    }));
    assert!(matches!(
        non_integer_position.digest_sha256(),
        Err(ContractError::MalformedSelectorParseDetails {
            carrier: "diagnostic.details"
        })
    ));

    let mut non_string_class = valid_error.clone();
    non_string_class.diagnostics[0].details = Some(json!({
        "selector_parse": {
            "line": 1,
            "column_utf16": 1,
            "parse_error_class": false,
        }
    }));
    assert!(matches!(
        non_string_class.digest_sha256(),
        Err(ContractError::MalformedSelectorParseDetails {
            carrier: "diagnostic.details"
        })
    ));

    let mut zero_line = valid_error.clone();
    zero_line.diagnostics[0].details =
        Some(selector_parse_details(0, 1, "invalid_attribute_selector"));
    assert!(matches!(
        zero_line.digest_sha256(),
        Err(ContractError::ZeroPositionSelectorParseDetails {
            carrier: "diagnostic.details"
        })
    ));

    let mut zero_column = valid_error.clone();
    zero_column.diagnostics[0].details =
        Some(selector_parse_details(1, 0, "invalid_attribute_selector"));
    assert!(matches!(
        zero_column.digest_sha256(),
        Err(ContractError::ZeroPositionSelectorParseDetails {
            carrier: "diagnostic.details"
        })
    ));

    let mut unknown_class = valid_error.clone();
    unknown_class.diagnostics[0].details = Some(selector_parse_details(1, 1, "not_a_class"));
    assert!(matches!(
        unknown_class.digest_sha256(),
        Err(ContractError::UnknownSelectorParseErrorClass {
            carrier: "diagnostic.details"
        })
    ));

    let mut mismatch = valid_error.clone();
    mismatch.details.insert(
        "core_details".to_owned(),
        selector_parse_details(2, 1, "invalid_attribute_selector"),
    );
    assert!(matches!(
        mismatch.digest_sha256(),
        Err(ContractError::MismatchedSelectorParseDetails)
    ));

    let mut duplicate_diagnostic = valid_error.clone();
    duplicate_diagnostic
        .diagnostics
        .push(duplicate_diagnostic.diagnostics[0].clone());
    assert!(matches!(
        duplicate_diagnostic.digest_sha256(),
        Err(ContractError::InvalidSelectorDiagnosticCardinality { received: 2 })
    ));

    let mut no_matching_diagnostic = valid_error.clone();
    no_matching_diagnostic.diagnostics.clear();
    assert!(matches!(
        no_matching_diagnostic.digest_sha256(),
        Err(ContractError::InvalidSelectorDiagnosticCardinality { received: 0 })
    ));

    let mut missing_core_diagnostic = valid_error.clone();
    missing_core_diagnostic
        .details
        .remove("core_diagnostic_code");
    assert!(matches!(
        missing_core_diagnostic.digest_sha256(),
        Err(ContractError::InvalidSelectorCoreDiagnostic)
    ));

    let mut missing_core_details = valid_error.clone();
    missing_core_details.details.remove("core_details");
    assert!(matches!(
        missing_core_details.digest_sha256(),
        Err(ContractError::MissingSelectorParseDetails {
            carrier: "details.core_details"
        })
    ));

    let mut malformed_core_details = valid_error.clone();
    malformed_core_details.details.insert(
        "core_details".to_owned(),
        json!({"selector_parse": {"line": 1}}),
    );
    assert!(matches!(
        malformed_core_details.digest_sha256(),
        Err(ContractError::MalformedSelectorParseDetails {
            carrier: "details.core_details"
        })
    ));

    let oversized = "x".repeat(1025);
    let root_message_error = InteropError::new(
        TEST_PLAN_DIGEST_SHA256,
        ErrorCode::InternalError,
        oversized.clone(),
        None,
        BTreeMap::new(),
        Vec::new(),
    );
    assert!(matches!(
        root_message_error.digest_sha256(),
        Err(ContractError::MessageTooLong {
            field: "message",
            ..
        })
    ));

    let diagnostic_message_error = InteropError::new(
        TEST_PLAN_DIGEST_SHA256,
        ErrorCode::NoMatch,
        "No matches were found.",
        None,
        BTreeMap::new(),
        vec![InteropDiagnostic {
            level: InteropDiagnosticLevel::Error,
            code: InteropDiagnosticCode::NoMatch,
            message: oversized.clone(),
            details: None,
        }],
    );
    assert!(matches!(
        diagnostic_message_error.digest_sha256(),
        Err(ContractError::MessageTooLong {
            field: "diagnostic.message",
            ..
        })
    ));

    let result = InteropResult::new(
        ResultExecution::new(
            TEST_PLAN_DIGEST_SHA256,
            StrategyKind::CssSelector,
            SelectionMode::Single,
            Output::text(),
            1,
        ),
        ResultSource {
            input_base_url: None,
            effective_base_url: None,
            document_title: None,
        },
        selector_selected_matches(),
        vec![InteropDiagnostic {
            level: InteropDiagnosticLevel::Warning,
            code: InteropDiagnosticCode::EffectiveBaseUrlUnresolved,
            message: oversized,
            details: None,
        }],
    );
    assert!(matches!(
        result.digest_sha256(),
        Err(ContractError::MessageTooLong {
            field: "diagnostic.message",
            ..
        })
    ));

    let within_byte_limit = "€".repeat(341);
    let within_byte_limit_error = InteropError::new(
        TEST_PLAN_DIGEST_SHA256,
        ErrorCode::InternalError,
        within_byte_limit,
        None,
        BTreeMap::new(),
        Vec::new(),
    );
    assert!(within_byte_limit_error.digest_sha256().is_ok());

    let over_byte_limit = "€".repeat(342);
    let over_byte_limit_diagnostic = InteropError::new(
        TEST_PLAN_DIGEST_SHA256,
        ErrorCode::NoMatch,
        "No matches were found.",
        None,
        BTreeMap::new(),
        vec![InteropDiagnostic {
            level: InteropDiagnosticLevel::Error,
            code: InteropDiagnosticCode::NoMatch,
            message: over_byte_limit,
            details: None,
        }],
    );
    assert!(matches!(
        over_byte_limit_diagnostic.digest_sha256(),
        Err(ContractError::MessageTooLong {
            field: "diagnostic.message",
            received: 1026,
            ..
        })
    ));
}
