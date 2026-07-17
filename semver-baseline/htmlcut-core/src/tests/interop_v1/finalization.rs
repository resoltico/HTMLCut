use super::*;
use serde_json::{Value, json};

const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";

fn assert_finalized_rejection(
    rejected: InteropError,
    expected_code: &str,
    expected_diagnostic_count: usize,
    expected_diagnostic_code_counts: Value,
) -> InteropError {
    let finalized = v1::finalize_error_for_tests(rejected);

    assert_eq!(finalized.error_code, ErrorCode::InternalError);
    assert_eq!(finalized.strategy_kind, Some(StrategyKind::CssSelector));
    assert!(finalized.diagnostics.is_empty());
    assert_eq!(
        finalized.details["interop_contract_rejection"],
        json!({
            "code": expected_code,
            "rejected_diagnostic_count": expected_diagnostic_count,
            "rejected_diagnostic_code_counts": expected_diagnostic_code_counts,
        })
    );
    assert!(finalized.validate().is_ok());

    finalized
}

fn valid_selector_parse() -> Value {
    selector_parse_details(1, 1, "invalid_attribute_selector")
}

#[test]
fn finalization_summarizes_an_oversized_root_message() {
    assert_finalized_rejection(
        InteropError::new(
            TEST_PLAN_DIGEST_SHA256,
            ErrorCode::InternalError,
            "x".repeat(1025),
            Some(StrategyKind::CssSelector),
            BTreeMap::new(),
            Vec::new(),
        ),
        "message_too_long",
        0,
        json!({}),
    );
}

#[test]
fn finalization_summarizes_an_oversized_diagnostic_message() {
    assert_finalized_rejection(
        InteropError::new(
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
        ),
        "message_too_long",
        1,
        json!({"NO_MATCH": 1}),
    );
}

#[test]
fn finalization_summarizes_a_noncanonical_invalid_selector_message() {
    let selector_parse = valid_selector_parse();
    let mut rejected = invalid_selector_interop_error(selector_parse.clone(), selector_parse);
    rejected.message = "Invalid selector: operator input".to_owned();

    assert_finalized_rejection(
        rejected,
        "invalid_selector_message",
        1,
        json!({"INVALID_SELECTOR": 1}),
    );
}

#[test]
fn finalization_summarizes_missing_selector_parse_details() {
    assert_finalized_rejection(
        invalid_selector_interop_error(json!({}), valid_selector_parse()),
        "missing_selector_parse_details",
        1,
        json!({"INVALID_SELECTOR": 1}),
    );
}

#[test]
fn finalization_summarizes_malformed_selector_parse_details() {
    assert_finalized_rejection(
        invalid_selector_interop_error(
            json!({"selector_parse": {"line": 1}}),
            valid_selector_parse(),
        ),
        "malformed_selector_parse_details",
        1,
        json!({"INVALID_SELECTOR": 1}),
    );
}

#[test]
fn finalization_summarizes_non_object_selector_parse_details() {
    assert_finalized_rejection(
        invalid_selector_interop_error(json!({"selector_parse": false}), valid_selector_parse()),
        "non_object_selector_parse_details",
        1,
        json!({"INVALID_SELECTOR": 1}),
    );
}

#[test]
fn finalization_summarizes_a_zero_line_selector_parse_position() {
    assert_finalized_rejection(
        invalid_selector_interop_error(
            selector_parse_details(0, 1, "invalid_attribute_selector"),
            valid_selector_parse(),
        ),
        "zero_position_selector_parse_details",
        1,
        json!({"INVALID_SELECTOR": 1}),
    );
}

#[test]
fn finalization_summarizes_a_zero_column_selector_parse_position() {
    assert_finalized_rejection(
        invalid_selector_interop_error(
            selector_parse_details(1, 0, "invalid_attribute_selector"),
            valid_selector_parse(),
        ),
        "zero_position_selector_parse_details",
        1,
        json!({"INVALID_SELECTOR": 1}),
    );
}

#[test]
fn finalization_summarizes_an_unknown_selector_parse_class() {
    assert_finalized_rejection(
        invalid_selector_interop_error(
            selector_parse_details(1, 1, "not_a_class"),
            valid_selector_parse(),
        ),
        "unknown_selector_parse_error_class",
        1,
        json!({"INVALID_SELECTOR": 1}),
    );
}

#[test]
fn finalization_summarizes_mismatched_selector_parse_details() {
    assert_finalized_rejection(
        invalid_selector_interop_error(
            valid_selector_parse(),
            selector_parse_details(2, 1, "invalid_attribute_selector"),
        ),
        "mismatched_selector_parse_details",
        1,
        json!({"INVALID_SELECTOR": 1}),
    );
}

#[test]
fn finalization_summarizes_ambiguous_invalid_selector_diagnostics() {
    let selector_parse = valid_selector_parse();
    let mut rejected = invalid_selector_interop_error(selector_parse.clone(), selector_parse)
        .with_computed_digest()
        .expect("valid selector error");
    rejected.diagnostics.push(rejected.diagnostics[0].clone());

    assert_finalized_rejection(
        rejected,
        "invalid_selector_diagnostic_cardinality",
        2,
        json!({"INVALID_SELECTOR": 2}),
    );
}

#[test]
fn finalization_summarizes_a_missing_core_invalid_selector_diagnostic() {
    let selector_parse = valid_selector_parse();
    let mut rejected = invalid_selector_interop_error(selector_parse.clone(), selector_parse)
        .with_computed_digest()
        .expect("valid selector error");
    rejected.details.remove("core_diagnostic_code");

    assert_finalized_rejection(
        rejected,
        "invalid_selector_core_diagnostic",
        1,
        json!({"INVALID_SELECTOR": 1}),
    );
}

#[test]
fn finalization_replaces_an_uppercase_plan_digest_without_losing_evidence() {
    let finalized = assert_finalized_rejection(
        InteropError::new(
            TEST_PLAN_DIGEST_SHA256.to_uppercase(),
            ErrorCode::NoMatch,
            "No matches were found.",
            Some(StrategyKind::CssSelector),
            BTreeMap::new(),
            vec![InteropDiagnostic {
                level: InteropDiagnosticLevel::Error,
                code: InteropDiagnosticCode::NoMatch,
                message: "No matches were found.".to_owned(),
                details: None,
            }],
        ),
        "invalid_interop_contract",
        1,
        json!({"NO_MATCH": 1}),
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
