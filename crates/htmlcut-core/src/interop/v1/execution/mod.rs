mod compile;
mod errors;
mod project;

use compile::{compile_request, exact_plan_digest_sha256, runtime_options};
use errors::{core_execution_error, plan_invalid_error};
use project::adapt_successful_extraction;

use crate::extract;

use super::{HtmlInput, InteropError, InteropResult, Plan};

/// Validates one plan and returns a typed interop error on failure.
pub fn validate_plan(plan: &Plan) -> Result<(), Box<InteropError>> {
    let plan_digest_sha256 = exact_plan_digest_sha256(plan);
    plan.validate()
        .map_err(|error| Box::new(plan_invalid_error(plan, &plan_digest_sha256, error)))
}

/// Executes one plan directly against in-memory HTML input.
pub fn execute_plan(source: &HtmlInput, plan: &Plan) -> Result<InteropResult, Box<InteropError>> {
    let plan_digest_sha256 = exact_plan_digest_sha256(plan);
    plan.validate()
        .map_err(|error| Box::new(plan_invalid_error(plan, &plan_digest_sha256, error)))?;

    let request = compile_request(source, plan);
    let runtime = runtime_options(source);
    let extraction = extract(&request, &runtime);

    if !extraction.ok {
        return Err(Box::new(core_execution_error(
            plan,
            &plan_digest_sha256,
            &extraction.diagnostics,
        )));
    }

    adapt_successful_extraction(source, plan, plan_digest_sha256, extraction)
}

#[cfg(test)]
pub(crate) fn compile_request_for_tests(
    source: &HtmlInput,
    plan: &Plan,
) -> crate::ExtractionRequest {
    compile::compile_request(source, plan)
}

#[cfg(test)]
pub(crate) fn compile_regex_flags_for_tests(flags: &[super::RegexFlag]) -> String {
    compile::compile_regex_flags(flags)
}

#[cfg(test)]
pub(crate) fn project_structured_match_for_tests(
    matched: &crate::result::ExtractionMatch,
    strategy_kind: super::StrategyKind,
    diagnostics: &[crate::Diagnostic],
) -> Result<(), Box<InteropError>> {
    project::project_structured_match(matched, strategy_kind, "plan-digest", diagnostics)
        .map(|_| ())
}

#[cfg(test)]
pub(crate) fn parse_optional_url_for_tests(
    value: Option<&str>,
    field: &'static str,
    diagnostics: &[crate::Diagnostic],
) -> Result<Option<url::Url>, Box<InteropError>> {
    project::parse_optional_url(
        value,
        "plan-digest",
        super::StrategyKind::CssSelector,
        field,
        diagnostics,
    )
}

#[cfg(test)]
pub(crate) fn core_execution_error_for_tests(
    plan: &Plan,
    diagnostics: &[crate::Diagnostic],
) -> InteropError {
    let plan_digest_sha256 = exact_plan_digest_sha256(plan);
    errors::core_execution_error(plan, &plan_digest_sha256, diagnostics)
}

#[cfg(test)]
pub(crate) fn internal_adapter_error_for_tests(
    message: impl Into<String>,
    details: std::collections::BTreeMap<String, serde_json::Value>,
    diagnostics: Vec<crate::Diagnostic>,
) -> InteropError {
    errors::internal_adapter_error(
        "plan-digest",
        Some(super::StrategyKind::CssSelector),
        message,
        details,
        diagnostics,
    )
}

#[cfg(test)]
pub(crate) fn adapt_successful_extraction_for_tests(
    source: &HtmlInput,
    plan: &Plan,
    extraction: crate::ExtractionResult,
) -> Result<InteropResult, Box<InteropError>> {
    project::adapt_successful_extraction(source, plan, exact_plan_digest_sha256(plan), extraction)
}
