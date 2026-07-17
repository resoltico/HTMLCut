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
pub const PLAN_SCHEMA_VERSION: u32 = 7;
/// Schema version for the extraction result.
pub const RESULT_SCHEMA_VERSION: u32 = 8;
/// Schema version for the extraction error.
pub const ERROR_SCHEMA_VERSION: u32 = 3;
/// Exact public message for an invalid CSS selector.
pub(crate) const INVALID_SELECTOR_MESSAGE: &str = "CSS selector is invalid.";
/// Maximum UTF-8 byte length for one human-readable interop error or diagnostic message.
pub(super) const MAX_INTEROP_MESSAGE_BYTES: usize = 1024;

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
    /// DOM canonicalization was requested for a non-CSS strategy.
    #[error("dom_canonicalization is only valid for css_selector strategies")]
    DomCanonicalizationRequiresCssSelector,
    /// DOM canonicalization was requested for an output that has no comparison-text projection.
    #[error(
        "dom_canonicalization requires css_selector text or structured output; output kind {output_kind:?} does not expose comparison text"
    )]
    DomCanonicalizationRequiresComparisonTextOutput {
        /// Output kind that cannot expose a detached-clone text projection.
        output_kind: super::plan::OutputKind,
    },
    /// DOM canonicalization ignored the attribute selected for direct measurement.
    #[error("dom_canonicalization cannot ignore measured attribute {attribute:?}")]
    DomCanonicalizationIgnoresMeasuredAttribute {
        /// Attribute that both canonicalization ignored and output requested.
        attribute: String,
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
    /// A non-CSS result unexpectedly carried a detached-clone comparison text projection.
    #[error("delimiter_pair selected matches must not carry comparison_text_output")]
    UnexpectedComparisonTextOutput,
    /// A raw CSS output kind unexpectedly carried a detached-clone comparison text projection.
    #[error(
        "selected matches for output kind {output_kind:?} must not carry comparison_text_output"
    )]
    UnexpectedComparisonTextOutputForOutput {
        /// Output kind declared by the result.
        output_kind: super::plan::OutputKind,
    },
    /// A text result's exact output did not agree with its authoritative text projection.
    #[error(
        "text output_value must equal comparison_text_output when present, otherwise text_output"
    )]
    TextOutputValueMismatch,
    /// Raw structured evidence leaked the interop-only clone comparison projection.
    #[error("structured output_value must not contain comparisonTextOutput")]
    StructuredOutputContainsComparisonText,
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
    /// One public interop message exceeded its fixed byte bound.
    #[error("{field} must contain at most {maximum} UTF-8 bytes; received {received} bytes")]
    MessageTooLong {
        /// Public field whose bound was exceeded.
        field: &'static str,
        /// Maximum allowed UTF-8 byte length.
        maximum: usize,
        /// Received UTF-8 byte length.
        received: usize,
    },
    /// An invalid-selector interop error did not identify exactly one matching diagnostic.
    #[error(
        "invalid-selector interop error must carry exactly one matching diagnostic; received {received}"
    )]
    InvalidSelectorDiagnosticCardinality {
        /// Number of `INVALID_SELECTOR` diagnostics carried by the error.
        received: usize,
    },
    /// An invalid-selector interop error did not identify its core diagnostic consistently.
    #[error("invalid-selector interop error must identify INVALID_SELECTOR as its core diagnostic")]
    InvalidSelectorCoreDiagnostic,
    /// An invalid-selector error carried a non-canonical public message.
    #[error("{carrier} must be the exact invalid-selector message")]
    InvalidSelectorMessage {
        /// Public field that carried the non-canonical message.
        carrier: &'static str,
    },
    /// One selector parse detail object was missing.
    #[error("{carrier} selector_parse is required")]
    MissingSelectorParseDetails {
        /// Public carrier that omitted the required detail object.
        carrier: &'static str,
    },
    /// One selector parse detail object had an invalid field shape.
    #[error("{carrier} selector_parse is malformed")]
    MalformedSelectorParseDetails {
        /// Public carrier that held the malformed detail object.
        carrier: &'static str,
    },
    /// One selector parse detail object was not an object.
    #[error("{carrier} selector_parse must be an object")]
    NonObjectSelectorParseDetails {
        /// Public carrier that held the non-object detail value.
        carrier: &'static str,
    },
    /// One selector parse position was zero.
    #[error("{carrier} selector_parse position must be positive")]
    ZeroPositionSelectorParseDetails {
        /// Public carrier that held the zero-valued position.
        carrier: &'static str,
    },
    /// One selector parse detail object named an unknown parse-error class.
    #[error("{carrier} selector_parse parse_error_class is unknown")]
    UnknownSelectorParseErrorClass {
        /// Public carrier that held the invalid detail object.
        carrier: &'static str,
    },
    /// The selector parse detail copies in the two public error carriers disagreed.
    #[error("invalid-selector diagnostic and core_details selector_parse values must match")]
    MismatchedSelectorParseDetails,
}

pub(super) fn validate_message_bytes(
    field: &'static str,
    value: &str,
) -> Result<(), ContractError> {
    let received = value.len();
    if received <= MAX_INTEROP_MESSAGE_BYTES {
        Ok(())
    } else {
        Err(ContractError::MessageTooLong {
            field,
            maximum: MAX_INTEROP_MESSAGE_BYTES,
            received,
        })
    }
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
    if is_valid_sha256_hex(value) {
        Ok(())
    } else {
        Err(ContractError::InvalidDigest {
            field,
            received: value.to_owned(),
        })
    }
}

pub(crate) fn is_valid_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
