use thiserror::Error;

/// Frozen interop profile identifier for v1.
pub const INTEROP_V1_PROFILE: &str = "htmlcut-v1";
/// Frozen schema name for the extraction plan.
pub const PLAN_SCHEMA_NAME: &str = "htmlcut.plan";
/// Frozen schema name for the extraction result.
pub const RESULT_SCHEMA_NAME: &str = "htmlcut.result";
/// Frozen schema name for the extraction error.
pub const ERROR_SCHEMA_NAME: &str = "htmlcut.error";
/// Frozen schema version for the extraction plan.
pub const PLAN_SCHEMA_VERSION: u32 = 1;
/// Frozen schema version for the extraction result.
pub const RESULT_SCHEMA_VERSION: u32 = 1;
/// Frozen schema version for the extraction error.
pub const ERROR_SCHEMA_VERSION: u32 = 1;

/// Error returned when interop contract values or schema identities are invalid.
#[derive(Debug, Error)]
pub enum ContractError {
    /// The input could not be serialized into stable JSON.
    #[error("could not serialize interop JSON: {0}")]
    Serialize(#[from] serde_json::Error),
    /// One schema identity field was not the required frozen value.
    #[error("{field} must be {expected:?}; received {received:?}")]
    InvalidIdentity {
        /// Schema field name that failed validation.
        field: &'static str,
        /// Expected frozen value.
        expected: &'static str,
        /// Received value.
        received: String,
    },
    /// One schema version field was not the required frozen value.
    #[error("{field} must be {expected}; received {received}")]
    InvalidVersion {
        /// Schema field name that failed validation.
        field: &'static str,
        /// Expected frozen value.
        expected: u32,
        /// Received value.
        received: u32,
    },
    /// A delimiter-pair plan tried to use regex flags in literal mode.
    #[error("delimiter_pair flags are only valid when mode is regex")]
    LiteralDelimiterFlags,
    /// A successful result claimed to select zero candidates.
    #[error("successful extraction results must report at least one candidate")]
    ZeroCandidateCount,
    /// A selected match referenced a candidate outside the candidate count.
    #[error("selected candidate index {selected} is out of range for {candidate_count} candidates")]
    SelectedCandidateOutOfRange {
        /// Selected candidate ordinal.
        selected: usize,
        /// Total candidate count.
        candidate_count: usize,
    },
    /// A successful result carried error-level diagnostics.
    #[error("successful extraction results must not contain error-level diagnostics")]
    ErrorDiagnosticsInSuccess,
    /// Result metadata did not describe the same strategy kind as the top-level result.
    #[error(
        "selected match metadata kind {metadata_kind:?} does not match result strategy kind {strategy_kind:?}"
    )]
    MetadataKindMismatch {
        /// Strategy kind declared by the result.
        strategy_kind: super::plan::StrategyKind,
        /// Strategy kind declared by the selected match metadata.
        metadata_kind: super::plan::StrategyKind,
    },
    /// The source label was blank.
    #[error("source label must not be empty")]
    EmptySourceLabel,
}

pub(super) fn validate_schema_identity(
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
