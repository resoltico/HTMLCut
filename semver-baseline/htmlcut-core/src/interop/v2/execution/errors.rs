use crate::{Diagnostic, DiagnosticCode};

use super::super::{
    ContractError, ErrorCode, InteropAdapterFailure, InteropDiagnostic, InteropDiagnosticCode,
    InteropError, InteropErrorDetail, InteropFinalizationRejection, Plan, StrategyKind,
    is_valid_sha256_hex,
};
use crate::selector_parse::validate_selector_parse_details;

const ZERO_SHA256: &str = "0000000000000000000000000000000000000000000000000000000000000000";

pub(super) fn plan_digest_error(plan: &Plan, _error: ContractError) -> InteropError {
    finalize_error(InteropError::new(
        ZERO_SHA256,
        ErrorCode::InternalError,
        "HTMLCut could not compute the interop plan digest.",
        Some(plan.strategy.kind()),
        InteropErrorDetail::ContractViolation,
        Vec::new(),
    ))
}

pub(super) fn plan_invalid_error(
    plan: &Plan,
    plan_digest_sha256: &str,
    error: ContractError,
) -> InteropError {
    finalize_error(InteropError::new(
        plan_digest_sha256.to_owned(),
        ErrorCode::PlanInvalid,
        error.to_string(),
        Some(plan.strategy.kind()),
        InteropErrorDetail::ContractViolation,
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
            InteropAdapterFailure::MissingExecutionErrorDiagnostic,
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
        DiagnosticCode::SelectorWorkLimitExceeded
        | DiagnosticCode::CandidateLimitExceeded
        | DiagnosticCode::SelectedMatchLimitExceeded
        | DiagnosticCode::OutputLimitExceeded => ErrorCode::ExecutionLimitExceeded,
        _ => ErrorCode::InternalError,
    };
    let diagnostics = diagnostics
        .iter()
        .map(InteropDiagnostic::from)
        .collect::<Vec<_>>();
    let candidate_count = u32::try_from(candidate_count)
        .expect("core execution bounds candidate counts to the v2 fixed-width range");
    let detail = if primary.code == DiagnosticCode::InvalidSelector {
        primary
            .details
            .as_ref()
            .and_then(|details| validate_selector_parse_details(details).ok())
            .map(|selector_parse| InteropErrorDetail::InvalidSelector {
                candidate_count,
                selector_parse,
            })
            .unwrap_or(InteropErrorDetail::AdapterInvariant {
                failure: InteropAdapterFailure::InvalidStructuredProjection,
            })
    } else {
        InteropErrorDetail::CoreExecution {
            core_diagnostic_code: InteropDiagnosticCode::from(primary.code),
            candidate_count,
        }
    };

    finalize_error(InteropError::new(
        plan_digest_sha256.to_owned(),
        error_code,
        primary.message.clone(),
        Some(plan.strategy.kind()),
        detail,
        diagnostics,
    ))
}

pub(super) fn internal_adapter_error(
    plan_digest_sha256: &str,
    strategy_kind: Option<StrategyKind>,
    message: impl Into<String>,
    failure: InteropAdapterFailure,
    diagnostics: &[Diagnostic],
) -> InteropError {
    finalize_error(InteropError::new(
        plan_digest_sha256.to_owned(),
        ErrorCode::InternalError,
        message,
        strategy_kind,
        InteropErrorDetail::AdapterInvariant { failure },
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
    let fallback = InteropError::new(
        sanitized_plan_digest_sha256(&error.plan_digest_sha256),
        ErrorCode::InternalError,
        "HTMLCut could not finalize its interop error payload.",
        error.strategy_kind,
        InteropErrorDetail::FinalizationRejected {
            rejection: finalization_rejection(contract_error),
        },
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

fn finalization_rejection(error: &ContractError) -> InteropFinalizationRejection {
    match error {
        ContractError::MessageTooLong { .. } => InteropFinalizationRejection::MessageTooLong,
        ContractError::InvalidSelectorDiagnosticCardinality { .. } => {
            InteropFinalizationRejection::InvalidSelectorDiagnostic
        }
        ContractError::InvalidSelectorCoreDiagnostic
        | ContractError::InvalidSelectorMessage { .. }
        | ContractError::MissingSelectorParseDetails { .. }
        | ContractError::MalformedSelectorParseDetails { .. }
        | ContractError::NonObjectSelectorParseDetails { .. }
        | ContractError::ZeroPositionSelectorParseDetails { .. }
        | ContractError::UnknownSelectorParseErrorClass { .. }
        | ContractError::MismatchedSelectorParseDetails => {
            InteropFinalizationRejection::InvalidSelectorDiagnostic
        }
        _ => InteropFinalizationRejection::InvalidInteropContract,
    }
}

fn last_resort_finalization_error() -> InteropError {
    let fallback = InteropError::new(
        ZERO_SHA256,
        ErrorCode::InternalError,
        "HTMLCut could not finalize its interop error payload.",
        None,
        InteropErrorDetail::FinalizationRejected {
            rejection: InteropFinalizationRejection::FallbackFinalizationFailed,
        },
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
