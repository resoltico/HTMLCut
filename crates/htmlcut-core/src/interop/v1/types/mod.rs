mod plan;
mod result;
mod shared;

pub use plan::{
    DelimiterMode, HtmlInput, Normalization, Output, OutputKind, Plan, PlanStrategy, RegexFlag,
    Selection, SelectionMode, StrategyKind, TextWhitespace,
};
pub use result::{
    ErrorCode, InteropError, InteropResult, ResultExecution, ResultSource, SelectedMatch,
    SelectedMatchMetadata,
};
pub use shared::{
    ContractError, ERROR_SCHEMA_NAME, ERROR_SCHEMA_VERSION, INTEROP_V1_PROFILE, PLAN_SCHEMA_NAME,
    PLAN_SCHEMA_VERSION, RESULT_SCHEMA_NAME, RESULT_SCHEMA_VERSION,
};

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
