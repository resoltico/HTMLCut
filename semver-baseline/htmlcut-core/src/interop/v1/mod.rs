//! Versioned extraction interop contracts (v1).

mod execution;
mod stable_json;
mod types;

#[cfg(test)]
pub(crate) use execution::{
    adapt_successful_extraction_for_tests, compile_regex_flags_for_tests,
    compile_request_for_tests, core_execution_error_for_tests, internal_adapter_error_for_tests,
    parse_optional_url_for_tests, project_structured_match_for_tests,
};
pub use execution::{execute_plan, validate_plan};
#[cfg(test)]
pub(crate) use stable_json::digest_stable_json_omitting_field_for_tests;
pub use stable_json::stable_json_v1;
pub use types::{
    ContractError, DelimiterMode, ERROR_SCHEMA_NAME, ERROR_SCHEMA_VERSION, ErrorCode, HtmlInput,
    INTEROP_V1_PROFILE, InteropError, InteropResult, Normalization, Output, OutputKind,
    PLAN_SCHEMA_NAME, PLAN_SCHEMA_VERSION, Plan, PlanStrategy, RESULT_SCHEMA_NAME,
    RESULT_SCHEMA_VERSION, RegexFlag, ResultExecution, ResultSource, SelectedMatch,
    SelectedMatchMetadata, Selection, SelectionMode, StrategyKind, TextWhitespace,
};

fn validate_schema_identity(
    schema_name: &str,
    expected_schema_name: &'static str,
    schema_version: u32,
    expected_schema_version: u32,
    interop_profile: &str,
    expected_interop_profile: &'static str,
) -> Result<(), ContractError> {
    if schema_name != expected_schema_name {
        return Err(ContractError::InvalidIdentity {
            field: "schema_name",
            expected: expected_schema_name,
            received: schema_name.to_owned(),
        });
    }

    if schema_version != expected_schema_version {
        return Err(ContractError::InvalidVersion {
            field: "schema_version",
            expected: expected_schema_version,
            received: schema_version,
        });
    }

    if interop_profile != expected_interop_profile {
        return Err(ContractError::InvalidIdentity {
            field: "interop_profile",
            expected: expected_interop_profile,
            received: interop_profile.to_owned(),
        });
    }

    Ok(())
}

#[cfg(test)]
pub(crate) fn validate_schema_identity_for_tests(
    schema_name: &str,
    expected_schema_name: &'static str,
    schema_version: u32,
    expected_schema_version: u32,
    interop_profile: &str,
    expected_interop_profile: &'static str,
) -> Result<(), ContractError> {
    validate_schema_identity(
        schema_name,
        expected_schema_name,
        schema_version,
        expected_schema_version,
        interop_profile,
        expected_interop_profile,
    )
}
