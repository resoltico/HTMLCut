use thiserror::Error;

/// Interop profile identifier for v1.
pub const INTEROP_V1_PROFILE: &str = "htmlcut-v1";
/// Schema name for the extraction plan.
pub const PLAN_SCHEMA_NAME: &str = "htmlcut.plan";
/// Schema name for the extraction result.
pub const RESULT_SCHEMA_NAME: &str = "htmlcut.result";
/// Schema name for the extraction error.
pub const ERROR_SCHEMA_NAME: &str = "htmlcut.error";
/// Schema version for the extraction plan.
pub const PLAN_SCHEMA_VERSION: u32 = 5;
/// Schema version for the extraction result.
pub const RESULT_SCHEMA_VERSION: u32 = 6;
/// Schema version for the extraction error.
pub const ERROR_SCHEMA_VERSION: u32 = 2;

/// Error returned when interop contract values or schema identities are invalid.
#[derive(Debug, Error)]
pub enum ContractError {
    /// The input could not be serialized into stable JSON.
    #[error("could not serialize interop JSON: {0}")]
    Serialize(#[from] serde_json::Error),
    /// One schema identity field was not the required value.
    #[error("{field} must be {expected:?}; received {received:?}")]
    InvalidIdentity {
        /// Schema field name that failed validation.
        field: &'static str,
        /// Expected value.
        expected: &'static str,
        /// Received value.
        received: String,
    },
    /// One schema version field was not the required value.
    #[error("{field} must be {expected}; received {received}")]
    InvalidVersion {
        /// Schema field name that failed validation.
        field: &'static str,
        /// Expected value.
        expected: u32,
        /// Received value.
        received: u32,
    },
    /// One digest field was not a lowercase SHA-256 hex string.
    #[error("{field} must be a 64-character lowercase SHA-256 hex digest; received {received:?}")]
    InvalidDigest {
        /// Digest field name that failed validation.
        field: &'static str,
        /// Received value.
        received: String,
    },
    /// One stored digest did not match the canonical document content.
    #[error("{field} did not match the canonical document digest")]
    DigestMismatch {
        /// Digest field name that failed validation.
        field: &'static str,
        /// Recomputed canonical digest.
        expected: String,
        /// Received value.
        received: String,
    },
    /// A delimiter-pair plan tried to use regex flags in literal mode.
    #[error("delimiter_pair flags are only valid when mode is regex")]
    LiteralDelimiterFlags,
    /// One interop selector value was blank.
    #[error("css selector must not be empty")]
    EmptyCssSelector,
    /// One delimiter boundary value was blank.
    #[error("delimiter boundary must not be empty")]
    EmptyDelimiterBoundary,
    /// One attribute name was blank.
    #[error("attribute name must not be empty")]
    EmptyAttributeName,
    /// One attribute name contained whitespace.
    #[error("attribute name must not contain whitespace")]
    AttributeNameContainsWhitespace,
    /// The requested output kind does not belong to the selected strategy.
    #[error("output kind {output_kind:?} is not valid for strategy kind {strategy_kind:?}")]
    UnsupportedOutputForStrategy {
        /// Strategy kind declared by the plan or result.
        strategy_kind: super::plan::StrategyKind,
        /// Output kind declared by the plan or result.
        output_kind: super::plan::OutputKind,
    },
    /// A successful result claimed to select zero candidates.
    #[error("successful extraction results must report at least one candidate")]
    ZeroCandidateCount,
    /// A successful result claimed to select zero matches.
    #[error("successful extraction results must include at least one selected match")]
    ZeroSelectedMatchCount,
    /// A selected match referenced a candidate outside the candidate count.
    #[error("selected candidate index {selected} is out of range for {candidate_count} candidates")]
    SelectedCandidateOutOfRange {
        /// Selected candidate ordinal.
        selected: usize,
        /// Total candidate count.
        candidate_count: usize,
    },
    /// A successful result repeated the same candidate more than once.
    #[error("selected candidate index {selected} appeared more than once")]
    DuplicateSelectedCandidate {
        /// Repeated selected candidate ordinal.
        selected: usize,
    },
    /// A successful result carried error-level diagnostics.
    #[error("successful extraction results must not contain error-level diagnostics")]
    ErrorDiagnosticsInSuccess,
    /// One selected match carried a non-string exact output where a string output kind was requested.
    #[error(
        "selected matches for output kind {output_kind:?} must carry string output_value values"
    )]
    NonStringOutputValue {
        /// Output kind requested by the result.
        output_kind: super::plan::OutputKind,
    },
    /// One structured-output result carried a non-object exact output value.
    #[error("selected matches for structured output must carry object output_value values")]
    NonObjectStructuredOutputValue,
    /// A selector result unexpectedly carried a selected-html alternate output.
    #[error("css_selector selected matches must not carry selected_html_output")]
    UnexpectedSelectedHtmlOutput,
    /// A delimiter-pair result omitted its selected-html alternate output.
    #[error("delimiter_pair selected matches must carry selected_html_output")]
    MissingSelectedHtmlOutput,
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
    /// Selected-match cardinality did not agree with the selection mode and candidate count.
    #[error(
        "selection mode {selection_mode:?} expected {expected_selected_matches} selected matches for {candidate_count} candidates, but received {selected_match_count}"
    )]
    SelectionModeCountMismatch {
        /// Executed selection mode.
        selection_mode: super::plan::SelectionMode,
        /// Number of matches actually carried in the result.
        selected_match_count: usize,
        /// Number of matches the selection mode requires.
        expected_selected_matches: usize,
        /// Total candidate count.
        candidate_count: usize,
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

pub(super) fn validate_sha256_hex(field: &'static str, value: &str) -> Result<(), ContractError> {
    let valid = value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
    if valid {
        Ok(())
    } else {
        Err(ContractError::InvalidDigest {
            field,
            received: value.to_owned(),
        })
    }
}
