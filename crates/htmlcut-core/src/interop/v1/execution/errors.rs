use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::{Diagnostic, DiagnosticCode};

use super::super::{
    ContractError, ErrorCode, InteropDiagnostic, InteropDiagnosticCode, InteropError, Plan,
    StrategyKind, is_valid_sha256_hex,
};

const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";

pub(super) fn plan_digest_error(plan: &Plan, error: ContractError) -> InteropError {
    let mut details = BTreeMap::new();
    details.insert("contract_error".to_owned(), Value::from(error.to_string()));
    finalize_error(InteropError::new(
        ZERO_SHA256,
        ErrorCode::InternalError,
        "HTMLCut could not compute the interop plan digest.",
        Some(plan.strategy.kind()),
        details,
        Vec::new(),
    ))
}

pub(super) fn plan_invalid_error(
    plan: &Plan,
    plan_digest_sha256: &str,
    error: ContractError,
) -> InteropError {
    let mut details = BTreeMap::new();
    details.insert("contract_error".to_owned(), Value::from(error.to_string()));
    finalize_error(InteropError::new(
        plan_digest_sha256.to_owned(),
        ErrorCode::PlanInvalid,
        error.to_string(),
        Some(plan.strategy.kind()),
        details,
        Vec::new(),
    ))
}

pub(super) fn core_execution_error(
    plan: &Plan,
    plan_digest_sha256: &str,
    candidate_count: usize,
    diagnostics: &[Diagnostic],
) -> InteropError {
    let Some(primary) = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.level == crate::DiagnosticLevel::Error)
    else {
        return internal_adapter_error(
            plan_digest_sha256,
            Some(plan.strategy.kind()),
            "execution failed without an error diagnostic",
            BTreeMap::new(),
            diagnostics,
        );
    };

    let error_code = match primary.code {
        DiagnosticCode::UnsupportedSpecVersion
        | DiagnosticCode::InvalidSelector
        | DiagnosticCode::InvalidSlicePattern => ErrorCode::PlanInvalid,
        DiagnosticCode::NoMatch | DiagnosticCode::MatchIndexOutOfRange => ErrorCode::NoMatch,
        DiagnosticCode::AmbiguousMatch => ErrorCode::AmbiguousMatch,
        DiagnosticCode::MissingAttribute => ErrorCode::MissingAttribute,
        _ => ErrorCode::InternalError,
    };
    let mut details = BTreeMap::new();
    details.insert(
        "core_diagnostic_code".to_owned(),
        Value::from(InteropDiagnosticCode::from(primary.code).as_str()),
    );
    let mut core_details = match primary.details.clone() {
        Some(Value::Object(details)) => details,
        Some(details) => {
            let mut wrapped = serde_json::Map::new();
            wrapped.insert("diagnostic_details".to_owned(), details);
            wrapped
        }
        None => serde_json::Map::new(),
    };
    core_details
        .entry("candidateCount".to_owned())
        .or_insert_with(|| Value::from(candidate_count));
    details.insert("core_details".to_owned(), Value::Object(core_details));

    finalize_error(InteropError::new(
        plan_digest_sha256.to_owned(),
        error_code,
        primary.message.clone(),
        Some(plan.strategy.kind()),
        details,
        diagnostics.iter().map(InteropDiagnostic::from).collect(),
    ))
}

pub(super) fn internal_adapter_error(
    plan_digest_sha256: &str,
    strategy_kind: Option<StrategyKind>,
    message: impl Into<String>,
    details: BTreeMap<String, Value>,
    diagnostics: &[Diagnostic],
) -> InteropError {
    finalize_error(InteropError::new(
        plan_digest_sha256.to_owned(),
        ErrorCode::InternalError,
        message,
        strategy_kind,
        details,
        diagnostics.iter().map(InteropDiagnostic::from).collect(),
    ))
}

pub(super) fn finalize_error(error: InteropError) -> InteropError {
    match error.clone().with_computed_digest() {
        Ok(error) => error,
        Err(contract_error) => sanitized_finalization_error(error, &contract_error),
    }
}

fn sanitized_finalization_error(
    error: InteropError,
    contract_error: &ContractError,
) -> InteropError {
    let mut diagnostic_code_counts = BTreeMap::<String, u64>::new();
    for diagnostic in &error.diagnostics {
        let count = diagnostic_code_counts
            .entry(diagnostic.code.as_str().to_owned())
            .or_default();
        *count = count.saturating_add(1);
    }
    let diagnostic_code_counts = diagnostic_code_counts
        .into_iter()
        .map(|(code, count)| (code, Value::from(count)))
        .collect::<serde_json::Map<_, _>>();
    let mut details = BTreeMap::new();
    details.insert(
        "interop_contract_rejection".to_owned(),
        json!({
            "code": finalization_rejection_code(contract_error),
            "rejected_diagnostic_count": error.diagnostics.len(),
            "rejected_diagnostic_code_counts": diagnostic_code_counts,
        }),
    );

    let fallback = InteropError::new(
        sanitized_plan_digest_sha256(&error.plan_digest_sha256),
        ErrorCode::InternalError,
        "HTMLCut could not finalize its interop error payload.",
        error.strategy_kind,
        details,
        Vec::new(),
    );

    finalize_sanitized_fallback(fallback)
}

fn finalize_sanitized_fallback(fallback: InteropError) -> InteropError {
    match fallback.clone().with_computed_digest() {
        Ok(fallback) => fallback,
        Err(_) => last_resort_finalization_error(),
    }
}

#[cfg(test)]
pub(super) fn finalize_sanitized_fallback_for_tests(fallback: InteropError) -> InteropError {
    finalize_sanitized_fallback(fallback)
}

fn finalization_rejection_code(error: &ContractError) -> &'static str {
    match error {
        ContractError::MessageTooLong { .. } => "message_too_long",
        ContractError::InvalidSelectorDiagnosticCardinality { .. } => {
            "invalid_selector_diagnostic_cardinality"
        }
        ContractError::InvalidSelectorCoreDiagnostic => "invalid_selector_core_diagnostic",
        ContractError::InvalidSelectorMessage { .. } => "invalid_selector_message",
        ContractError::MissingSelectorParseDetails { .. } => "missing_selector_parse_details",
        ContractError::MalformedSelectorParseDetails { .. } => "malformed_selector_parse_details",
        ContractError::NonObjectSelectorParseDetails { .. } => "non_object_selector_parse_details",
        ContractError::ZeroPositionSelectorParseDetails { .. } => {
            "zero_position_selector_parse_details"
        }
        ContractError::UnknownSelectorParseErrorClass { .. } => {
            "unknown_selector_parse_error_class"
        }
        ContractError::MismatchedSelectorParseDetails => "mismatched_selector_parse_details",
        _ => "invalid_interop_contract",
    }
}

fn last_resort_finalization_error() -> InteropError {
    let mut details = BTreeMap::new();
    details.insert(
        "interop_contract_rejection".to_owned(),
        json!({
            "code": "fallback_finalization_failed",
            "rejected_diagnostic_count": 0,
            "rejected_diagnostic_code_counts": {},
        }),
    );
    let fallback = InteropError::new(
        ZERO_SHA256,
        ErrorCode::InternalError,
        "HTMLCut could not finalize its interop error payload.",
        None,
        details,
        Vec::new(),
    );

    fallback.clone().with_computed_digest().unwrap_or(fallback)
}

fn sanitized_plan_digest_sha256(value: &str) -> String {
    if is_valid_sha256_hex(value) {
        value.to_owned()
    } else {
        ZERO_SHA256.to_owned()
    }
}
