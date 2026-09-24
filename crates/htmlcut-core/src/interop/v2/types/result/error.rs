//! Interop error document and the permanent contract-error validation rules.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::super::super::stable_json::digest_stable_json_omitting_field;
use super::super::plan::StrategyKind;
use super::super::shared::{
    ContractError, ERROR_SCHEMA_NAME, ERROR_SCHEMA_VERSION, INTEROP_V2_PROFILE,
    INVALID_SELECTOR_MESSAGE, validate_message_bytes, validate_schema_identity,
    validate_sha256_hex,
};
use super::{InteropDiagnostic, InteropDiagnosticCode};
use crate::selector_parse::{SelectorParseDetail, validate_selector_parse_details};

/// Extraction error vocabulary owned by HTMLCut.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    /// The plan was invalid for the interop profile.
    PlanInvalid,
    /// No candidate matched the requested strategy and selection.
    NoMatch,
    /// Exact-one selection saw multiple candidates.
    AmbiguousMatch,
    /// The selected candidate did not carry the requested attribute.
    MissingAttribute,
    /// Execution exceeded a plan-owned resource limit.
    ExecutionLimitExceeded,
    /// An internal failure occurred inside HTMLCut.
    InternalError,
}

/// Typed extraction error document owned by HTMLCut.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct InteropError {
    /// Schema identity.
    pub schema_name: String,
    /// Schema version.
    pub schema_version: u32,
    /// Interoperability profile identifier.
    pub interop_profile: String,
    /// Digest of the exact validated plan document.
    pub plan_digest_sha256: String,
    /// Digest of this exact error document with this field omitted.
    pub error_digest_sha256: String,
    /// Interop error code.
    pub error_code: ErrorCode,
    /// Human-readable error summary.
    #[schemars(length(max = 1024))]
    pub message: String,
    /// Strategy kind when one was known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strategy_kind: Option<StrategyKind>,
    /// Closed HTMLCut-owned failure detail.
    pub detail: InteropErrorDetail,
    /// Underlying HTMLCut diagnostics that produced this error.
    pub diagnostics: Vec<InteropDiagnostic>,
}

impl InteropError {
    /// Builds one extraction error with the v2 schema identity.
    pub fn new(
        plan_digest_sha256: impl Into<String>,
        error_code: ErrorCode,
        message: impl Into<String>,
        strategy_kind: Option<StrategyKind>,
        detail: InteropErrorDetail,
        diagnostics: Vec<InteropDiagnostic>,
    ) -> Self {
        Self {
            schema_name: ERROR_SCHEMA_NAME.to_owned(),
            schema_version: ERROR_SCHEMA_VERSION,
            interop_profile: INTEROP_V2_PROFILE.to_owned(),
            plan_digest_sha256: plan_digest_sha256.into(),
            error_digest_sha256: String::new(),
            error_code,
            message: message.into(),
            strategy_kind,
            detail,
            diagnostics,
        }
    }

    fn validate_body(&self) -> Result<(), ContractError> {
        validate_schema_identity(
            &self.schema_name,
            ERROR_SCHEMA_NAME,
            self.schema_version,
            ERROR_SCHEMA_VERSION,
            &self.interop_profile,
            INTEROP_V2_PROFILE,
        )?;
        validate_sha256_hex("plan_digest_sha256", &self.plan_digest_sha256)?;
        validate_message_bytes("message", &self.message)?;
        for diagnostic in &self.diagnostics {
            diagnostic.validate_body()?;
        }
        self.validate_invalid_selector_contract()?;
        Ok(())
    }

    fn validate_invalid_selector_contract(&self) -> Result<(), ContractError> {
        let matching_diagnostics = self
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == InteropDiagnosticCode::InvalidSelector)
            .collect::<Vec<_>>();
        let InteropErrorDetail::InvalidSelector { selector_parse, .. } = &self.detail else {
            if matching_diagnostics.is_empty() {
                return Ok(());
            }
            return Err(ContractError::InvalidSelectorCoreDiagnostic);
        };

        if self.error_code != ErrorCode::PlanInvalid {
            return Err(ContractError::InvalidSelectorCoreDiagnostic);
        }
        if matching_diagnostics.len() != 1 {
            return Err(ContractError::InvalidSelectorDiagnosticCardinality {
                received: matching_diagnostics.len(),
            });
        }
        if self.message != INVALID_SELECTOR_MESSAGE {
            return Err(ContractError::InvalidSelectorMessage { carrier: "message" });
        }
        if matching_diagnostics[0].message != INVALID_SELECTOR_MESSAGE {
            return Err(ContractError::InvalidSelectorMessage {
                carrier: "diagnostic.message",
            });
        }
        let diagnostic_selector_parse = matching_diagnostics[0]
            .details
            .as_ref()
            .and_then(|detail| validate_selector_parse_details(detail).ok())
            .ok_or(ContractError::InvalidSelectorCoreDiagnostic)?;

        if diagnostic_selector_parse != *selector_parse {
            return Err(ContractError::MismatchedSelectorParseDetails);
        }

        Ok(())
    }

    /// Validates the schema identity and canonical digest for this error document.
    pub fn validate(&self) -> Result<(), ContractError> {
        self.validate_body()?;
        validate_sha256_hex("error_digest_sha256", &self.error_digest_sha256)?;

        let expected = digest_stable_json_omitting_field(self, "error_digest_sha256")?;
        if self.error_digest_sha256 != expected {
            return Err(ContractError::DigestMismatch {
                field: "error_digest_sha256",
                expected,
                received: self.error_digest_sha256.clone(),
            });
        }

        Ok(())
    }

    /// Serializes this error with the stable JSON profile.
    pub fn stable_json(&self) -> Result<String, ContractError> {
        self.validate()?;
        super::super::super::stable_json::stable_json_v2(self)
    }

    /// Computes the SHA-256 digest of this error with `error_digest_sha256` omitted.
    pub fn digest_sha256(&self) -> Result<String, ContractError> {
        self.validate_body()?;
        digest_stable_json_omitting_field(self, "error_digest_sha256")
    }

    /// Computes and stores `error_digest_sha256` on this error document.
    pub fn with_computed_digest(mut self) -> Result<Self, ContractError> {
        self.error_digest_sha256 = self.digest_sha256()?;
        Ok(self)
    }
}

/// Closed machine-readable evidence for an interop execution failure.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum InteropErrorDetail {
    /// Plan validation or canonical serialization rejected the declarative contract.
    ContractViolation,
    /// One core execution diagnostic determined the failure outcome.
    CoreExecution {
        /// Closed primary diagnostic code selected by HTMLCut.
        core_diagnostic_code: InteropDiagnosticCode,
        /// Exact candidate count at the point execution stopped.
        candidate_count: u32,
    },
    /// CSS grammar compilation failed before source preparation or execution.
    InvalidSelector {
        /// Exact candidate count, which is zero during source-independent compilation.
        candidate_count: u32,
        /// Normalized parser position and closed failure class.
        selector_parse: SelectorParseDetail,
    },
    /// A selected candidate lacked the attribute requested by the compiled plan.
    MissingRequestedAttribute,
    /// An internal adapter invariant was violated while projecting a core result.
    AdapterInvariant {
        /// Closed projection or finalization invariant that failed.
        failure: InteropAdapterFailure,
    },
    /// HTMLCut sanitized a malformed error payload before it could cross the boundary.
    FinalizationRejected {
        /// Closed reason the original payload could not be finalized.
        rejection: InteropFinalizationRejection,
    },
}

/// Closed adapter-invariant vocabulary for interop result projection.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InteropAdapterFailure {
    /// Core execution failed without an error-level diagnostic.
    MissingExecutionErrorDiagnostic,
    /// Core execution exposed an invalid URL where a typed URL was required.
    InvalidSourceUrl,
    /// A completed core extraction had no selected match.
    EmptySelectedMatchSet,
    /// A core match that must be structured was not a JSON object.
    ExpectedStructuredMatchObject,
    /// A required structured field was absent or had the wrong value shape.
    InvalidStructuredProjection,
    /// Core metadata supplied an invalid zero candidate ordinal.
    InvalidCandidateIndex,
    /// The selected output cannot be projected by the plan's strategy.
    UnsupportedOutputProjection,
    /// A successful interop result violated its own closed result contract.
    ResultFinalization,
}

/// Closed reason an error payload needed finalization sanitization.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InteropFinalizationRejection {
    /// A public message exceeded its bounded contract.
    MessageTooLong,
    /// Invalid-selector diagnostics did not match the closed invalid-selector shape.
    InvalidSelectorDiagnostic,
    /// Any other closed contract validation rejected the payload.
    InvalidInteropContract,
    /// The sanitized fallback itself could not be finalized.
    FallbackFinalizationFailed,
}

/*
 * `InteropErrorDetail` deliberately has no generic JSON map. The v2 error envelope is a
 * Published Language, so an unknown diagnostic or adapter condition must select one of the
 * closed variants above and keep its explanatory prose in the bounded `message` field.
 */
