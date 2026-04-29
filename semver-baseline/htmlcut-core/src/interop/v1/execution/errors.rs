use std::collections::BTreeMap;

use serde_json::Value;

use crate::{Diagnostic, DiagnosticCode};

use super::super::{ContractError, ErrorCode, InteropError, Plan, StrategyKind};

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

    let error_code = match primary.code {
        DiagnosticCode::UnsupportedSpecVersion
        | DiagnosticCode::InvalidSelector
        | DiagnosticCode::InvalidSlicePattern => ErrorCode::PlanInvalid,
        DiagnosticCode::NoMatch | DiagnosticCode::MatchIndexOutOfRange => ErrorCode::NoMatch,
        DiagnosticCode::AmbiguousMatch => ErrorCode::AmbiguousMatch,
        _ => ErrorCode::InternalError,
    };
    let mut details = BTreeMap::new();
    details.insert(
        "core_diagnostic_code".to_owned(),
        Value::from(primary.code.as_str()),
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
    let plan_digest_sha256 = error.plan_digest_sha256.clone();
    let strategy_kind = error.strategy_kind;
    let diagnostics = error.diagnostics.clone();

    match error.with_computed_digest() {
        Ok(error) => error,
        Err(contract_error) => {
            let mut details = BTreeMap::new();
            details.insert(
                "contract_error".to_owned(),
                Value::from(contract_error.to_string()),
            );

            let fallback = InteropError::new(
                plan_digest_sha256,
                ErrorCode::InternalError,
                "HTMLCut could not finalize its interop error payload.",
                strategy_kind,
                details,
                diagnostics,
            );

            match fallback.clone().with_computed_digest() {
                Ok(fallback) => fallback,
                Err(_) => InteropError {
                    error_digest_sha256: ZERO_SHA256.to_owned(),
                    ..fallback
                },
            }
        }
    }
}
