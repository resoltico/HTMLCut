mod compile;
mod errors;
mod project;

use compile::{compile_request, exact_plan_digest_sha256, runtime_options};
use errors::{core_execution_error, plan_digest_error, plan_invalid_error};
use project::adapt_successful_extraction;

use crate::extract;

use super::{HtmlInput, InteropError, InteropResult, Plan};

#[cfg(test)]
const TEST_PLAN_DIGEST_SHA256: &str =
    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

/// One plan that has already passed interop validation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidatedPlan {
    plan: Plan,
    plan_digest_sha256: String,
}

impl ValidatedPlan {
    /// Returns the validated plan document.
    pub fn plan(&self) -> &Plan {
        &self.plan
    }

    /// Returns the exact SHA-256 digest of the validated plan document.
    pub fn plan_digest_sha256(&self) -> &str {
        &self.plan_digest_sha256
    }
}

/// Validates one plan and returns a reusable validated execution input on success.
pub fn prepare_plan(plan: &Plan) -> Result<ValidatedPlan, Box<InteropError>> {
    let plan_digest_sha256 =
        exact_plan_digest_sha256(plan).map_err(|error| Box::new(plan_digest_error(plan, error)))?;
    plan.validate()
        .map_err(|error| Box::new(plan_invalid_error(plan, &plan_digest_sha256, error)))?;

    Ok(ValidatedPlan {
        plan: plan.clone(),
        plan_digest_sha256,
    })
}

/// Executes one previously validated plan directly against in-memory HTML input.
pub fn execute_validated_plan(
    source: &HtmlInput,
    validated_plan: &ValidatedPlan,
) -> Result<InteropResult, Box<InteropError>> {
    let request = compile_request(source, validated_plan.plan());
    let runtime = runtime_options(source);
    let extraction = extract(&request, &runtime);

    if !extraction.ok {
        return Err(Box::new(core_execution_error(
            validated_plan.plan(),
            validated_plan.plan_digest_sha256(),
            &extraction.diagnostics,
        )));
    }

    adapt_successful_extraction(
        source,
        validated_plan.plan(),
        validated_plan.plan_digest_sha256.clone(),
        extraction,
    )
}

/// Executes one plan directly against in-memory HTML input.
pub fn execute_plan(source: &HtmlInput, plan: &Plan) -> Result<InteropResult, Box<InteropError>> {
    let validated_plan = prepare_plan(plan)?;
    execute_validated_plan(source, &validated_plan)
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
    project::project_structured_match(matched, strategy_kind, TEST_PLAN_DIGEST_SHA256, diagnostics)
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
        TEST_PLAN_DIGEST_SHA256,
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
    let plan_digest_sha256 = exact_plan_digest_sha256(plan).expect("plan digest");
    errors::core_execution_error(plan, &plan_digest_sha256, diagnostics)
}

#[cfg(test)]
pub(crate) fn internal_adapter_error_for_tests(
    message: impl Into<String>,
    details: std::collections::BTreeMap<String, serde_json::Value>,
    diagnostics: Vec<crate::Diagnostic>,
) -> InteropError {
    errors::internal_adapter_error(
        TEST_PLAN_DIGEST_SHA256,
        Some(super::StrategyKind::CssSelector),
        message,
        details,
        diagnostics,
    )
}

#[cfg(test)]
pub(crate) fn internal_adapter_error_with_plan_digest_for_tests(
    plan_digest_sha256: &str,
    message: impl Into<String>,
    details: std::collections::BTreeMap<String, serde_json::Value>,
    diagnostics: Vec<crate::Diagnostic>,
) -> InteropError {
    errors::internal_adapter_error(
        plan_digest_sha256,
        Some(super::StrategyKind::CssSelector),
        message,
        details,
        diagnostics,
    )
}

#[cfg(test)]
pub(crate) fn finalize_error_for_tests(error: InteropError) -> InteropError {
    errors::finalize_error(error)
}

#[cfg(test)]
pub(crate) fn plan_digest_error_for_tests(
    plan: &Plan,
    error: super::ContractError,
) -> InteropError {
    errors::plan_digest_error(plan, error)
}

#[cfg(test)]
pub(crate) fn adapt_successful_extraction_for_tests(
    source: &HtmlInput,
    plan: &Plan,
    extraction: crate::ExtractionResult,
) -> Result<InteropResult, Box<InteropError>> {
    project::adapt_successful_extraction(
        source,
        plan,
        exact_plan_digest_sha256(plan).expect("plan digest"),
        extraction,
    )
}
