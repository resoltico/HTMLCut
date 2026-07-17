use super::*;

#[test]
fn execution_errors_map_to_closed_interop_error_documents() {
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

    let _typed: Box<InteropError> = Box::new(adapter_error);
}
