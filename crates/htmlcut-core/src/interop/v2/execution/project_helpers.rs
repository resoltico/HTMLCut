//! Shared validation and finalization helpers for interop result projection.

use crate::{Diagnostic, DisplayedHttpUrl};

use super::super::{InteropAdapterFailure, InteropError, InteropResult, StrategyKind};
use super::errors::internal_adapter_error;

pub(super) fn parse_optional_url(
    value: Option<&str>,
    plan_digest_sha256: &str,
    strategy_kind: StrategyKind,
    field: &'static str,
    diagnostics: &[Diagnostic],
) -> Result<Option<DisplayedHttpUrl>, Box<InteropError>> {
    value
        .map(|value| {
            DisplayedHttpUrl::parse(value).map_err(|_| {
                Box::new(internal_adapter_error(
                    plan_digest_sha256,
                    Some(strategy_kind),
                    format!("execution produced an invalid URL in {field}"),
                    InteropAdapterFailure::InvalidSourceUrl,
                    diagnostics,
                ))
            })
        })
        .transpose()
}

pub(super) fn finalize_interop_result(
    result: InteropResult,
    plan_digest_sha256: &str,
    strategy_kind: StrategyKind,
    diagnostics: &[Diagnostic],
) -> Result<InteropResult, Box<InteropError>> {
    result.with_computed_digest().map_err(|_| {
        Box::new(internal_adapter_error(
            plan_digest_sha256,
            Some(strategy_kind),
            "execution produced an invalid interop result during finalization",
            InteropAdapterFailure::ResultFinalization,
            diagnostics,
        ))
    })
}
