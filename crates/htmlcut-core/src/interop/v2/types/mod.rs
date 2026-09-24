mod exploration;
mod plan;
mod prepared;
mod result;
mod shared;

pub use crate::{AttributeName, DisplayedHttpUrl, HttpUrl};
pub use exploration::{
    EXPLORATION_ERROR_SCHEMA_NAME, EXPLORATION_ERROR_SCHEMA_VERSION,
    EXPLORATION_RESULT_SCHEMA_NAME, EXPLORATION_RESULT_SCHEMA_VERSION, ElementDescriptor,
    ElementName, ElementNamespace, ElementTargetHint, ExplorationAttribute, ExplorationCursor,
    ExplorationError, ExplorationErrorCode, ExplorationOmission, ExplorationOmissionReason,
    ExplorationOptions, ExplorationResult, ExplorationTruncationReason, SelectorProposal,
    SelectorStability, SelectorStabilityReason, TARGET_RESOLUTION_RESULT_SCHEMA_NAME,
    TARGET_RESOLUTION_RESULT_SCHEMA_VERSION, TargetResolutionOptions, TargetResolutionResult,
};
pub use plan::{
    CssSelectorText, DelimiterBoundaryRetention, DelimiterBoundaryText, DelimiterMode,
    DomCanonicalization, ExecutionBudget, HtmlInput, Output, OutputKind, Plan, PlanStrategy,
    RegexFlag, Rendering, Selection, SelectionMode, StrategyKind, TextWhitespace,
};
pub use prepared::{
    PREPARATION_ERROR_SCHEMA_NAME, PREPARATION_ERROR_SCHEMA_VERSION, PreparationError,
    PreparationErrorCode, PreparationLimits, PreparedDocument,
};
#[cfg(test)]
pub(crate) use result::selected_match_count_for_tests;
pub use result::{
    ByteRange, ErrorCode, InteropAdapterFailure, InteropDiagnostic, InteropDiagnosticCode,
    InteropDiagnosticLevel, InteropError, InteropErrorDetail, InteropFinalizationRejection,
    InteropResult, ResultExecution, ResultSource, SelectedMatch, SelectedMatchMetadata,
};
pub use shared::{
    ContractError, ERROR_SCHEMA_NAME, ERROR_SCHEMA_VERSION, INTEROP_V2_PROFILE, PLAN_SCHEMA_NAME,
    PLAN_SCHEMA_VERSION, RESULT_SCHEMA_NAME, RESULT_SCHEMA_VERSION,
};
pub(crate) use shared::{INVALID_SELECTOR_MESSAGE, is_valid_sha256_hex};

#[cfg(test)]
pub(super) fn validate_schema_identity(
    schema_name: &str,
    expected_schema_name: &'static str,
    schema_version: u32,
    expected_schema_version: u32,
    interop_profile: &str,
    expected_interop_profile: &'static str,
) -> Result<(), ContractError> {
    shared::validate_schema_identity(
        schema_name,
        expected_schema_name,
        schema_version,
        expected_schema_version,
        interop_profile,
        expected_interop_profile,
    )
}
