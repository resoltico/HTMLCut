use std::collections::BTreeMap;
use std::num::NonZeroUsize;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use url::Url;

use crate::{Diagnostic, DiagnosticLevel, result::Range};

use super::super::stable_json::digest_stable_json_omitting_field;
use super::plan::{OutputKind, SelectionMode, StrategyKind};
use super::shared::{
    ContractError, ERROR_SCHEMA_NAME, ERROR_SCHEMA_VERSION, INTEROP_V1_PROFILE, RESULT_SCHEMA_NAME,
    RESULT_SCHEMA_VERSION, validate_schema_identity,
};

/// Source summary carried in one successful extraction result.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ResultSource {
    /// Base URL supplied before document parsing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_base_url: Option<Url>,
    /// Effective base URL after document `<base href>` resolution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effective_base_url: Option<Url>,
    /// Parsed document title when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_title: Option<String>,
}

/// Execution summary fields shared by one successful extraction result.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ResultExecution {
    /// Digest of the exact validated plan document.
    pub plan_digest_sha256: String,
    /// Executed strategy kind.
    pub strategy_kind: StrategyKind,
    /// Executed selection mode.
    pub selection_mode: SelectionMode,
    /// Total candidate count before selection.
    pub candidate_count: usize,
}

impl ResultExecution {
    /// Builds one extraction execution summary.
    pub fn new(
        plan_digest_sha256: impl Into<String>,
        strategy_kind: StrategyKind,
        selection_mode: SelectionMode,
        candidate_count: usize,
    ) -> Self {
        Self {
            plan_digest_sha256: plan_digest_sha256.into(),
            strategy_kind,
            selection_mode,
            candidate_count,
        }
    }
}

/// Typed metadata for one selected match.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SelectedMatchMetadata {
    /// Selector-backed selected match metadata.
    CssSelector {
        /// Total selector candidate count before selection.
        candidate_count: usize,
        /// Selected 1-based candidate ordinal.
        candidate_index: NonZeroUsize,
        /// DOM path to the selected node.
        path: String,
        /// Matched tag name.
        tag_name: String,
    },
    /// Delimiter-pair selected match metadata.
    DelimiterPair {
        /// Total slice candidate count before selection.
        candidate_count: usize,
        /// Selected 1-based candidate ordinal.
        candidate_index: NonZeroUsize,
        /// Selected byte range after applying the include-start/include-end policy.
        selected_range: Range,
        /// Inner byte range between the two matched boundaries.
        inner_range: Range,
        /// Outer byte range including both matched boundaries.
        outer_range: Range,
        /// Whether the selected payload includes the matched start boundary.
        include_start: bool,
        /// Whether the selected payload includes the matched end boundary.
        include_end: bool,
    },
}

impl SelectedMatchMetadata {
    /// Returns the strategy kind described by this metadata.
    pub const fn kind(&self) -> StrategyKind {
        match self {
            Self::CssSelector { .. } => StrategyKind::CssSelector,
            Self::DelimiterPair { .. } => StrategyKind::DelimiterPair,
        }
    }

    pub(super) fn candidate_count(&self) -> usize {
        match self {
            Self::CssSelector {
                candidate_count, ..
            }
            | Self::DelimiterPair {
                candidate_count, ..
            } => *candidate_count,
        }
    }

    pub(super) fn candidate_index(&self) -> NonZeroUsize {
        match self {
            Self::CssSelector {
                candidate_index, ..
            }
            | Self::DelimiterPair {
                candidate_index, ..
            } => *candidate_index,
        }
    }
}

/// One selected match returned on successful extraction.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct SelectedMatch {
    /// Selected 1-based candidate ordinal within all discovered candidates.
    pub candidate_index: NonZeroUsize,
    /// Exact output payload kind requested.
    pub value_kind: OutputKind,
    /// Exact output payload returned for the selected candidate.
    pub value: String,
    /// Text handed into compare-time canonicalization.
    pub comparison_input_text: String,
    /// Inner HTML for the selected match when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inner_html: Option<String>,
    /// Outer HTML for the selected match when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outer_html: Option<String>,
    /// Stable typed metadata for the selected match.
    pub metadata: SelectedMatchMetadata,
}

/// Successful extraction result owned by HTMLCut.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct InteropResult {
    /// Frozen schema identity.
    pub schema_name: String,
    /// Frozen schema version.
    pub schema_version: u32,
    /// Frozen interoperability profile identifier.
    pub interop_profile: String,
    /// Digest of the exact validated plan document.
    pub plan_digest_sha256: String,
    /// Digest of this exact result document with this field omitted.
    pub result_digest_sha256: String,
    /// Executed strategy kind.
    pub strategy_kind: StrategyKind,
    /// Executed selection mode.
    pub selection_mode: SelectionMode,
    /// Total candidate count before selection.
    pub candidate_count: usize,
    /// Source summary.
    pub source: ResultSource,
    /// Exactly one selected match.
    pub selected_match: SelectedMatch,
    /// Warning and informational diagnostics emitted during extraction.
    pub diagnostics: Vec<Diagnostic>,
    /// Reserved extension object ignored by v1 consumers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<BTreeMap<String, Value>>,
}

impl InteropResult {
    /// Builds one successful extraction result with the frozen v1 schema identity.
    pub fn new(
        execution: ResultExecution,
        source: ResultSource,
        selected_match: SelectedMatch,
        diagnostics: Vec<Diagnostic>,
    ) -> Self {
        Self {
            schema_name: RESULT_SCHEMA_NAME.to_owned(),
            schema_version: RESULT_SCHEMA_VERSION,
            interop_profile: INTEROP_V1_PROFILE.to_owned(),
            plan_digest_sha256: execution.plan_digest_sha256,
            result_digest_sha256: String::new(),
            strategy_kind: execution.strategy_kind,
            selection_mode: execution.selection_mode,
            candidate_count: execution.candidate_count,
            source,
            selected_match,
            diagnostics,
            extensions: None,
        }
    }

    /// Validates the schema identity and semantic invariants for this result.
    pub fn validate(&self) -> Result<(), ContractError> {
        validate_schema_identity(
            &self.schema_name,
            RESULT_SCHEMA_NAME,
            self.schema_version,
            RESULT_SCHEMA_VERSION,
            &self.interop_profile,
            INTEROP_V1_PROFILE,
        )?;

        if self.candidate_count == 0 {
            return Err(ContractError::ZeroCandidateCount);
        }

        let selected = self.selected_match.candidate_index.get();
        if selected > self.candidate_count {
            return Err(ContractError::SelectedCandidateOutOfRange {
                selected,
                candidate_count: self.candidate_count,
            });
        }

        if self.selected_match.metadata.kind() != self.strategy_kind {
            return Err(ContractError::MetadataKindMismatch {
                strategy_kind: self.strategy_kind,
                metadata_kind: self.selected_match.metadata.kind(),
            });
        }

        if self.selected_match.metadata.candidate_count() != self.candidate_count
            || self.selected_match.metadata.candidate_index() != self.selected_match.candidate_index
        {
            return Err(ContractError::SelectedCandidateOutOfRange {
                selected,
                candidate_count: self.candidate_count,
            });
        }

        if self
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.level == DiagnosticLevel::Error)
        {
            return Err(ContractError::ErrorDiagnosticsInSuccess);
        }

        Ok(())
    }

    /// Serializes this result with the frozen stable JSON profile.
    pub fn stable_json(&self) -> Result<String, ContractError> {
        self.validate()?;
        super::super::stable_json::stable_json_v1(self)
    }

    /// Computes the SHA-256 digest of this result with `result_digest_sha256` omitted.
    pub fn digest_sha256(&self) -> Result<String, ContractError> {
        self.validate()?;
        digest_stable_json_omitting_field(self, "result_digest_sha256")
    }

    /// Computes and stores `result_digest_sha256` on this result.
    pub fn with_computed_digest(mut self) -> Result<Self, ContractError> {
        self.result_digest_sha256 = self.digest_sha256()?;
        Ok(self)
    }
}

/// Frozen extraction error vocabulary owned by HTMLCut.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    /// The plan was invalid for the frozen interop profile.
    PlanInvalid,
    /// No candidate matched the requested strategy and selection.
    NoMatch,
    /// Exact-one selection saw multiple candidates.
    AmbiguousMatch,
    /// An internal failure occurred inside HTMLCut.
    InternalError,
}

/// Typed extraction error document owned by HTMLCut.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct InteropError {
    /// Frozen schema identity.
    pub schema_name: String,
    /// Frozen schema version.
    pub schema_version: u32,
    /// Frozen interoperability profile identifier.
    pub interop_profile: String,
    /// Digest of the exact validated plan document.
    pub plan_digest_sha256: String,
    /// Digest of this exact error document with this field omitted.
    pub error_digest_sha256: String,
    /// Frozen error code.
    pub error_code: ErrorCode,
    /// Human-readable error summary.
    pub message: String,
    /// Strategy kind when one was known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strategy_kind: Option<StrategyKind>,
    /// Machine-readable detail object.
    pub details: BTreeMap<String, Value>,
    /// Underlying HTMLCut diagnostics that produced this error.
    pub diagnostics: Vec<Diagnostic>,
    /// Reserved extension object ignored by v1 consumers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<BTreeMap<String, Value>>,
}

impl InteropError {
    /// Builds one extraction error with the frozen v1 schema identity.
    pub fn new(
        plan_digest_sha256: impl Into<String>,
        error_code: ErrorCode,
        message: impl Into<String>,
        strategy_kind: Option<StrategyKind>,
        details: BTreeMap<String, Value>,
        diagnostics: Vec<Diagnostic>,
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

    /// Validates the schema identity for this error document.
    pub fn validate(&self) -> Result<(), ContractError> {
        validate_schema_identity(
            &self.schema_name,
            ERROR_SCHEMA_NAME,
            self.schema_version,
            ERROR_SCHEMA_VERSION,
            &self.interop_profile,
            INTEROP_V1_PROFILE,
        )
    }

    /// Serializes this error with the frozen stable JSON profile.
    pub fn stable_json(&self) -> Result<String, ContractError> {
        self.validate()?;
        super::super::stable_json::stable_json_v1(self)
    }

    /// Computes the SHA-256 digest of this error with `error_digest_sha256` omitted.
    pub fn digest_sha256(&self) -> Result<String, ContractError> {
        self.validate()?;
        digest_stable_json_omitting_field(self, "error_digest_sha256")
    }

    /// Computes and stores `error_digest_sha256` on this error document.
    pub fn with_computed_digest(mut self) -> Result<Self, ContractError> {
        self.error_digest_sha256 = self.digest_sha256()?;
        Ok(self)
    }
}
