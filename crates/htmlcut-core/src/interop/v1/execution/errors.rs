use std::collections::BTreeMap;

use serde_json::Value;

use crate::{Diagnostic, DiagnosticCode};

use super::super::{ContractError, ErrorCode, InteropError, Plan, StrategyKind};

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
            diagnostics.to_vec(),
        );
    };

    let error_code = match primary.code.parse::<DiagnosticCode>() {
        Ok(
            DiagnosticCode::UnsupportedSpecVersion
            | DiagnosticCode::InvalidSelector
            | DiagnosticCode::InvalidSlicePattern
            | DiagnosticCode::InvalidRequest,
        ) => ErrorCode::PlanInvalid,
        Ok(DiagnosticCode::NoMatch | DiagnosticCode::MatchIndexOutOfRange) => ErrorCode::NoMatch,
        Ok(DiagnosticCode::AmbiguousMatch) => ErrorCode::AmbiguousMatch,
        _ => ErrorCode::InternalError,
    };
    let mut details = BTreeMap::new();
    details.insert(
        "core_diagnostic_code".to_owned(),
        Value::from(primary.code.clone()),
    );
    if let Some(core_details) = &primary.details {
        details.insert("core_details".to_owned(), core_details.clone());
    }

    finalize_error(InteropError::new(
        plan_digest_sha256.to_owned(),
        error_code,
        primary.message.clone(),
        Some(plan.strategy.kind()),
        details,
        diagnostics.to_vec(),
    ))
}

pub(super) fn internal_adapter_error(
    plan_digest_sha256: &str,
    strategy_kind: Option<StrategyKind>,
    message: impl Into<String>,
    details: BTreeMap<String, Value>,
    diagnostics: Vec<Diagnostic>,
) -> InteropError {
    finalize_error(InteropError::new(
        plan_digest_sha256.to_owned(),
        ErrorCode::InternalError,
        message,
        strategy_kind,
        details,
        diagnostics,
    ))
}

pub(super) fn finalize_error(error: InteropError) -> InteropError {
    error
        .with_computed_digest()
        .expect("errors should always validate and serialize")
}
