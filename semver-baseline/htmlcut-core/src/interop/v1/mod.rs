//! Versioned extraction interop contracts (v1).

mod execution;
mod stable_json;
mod types;

/// Monotonic identity for HTMLCut extraction semantics.
///
/// Increment this counter whenever the same complete [`HtmlInput`] and a plan that passes
/// preflight could produce a different projected extraction result. It is deliberately independent
/// of the HTMLCut crate version, the core specification version, and dependency versions.
///
/// Invalid-plan diagnostic envelope changes belong to the versioned interop error contract rather
/// than this measurement identity.
///
/// [`HtmlInput::extraction_identity_sha256`] includes this counter in the identity that
/// downstream consumers persist for one extraction.
pub const HTMLCUT_EXTRACTION_SEMANTICS_VERSION: u32 = 4;

pub(crate) use types::{INVALID_SELECTOR_MESSAGE, is_valid_sha256_hex};

pub use execution::{ValidatedPlan, execute_plan, execute_validated_plan, prepare_plan};
#[cfg(test)]
pub(crate) use execution::{
    adapt_successful_extraction_for_tests, compile_regex_flags_for_tests,
    compile_request_for_tests, core_execution_error_for_tests, exact_plan_digest_sha256_for_tests,
    finalize_error_for_tests, finalize_sanitized_fallback_for_tests,
    internal_adapter_error_for_tests, internal_adapter_error_with_plan_digest_for_tests,
    parse_optional_url_for_tests, plan_digest_error_for_tests, project_plain_text_for_tests,
    project_structured_match_for_tests,
};
#[cfg(test)]
pub(crate) use stable_json::digest_stable_json_omitting_field_for_tests;
pub use stable_json::stable_json_v1;
pub use types::{
    AttributeName, ByteRange, ContractError, CssSelectorText, DelimiterBoundaryRetention,
    DelimiterBoundaryText, DelimiterMode, DisplayedHttpUrl, DomCanonicalization, ERROR_SCHEMA_NAME,
    ERROR_SCHEMA_VERSION, ErrorCode, HtmlInput, HttpUrl, INTEROP_V1_PROFILE, InteropDiagnostic,
    InteropDiagnosticCode, InteropDiagnosticLevel, InteropError, InteropResult, Output, OutputKind,
    PLAN_SCHEMA_NAME, PLAN_SCHEMA_VERSION, Plan, PlanStrategy, RESULT_SCHEMA_NAME,
    RESULT_SCHEMA_VERSION, RegexFlag, Rendering, ResultExecution, ResultSource, SelectedMatch,
    SelectedMatchMetadata, Selection, SelectionMode, StrategyKind, TextWhitespace,
};

#[cfg(test)]
pub(crate) fn validate_schema_identity_for_tests(
    schema_name: &str,
    expected_schema_name: &'static str,
    schema_version: u32,
    expected_schema_version: u32,
    interop_profile: &str,
    expected_interop_profile: &'static str,
) -> Result<(), ContractError> {
    types::validate_schema_identity(
        schema_name,
        expected_schema_name,
        schema_version,
        expected_schema_version,
        interop_profile,
        expected_interop_profile,
    )
}
