//! Interop error document and the permanent contract-error validation rules.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::super::super::stable_json::digest_stable_json_omitting_field;
use super::super::plan::StrategyKind;
use super::super::shared::{
    ContractError, ERROR_SCHEMA_NAME, ERROR_SCHEMA_VERSION, INTEROP_V1_PROFILE,
    INVALID_SELECTOR_MESSAGE, validate_message_bytes, validate_schema_identity,
    validate_sha256_hex,
};
use super::{InteropDiagnostic, InteropDiagnosticCode};
use crate::selector_parse::{SelectorParseDetailsViolation, validate_selector_parse_details};

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
    /// Machine-readable detail object.
    pub details: BTreeMap<String, Value>,
    /// Underlying HTMLCut diagnostics that produced this error.
    pub diagnostics: Vec<InteropDiagnostic>,
    /// Reserved extension object ignored by v1 consumers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<BTreeMap<String, Value>>,
}

impl InteropError {
    /// Builds one extraction error with the v1 schema identity.
    pub fn new(
        plan_digest_sha256: impl Into<String>,
        error_code: ErrorCode,
        message: impl Into<String>,
        strategy_kind: Option<StrategyKind>,
        details: BTreeMap<String, Value>,
        diagnostics: Vec<InteropDiagnostic>,
    ) -> Self {
        Self {
            schema_name: ERROR_SCHEMA_NAME.to_owned(),
            schema_version: ERROR_SCHEMA_VERSION,
            interop_profile: INTEROP_V1_PROFILE.to_owned(),
            plan_digest_sha256: plan_digest_sha256.into(),
            error_digest_sha256: String::new(),
            error_code,
            message: message.into(),
            strategy_kind,
            details,
            diagnostics,
            extensions: None,
        }
    }

    fn validate_body(&self) -> Result<(), ContractError> {
        validate_schema_identity(
            &self.schema_name,
            ERROR_SCHEMA_NAME,
            self.schema_version,
            ERROR_SCHEMA_VERSION,
            &self.interop_profile,
            INTEROP_V1_PROFILE,
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
        let core_diagnostic_code = self
            .details
            .get("core_diagnostic_code")
            .and_then(Value::as_str);
        let identifies_invalid_selector = !matching_diagnostics.is_empty()
            || core_diagnostic_code == Some(InteropDiagnosticCode::InvalidSelector.as_str());

        if !identifies_invalid_selector {
            return Ok(());
        }

        if core_diagnostic_code != Some(InteropDiagnosticCode::InvalidSelector.as_str()) {
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

        let diagnostic_details = matching_diagnostics[0].details.as_ref().ok_or(
            ContractError::MissingSelectorParseDetails {
                carrier: "diagnostic.details",
            },
        )?;
        let diagnostic_selector_parse = validate_selector_parse_details(diagnostic_details)
            .map_err(|violation| selector_parse_contract_error("diagnostic.details", violation))?;
        let core_details =
            self.details
                .get("core_details")
                .ok_or(ContractError::MissingSelectorParseDetails {
                    carrier: "details.core_details",
                })?;
        let core_selector_parse =
            validate_selector_parse_details(core_details).map_err(|violation| {
                selector_parse_contract_error("details.core_details", violation)
            })?;

        if diagnostic_selector_parse != core_selector_parse {
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
        super::super::super::stable_json::stable_json_v1(self)
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

fn selector_parse_contract_error(
    carrier: &'static str,
    violation: SelectorParseDetailsViolation,
) -> ContractError {
    match violation {
        SelectorParseDetailsViolation::Missing => {
            ContractError::MissingSelectorParseDetails { carrier }
        }
        SelectorParseDetailsViolation::Malformed => {
            ContractError::MalformedSelectorParseDetails { carrier }
        }
        SelectorParseDetailsViolation::NonObject => {
            ContractError::NonObjectSelectorParseDetails { carrier }
        }
        SelectorParseDetailsViolation::ZeroPosition => {
            ContractError::ZeroPositionSelectorParseDetails { carrier }
        }
        SelectorParseDetailsViolation::UnknownClass => {
            ContractError::UnknownSelectorParseErrorClass { carrier }
        }
    }
}
