//! Versioned extraction interop contracts (v2).

mod execution;
mod exploration;
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
/// Monotonic identity for exploration and selector-proposal semantics.
pub const HTMLCUT_DISCOVERY_SEMANTICS_VERSION: u32 = 1;

pub(crate) use types::{INVALID_SELECTOR_MESSAGE, is_valid_sha256_hex};

pub use crate::selector_parse::{SelectorParseDetail, SelectorParseErrorClass};
pub use execution::{
    CompiledPlan, PrepareSourceError, PreparedSource, compile_plan, execute, inspect_source,
    prepare_document, prepare_source,
};
#[cfg(test)]
pub(crate) use execution::{
    adapt_successful_extraction_for_tests, compile_regex_flags_for_tests,
    compile_request_for_tests, core_execution_error_for_tests, exact_plan_digest_sha256_for_tests,
    execute_one_shot_for_tests, finalize_error_for_tests, finalize_sanitized_fallback_for_tests,
    internal_adapter_error_for_tests, internal_adapter_error_with_plan_digest_for_tests,
    parse_optional_url_for_tests, plan_digest_error_for_tests, preparation_parse_count_for_tests,
    prepare_loaded_source_for_tests, project_plain_text_for_tests,
    project_structured_match_for_tests, reset_preparation_parse_count_for_tests,
    source_inspection_from_prepared_for_tests,
};
#[cfg(test)]
pub(crate) use exploration::{
    element_namespace_for_tests, exploration_cursor_for_tests, is_lower_sha256_for_tests,
    normalized_dom_text_nodes_for_tests, selector_is_unique_for_tests,
};
pub use exploration::{explore, normalized_dom_text_digest, resolve_target_and_propose};
#[cfg(test)]
pub(crate) use stable_json::digest_stable_json_omitting_field_for_tests;
pub use stable_json::stable_json_v2;
#[cfg(test)]
pub(crate) use types::selected_match_count_for_tests;
pub use types::{
    AttributeName, ByteRange, ContractError, CssSelectorText, DelimiterBoundaryRetention,
    DelimiterBoundaryText, DelimiterMode, DisplayedHttpUrl, DomCanonicalization, ERROR_SCHEMA_NAME,
    ERROR_SCHEMA_VERSION, EXPLORATION_ERROR_SCHEMA_NAME, EXPLORATION_ERROR_SCHEMA_VERSION,
    EXPLORATION_RESULT_SCHEMA_NAME, EXPLORATION_RESULT_SCHEMA_VERSION, ElementDescriptor,
    ElementName, ElementNamespace, ElementTargetHint, ErrorCode, ExecutionBudget,
    ExplorationAttribute, ExplorationCursor, ExplorationError, ExplorationErrorCode,
    ExplorationOmission, ExplorationOmissionReason, ExplorationOptions, ExplorationResult,
    ExplorationTruncationReason, HtmlInput, HttpUrl, INTEROP_V2_PROFILE, InteropAdapterFailure,
    InteropDiagnostic, InteropDiagnosticCode, InteropDiagnosticLevel, InteropError,
    InteropErrorDetail, InteropFinalizationRejection, InteropResult, Output, OutputKind,
    PLAN_SCHEMA_NAME, PLAN_SCHEMA_VERSION, PREPARATION_ERROR_SCHEMA_NAME,
    PREPARATION_ERROR_SCHEMA_VERSION, Plan, PlanStrategy, PreparationError, PreparationErrorCode,
    PreparationLimits, PreparedDocument, RESULT_SCHEMA_NAME, RESULT_SCHEMA_VERSION, RegexFlag,
    Rendering, ResultExecution, ResultSource, SelectedMatch, SelectedMatchMetadata, Selection,
    SelectionMode, SelectorProposal, SelectorStability, SelectorStabilityReason, StrategyKind,
    TARGET_RESOLUTION_RESULT_SCHEMA_NAME, TARGET_RESOLUTION_RESULT_SCHEMA_VERSION,
    TargetResolutionOptions, TargetResolutionResult, TextWhitespace,
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
