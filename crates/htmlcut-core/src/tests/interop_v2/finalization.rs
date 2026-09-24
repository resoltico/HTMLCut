use super::*;

const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";

fn assert_finalized_rejection(
    rejected: InteropError,
    expected_rejection: InteropFinalizationRejection,
) -> InteropError {
    let finalized = v2::finalize_error_for_tests(rejected);

    assert_eq!(finalized.error_code, ErrorCode::InternalError);
    assert_eq!(finalized.strategy_kind, Some(StrategyKind::CssSelector));
    assert!(finalized.diagnostics.is_empty());
    assert_eq!(
        finalized.detail,
        InteropErrorDetail::FinalizationRejected {
            rejection: expected_rejection,
        }
    );
    assert!(finalized.validate().is_ok());

    finalized
}

fn valid_selector_parse() -> SelectorParseDetail {
    selector_parse_detail(1, 1, SelectorParseErrorClass::InvalidAttributeSelector)
}

fn valid_invalid_selector_error() -> InteropError {
    let selector_parse = valid_selector_parse();
    invalid_selector_interop_error(
        selector_parse_details(1, 1, "invalid_attribute_selector"),
        selector_parse,
    )
}

#[test]
fn finalization_uses_closed_rejection_evidence_for_oversized_messages() {
    assert_finalized_rejection(
        InteropError::new(
            TEST_PLAN_DIGEST_SHA256,
            ErrorCode::InternalError,
            "x".repeat(1025),
            Some(StrategyKind::CssSelector),
            InteropErrorDetail::AdapterInvariant {
                failure: InteropAdapterFailure::ResultFinalization,
            },
            Vec::new(),
        ),
        InteropFinalizationRejection::MessageTooLong,
    );

    assert_finalized_rejection(
        InteropError::new(
            TEST_PLAN_DIGEST_SHA256,
            ErrorCode::NoMatch,
            "No matches were found.",
            Some(StrategyKind::CssSelector),
            InteropErrorDetail::CoreExecution {
                core_diagnostic_code: InteropDiagnosticCode::NoMatch,
                candidate_count: 0,
            },
            vec![InteropDiagnostic {
                level: InteropDiagnosticLevel::Error,
                code: InteropDiagnosticCode::NoMatch,
                message: "x".repeat(1025),
                details: None,
            }],
        ),
        InteropFinalizationRejection::MessageTooLong,
    );
}

#[test]
fn finalization_rejects_invalid_selector_payloads_without_generic_json_fallbacks() {
    let mut noncanonical_message = valid_invalid_selector_error();
    noncanonical_message.message = "Invalid selector: operator input".to_owned();
    assert_finalized_rejection(
        noncanonical_message,
        InteropFinalizationRejection::InvalidSelectorDiagnostic,
    );

    let mut malformed_diagnostic = valid_invalid_selector_error();
    malformed_diagnostic.diagnostics[0].details = Some(json!({"selector_parse": {"line": 1}}));
    assert_finalized_rejection(
        malformed_diagnostic,
        InteropFinalizationRejection::InvalidSelectorDiagnostic,
    );

    let mut mismatched_detail = valid_invalid_selector_error();
    mismatched_detail.detail = InteropErrorDetail::InvalidSelector {
        candidate_count: 0,
        selector_parse: selector_parse_detail(
            2,
            1,
            SelectorParseErrorClass::InvalidAttributeSelector,
        ),
    };
    assert_finalized_rejection(
        mismatched_detail,
        InteropFinalizationRejection::InvalidSelectorDiagnostic,
    );

    let mut duplicate_diagnostic = valid_invalid_selector_error();
    duplicate_diagnostic
        .diagnostics
        .push(duplicate_diagnostic.diagnostics[0].clone());
    assert_finalized_rejection(
        duplicate_diagnostic,
        InteropFinalizationRejection::InvalidSelectorDiagnostic,
    );
}

#[test]
fn finalization_replaces_an_invalid_plan_digest_with_the_closed_contract_rejection() {
    let finalized = assert_finalized_rejection(
        InteropError::new(
            TEST_PLAN_DIGEST_SHA256.to_uppercase(),
            ErrorCode::NoMatch,
            "No matches were found.",
            Some(StrategyKind::CssSelector),
            InteropErrorDetail::CoreExecution {
                core_diagnostic_code: InteropDiagnosticCode::NoMatch,
                candidate_count: 0,
            },
            vec![InteropDiagnostic {
                level: InteropDiagnosticLevel::Error,
                code: InteropDiagnosticCode::NoMatch,
                message: "No matches were found.".to_owned(),
                details: None,
            }],
        ),
        InteropFinalizationRejection::InvalidInteropContract,
    );

    assert_eq!(finalized.plan_digest_sha256, ZERO_SHA256);
}

#[test]
fn finalization_has_a_valid_last_resort_when_the_sanitized_fallback_is_invalid() {
    let invalid_fallback = InteropError::new(
        "not-a-sha256-digest",
        ErrorCode::InternalError,
        "HTMLCut could not finalize its interop error payload.",
        None,
        InteropErrorDetail::AdapterInvariant {
            failure: InteropAdapterFailure::ResultFinalization,
        },
        Vec::new(),
    );

    let finalized = v2::finalize_sanitized_fallback_for_tests(invalid_fallback);
    assert_eq!(finalized.error_code, ErrorCode::InternalError);
    assert_eq!(
        finalized.detail,
        InteropErrorDetail::FinalizationRejected {
            rejection: InteropFinalizationRejection::FallbackFinalizationFailed,
        }
    );
    assert!(finalized.validate().is_ok());
}
