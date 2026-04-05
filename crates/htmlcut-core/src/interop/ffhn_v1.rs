//! FFHN-facing versioned extraction interop contracts.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::num::NonZeroUsize;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use thiserror::Error;
use url::Url;

use crate::{
    DEFAULT_FETCH_TIMEOUT_MS, DEFAULT_REGEX_FLAGS, Diagnostic, ExtractionMatch,
    ExtractionMatchMetadata, ExtractionRequest, ExtractionSpec, NormalizationOptions,
    OutputOptions, Range, RuntimeOptions, SelectionSpec, SelectorQuery, SliceBoundary,
    SlicePatternSpec, SliceSpec, SourceRequest, ValueSpec, WhitespaceMode, extract,
};

/// Frozen interop profile identifier for FFHN v1.
pub const FFHN_V1_INTEROP_PROFILE: &str = "ffhn-htmlcut-v1";
/// Frozen schema name for the FFHN extraction plan.
pub const FFHN_PLAN_SCHEMA_NAME: &str = "htmlcut.ffhn_plan";
/// Frozen schema name for the FFHN extraction result.
pub const FFHN_RESULT_SCHEMA_NAME: &str = "htmlcut.ffhn_result";
/// Frozen schema name for the FFHN extraction error.
pub const FFHN_ERROR_SCHEMA_NAME: &str = "htmlcut.ffhn_error";
/// Frozen schema version for the FFHN extraction plan.
pub const FFHN_PLAN_SCHEMA_VERSION: u32 = 1;
/// Frozen schema version for the FFHN extraction result.
pub const FFHN_RESULT_SCHEMA_VERSION: u32 = 1;
/// Frozen schema version for the FFHN extraction error.
pub const FFHN_ERROR_SCHEMA_VERSION: u32 = 1;

/// Error returned when FFHN interop values or schema identities are invalid.
#[derive(Debug, Error)]
pub enum FfhnInteropError {
    /// The input could not be serialized into stable JSON.
    #[error("could not serialize FFHN interop JSON: {0}")]
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
    #[error("successful FFHN extraction results must report at least one candidate")]
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
    #[error("successful FFHN extraction results must not contain error-level diagnostics")]
    ErrorDiagnosticsInSuccess,
    /// Result metadata did not describe the same strategy kind as the top-level result.
    #[error(
        "selected match metadata kind {metadata_kind:?} does not match result strategy kind {strategy_kind:?}"
    )]
    MetadataKindMismatch {
        /// Strategy kind declared by the result.
        strategy_kind: FfhnStrategyKind,
        /// Strategy kind declared by the selected match metadata.
        metadata_kind: FfhnStrategyKind,
    },
    /// The FFHN source label was blank.
    #[error("source label must not be empty")]
    EmptySourceLabel,
}

/// Strategy family available to FFHN v1.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FfhnStrategyKind {
    /// Select one DOM node candidate set with a CSS selector.
    CssSelector,
    /// Slice raw source text between two explicit boundaries.
    DelimiterPair,
}

/// Delimiter matching mode for delimiter-pair extraction.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FfhnDelimiterMode {
    /// Treat `start` and `end` as literal substrings.
    Literal,
    /// Treat `start` and `end` as regular expressions.
    Regex,
}

/// Supported regex flags for FFHN delimiter-pair extraction.
#[derive(
    Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
#[serde(rename_all = "snake_case")]
pub enum FfhnRegexFlag {
    /// Match without ASCII case sensitivity.
    CaseInsensitive,
    /// Let `^` and `$` operate on line boundaries.
    MultiLine,
    /// Let `.` match newline characters.
    DotMatchesNewLine,
    /// Swap regex greediness defaults.
    SwapGreed,
    /// Ignore pattern whitespace.
    IgnoreWhitespace,
}

/// FFHN v1 strategy union.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FfhnPlanStrategy {
    /// Select candidates with a CSS selector.
    CssSelector {
        /// Non-empty CSS selector text.
        selector: SelectorQuery,
    },
    /// Slice raw source text between two explicit boundaries.
    DelimiterPair {
        /// Non-empty start boundary.
        start: SliceBoundary,
        /// Non-empty end boundary.
        end: SliceBoundary,
        /// Literal or regex boundary semantics.
        mode: FfhnDelimiterMode,
        /// Whether the selected payload includes the matched start boundary.
        include_start: bool,
        /// Whether the selected payload includes the matched end boundary.
        include_end: bool,
        /// Regex flags when `mode = "regex"`.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        flags: Vec<FfhnRegexFlag>,
    },
}

impl FfhnPlanStrategy {
    /// Builds a CSS-selector FFHN plan strategy.
    pub fn css_selector(selector: SelectorQuery) -> Self {
        Self::CssSelector { selector }
    }

    /// Builds a delimiter-pair FFHN plan strategy.
    pub fn delimiter_pair(
        start: SliceBoundary,
        end: SliceBoundary,
        mode: FfhnDelimiterMode,
        include_start: bool,
        include_end: bool,
        flags: Vec<FfhnRegexFlag>,
    ) -> Self {
        Self::DelimiterPair {
            start,
            end,
            mode,
            include_start,
            include_end,
            flags,
        }
    }

    /// Returns the stable strategy kind for this plan strategy.
    pub const fn kind(&self) -> FfhnStrategyKind {
        match self {
            Self::CssSelector { .. } => FfhnStrategyKind::CssSelector,
            Self::DelimiterPair { .. } => FfhnStrategyKind::DelimiterPair,
        }
    }

    fn validate(&self) -> Result<(), FfhnInteropError> {
        if let Self::DelimiterPair {
            mode: FfhnDelimiterMode::Literal,
            flags,
            ..
        } = self
            && !flags.is_empty()
        {
            return Err(FfhnInteropError::LiteralDelimiterFlags);
        }

        Ok(())
    }
}

/// Candidate selection mode available to FFHN v1.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FfhnSelectionMode {
    /// Require exactly one candidate.
    Single,
    /// Select the first candidate.
    First,
    /// Select one explicit 1-based candidate.
    Nth,
}

/// FFHN v1 candidate selection contract.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum FfhnSelection {
    /// Require exactly one candidate.
    Single,
    /// Select the first candidate.
    First,
    /// Select one explicit 1-based candidate.
    Nth {
        /// 1-based selected candidate index.
        index: NonZeroUsize,
    },
}

impl FfhnSelection {
    /// Builds the exact-one selection mode.
    pub const fn single() -> Self {
        Self::Single
    }

    /// Builds the first-candidate selection mode.
    pub const fn first() -> Self {
        Self::First
    }

    /// Builds the explicit nth-candidate selection mode.
    pub const fn nth(index: NonZeroUsize) -> Self {
        Self::Nth { index }
    }

    /// Returns the stable selection mode for this selection contract.
    pub const fn mode(&self) -> FfhnSelectionMode {
        match self {
            Self::Single => FfhnSelectionMode::Single,
            Self::First => FfhnSelectionMode::First,
            Self::Nth { .. } => FfhnSelectionMode::Nth,
        }
    }
}

/// Output payload kind available to FFHN v1.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FfhnOutputKind {
    /// Extract normalized or preserved text.
    Text,
    /// Extract inner HTML.
    InnerHtml,
    /// Extract outer HTML.
    OuterHtml,
}

/// FFHN v1 output selection object.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct FfhnOutput {
    /// Requested output payload kind.
    pub kind: FfhnOutputKind,
}

impl FfhnOutput {
    /// Builds one FFHN output selection.
    pub const fn new(kind: FfhnOutputKind) -> Self {
        Self { kind }
    }
}

/// Whitespace normalization mode available to FFHN v1.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FfhnWhitespaceMode {
    /// Preserve source whitespace.
    Preserve,
    /// Normalize whitespace for human-readable text.
    Normalize,
}

/// FFHN v1 extraction-time normalization contract.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct FfhnNormalization {
    /// Whitespace handling for text generation inside HTMLCut.
    pub whitespace: FfhnWhitespaceMode,
    /// Whether relative URLs should be rewritten against the effective base URL.
    pub rewrite_urls: bool,
}

impl FfhnNormalization {
    /// Builds one FFHN normalization contract.
    pub const fn new(whitespace: FfhnWhitespaceMode, rewrite_urls: bool) -> Self {
        Self {
            whitespace,
            rewrite_urls,
        }
    }
}

/// Versioned FFHN extraction plan owned by HTMLCut.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct FfhnPlan {
    /// Frozen schema identity.
    pub schema_name: String,
    /// Frozen schema version.
    pub schema_version: u32,
    /// Frozen interoperability profile identifier.
    pub interop_profile: String,
    /// Strategy requested by FFHN.
    pub strategy: FfhnPlanStrategy,
    /// Candidate selection requested by FFHN.
    pub selection: FfhnSelection,
    /// Output payload requested by FFHN.
    pub output: FfhnOutput,
    /// Extraction-time normalization requested by FFHN.
    pub normalization: FfhnNormalization,
    /// Reserved extension object ignored by v1 consumers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<BTreeMap<String, Value>>,
}

impl FfhnPlan {
    /// Builds one FFHN extraction plan with the frozen v1 schema identity.
    pub fn new(
        strategy: FfhnPlanStrategy,
        selection: FfhnSelection,
        output: FfhnOutput,
        normalization: FfhnNormalization,
    ) -> Self {
        Self {
            schema_name: FFHN_PLAN_SCHEMA_NAME.to_owned(),
            schema_version: FFHN_PLAN_SCHEMA_VERSION,
            interop_profile: FFHN_V1_INTEROP_PROFILE.to_owned(),
            strategy,
            selection,
            output,
            normalization,
            extensions: None,
        }
    }

    /// Validates the schema identity and semantic invariants for this plan.
    pub fn validate(&self) -> Result<(), FfhnInteropError> {
        validate_schema_identity(
            &self.schema_name,
            FFHN_PLAN_SCHEMA_NAME,
            self.schema_version,
            FFHN_PLAN_SCHEMA_VERSION,
            &self.interop_profile,
            FFHN_V1_INTEROP_PROFILE,
        )?;
        self.strategy.validate()
    }

    /// Serializes this plan with the frozen stable JSON profile.
    pub fn stable_json(&self) -> Result<String, FfhnInteropError> {
        self.validate()?;
        stable_json_v1(self)
    }

    /// Computes the SHA-256 digest of this exact plan document.
    pub fn digest_sha256(&self) -> Result<String, FfhnInteropError> {
        self.validate()?;
        digest_stable_json(self)
    }
}

/// FFHN-owned HTML source input handed into HTMLCut after fetch and decode.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FfhnSourceInput {
    /// Logical label for the fetched HTML source, typically the FFHN target id.
    pub label: String,
    /// Decoded HTML handed into HTMLCut.
    pub html: String,
    /// Input base URL that HTMLCut should use before any document `<base href>`.
    pub input_base_url: Option<Url>,
}

impl FfhnSourceInput {
    /// Builds a new FFHN source input from a logical label and decoded HTML.
    pub fn new(
        label: impl Into<String>,
        html: impl Into<String>,
    ) -> Result<Self, FfhnInteropError> {
        let label = label.into();
        if label.trim().is_empty() {
            return Err(FfhnInteropError::EmptySourceLabel);
        }

        Ok(Self {
            label,
            html: html.into(),
            input_base_url: None,
        })
    }

    /// Sets the input base URL used by HTMLCut before resolving any document base href.
    pub fn with_input_base_url(mut self, input_base_url: Url) -> Self {
        self.input_base_url = Some(input_base_url);
        self
    }

    /// Builds the existing generic HTMLCut source request for this FFHN source input.
    pub fn to_source_request(&self) -> SourceRequest {
        let mut source = SourceRequest::memory(self.label.clone(), self.html.clone());
        if let Some(base_url) = &self.input_base_url {
            source = source.with_base_url(base_url.clone());
        }

        source
    }

    /// Consumes this FFHN source input and produces the generic HTMLCut source request.
    pub fn into_source_request(self) -> SourceRequest {
        let mut source = SourceRequest::memory(self.label, self.html);
        if let Some(base_url) = self.input_base_url {
            source = source.with_base_url(base_url);
        }

        source
    }
}

/// Source summary carried in one successful FFHN extraction result.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct FfhnResultSource {
    /// Base URL supplied by FFHN before document parsing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_base_url: Option<Url>,
    /// Effective base URL after document `<base href>` resolution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effective_base_url: Option<Url>,
    /// Parsed document title when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_title: Option<String>,
}

/// Execution summary fields shared by one successful FFHN extraction result.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct FfhnResultExecution {
    /// Digest of the exact validated FFHN plan document.
    pub plan_digest_sha256: String,
    /// Executed strategy kind.
    pub strategy_kind: FfhnStrategyKind,
    /// Executed selection mode.
    pub selection_mode: FfhnSelectionMode,
    /// Total candidate count before selection.
    pub candidate_count: usize,
}

impl FfhnResultExecution {
    /// Builds one FFHN extraction execution summary.
    pub fn new(
        plan_digest_sha256: impl Into<String>,
        strategy_kind: FfhnStrategyKind,
        selection_mode: FfhnSelectionMode,
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

/// Typed metadata for one selected FFHN match.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FfhnSelectedMatchMetadata {
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

impl FfhnSelectedMatchMetadata {
    /// Returns the strategy kind described by this metadata.
    pub const fn kind(&self) -> FfhnStrategyKind {
        match self {
            Self::CssSelector { .. } => FfhnStrategyKind::CssSelector,
            Self::DelimiterPair { .. } => FfhnStrategyKind::DelimiterPair,
        }
    }

    fn candidate_count(&self) -> usize {
        match self {
            Self::CssSelector {
                candidate_count, ..
            }
            | Self::DelimiterPair {
                candidate_count, ..
            } => *candidate_count,
        }
    }

    fn candidate_index(&self) -> NonZeroUsize {
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

/// One selected FFHN match returned on successful extraction.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct FfhnSelectedMatch {
    /// Selected 1-based candidate ordinal within all discovered candidates.
    pub candidate_index: NonZeroUsize,
    /// Exact output payload kind requested by FFHN.
    pub value_kind: FfhnOutputKind,
    /// Exact output payload returned for the selected candidate.
    pub value: String,
    /// Text handed from HTMLCut into FFHN compare-time canonicalization.
    pub comparison_input_text: String,
    /// Inner HTML for the selected match when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inner_html: Option<String>,
    /// Outer HTML for the selected match when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outer_html: Option<String>,
    /// Stable typed metadata for the selected match.
    pub metadata: FfhnSelectedMatchMetadata,
}

/// Successful FFHN-facing extraction result owned by HTMLCut.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct FfhnResult {
    /// Frozen schema identity.
    pub schema_name: String,
    /// Frozen schema version.
    pub schema_version: u32,
    /// Frozen interoperability profile identifier.
    pub interop_profile: String,
    /// Digest of the exact validated FFHN plan document.
    pub plan_digest_sha256: String,
    /// Digest of this exact result document with this field omitted.
    pub result_digest_sha256: String,
    /// Executed strategy kind.
    pub strategy_kind: FfhnStrategyKind,
    /// Executed selection mode.
    pub selection_mode: FfhnSelectionMode,
    /// Total candidate count before selection.
    pub candidate_count: usize,
    /// Source summary relevant to FFHN.
    pub source: FfhnResultSource,
    /// Exactly one selected match.
    pub selected_match: FfhnSelectedMatch,
    /// Warning and informational diagnostics emitted during extraction.
    pub diagnostics: Vec<Diagnostic>,
    /// Reserved extension object ignored by v1 consumers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<BTreeMap<String, Value>>,
}

impl FfhnResult {
    /// Builds one successful FFHN extraction result with the frozen v1 schema identity.
    pub fn new(
        execution: FfhnResultExecution,
        source: FfhnResultSource,
        selected_match: FfhnSelectedMatch,
        diagnostics: Vec<Diagnostic>,
    ) -> Self {
        Self {
            schema_name: FFHN_RESULT_SCHEMA_NAME.to_owned(),
            schema_version: FFHN_RESULT_SCHEMA_VERSION,
            interop_profile: FFHN_V1_INTEROP_PROFILE.to_owned(),
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
    pub fn validate(&self) -> Result<(), FfhnInteropError> {
        validate_schema_identity(
            &self.schema_name,
            FFHN_RESULT_SCHEMA_NAME,
            self.schema_version,
            FFHN_RESULT_SCHEMA_VERSION,
            &self.interop_profile,
            FFHN_V1_INTEROP_PROFILE,
        )?;

        if self.candidate_count == 0 {
            return Err(FfhnInteropError::ZeroCandidateCount);
        }

        let selected = self.selected_match.candidate_index.get();
        if selected > self.candidate_count {
            return Err(FfhnInteropError::SelectedCandidateOutOfRange {
                selected,
                candidate_count: self.candidate_count,
            });
        }

        if self.selected_match.metadata.kind() != self.strategy_kind {
            return Err(FfhnInteropError::MetadataKindMismatch {
                strategy_kind: self.strategy_kind,
                metadata_kind: self.selected_match.metadata.kind(),
            });
        }

        if self.selected_match.metadata.candidate_count() != self.candidate_count
            || self.selected_match.metadata.candidate_index() != self.selected_match.candidate_index
        {
            return Err(FfhnInteropError::SelectedCandidateOutOfRange {
                selected,
                candidate_count: self.candidate_count,
            });
        }

        if self
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.level == crate::DiagnosticLevel::Error)
        {
            return Err(FfhnInteropError::ErrorDiagnosticsInSuccess);
        }

        Ok(())
    }

    /// Serializes this result with the frozen stable JSON profile.
    pub fn stable_json(&self) -> Result<String, FfhnInteropError> {
        self.validate()?;
        stable_json_v1(self)
    }

    /// Computes the SHA-256 digest of this result with `result_digest_sha256` omitted.
    pub fn digest_sha256(&self) -> Result<String, FfhnInteropError> {
        self.validate()?;
        digest_stable_json_omitting_field(self, "result_digest_sha256")
    }

    /// Computes and stores `result_digest_sha256` on this result.
    pub fn with_computed_digest(mut self) -> Result<Self, FfhnInteropError> {
        self.result_digest_sha256 = self.digest_sha256()?;
        Ok(self)
    }
}

/// Frozen FFHN-facing extraction error vocabulary owned by HTMLCut.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FfhnErrorCode {
    /// The plan was invalid for the frozen interop profile.
    PlanInvalid,
    /// No candidate matched the requested strategy and selection.
    NoMatch,
    /// Exact-one selection saw multiple candidates.
    AmbiguousMatch,
    /// An internal failure occurred inside HTMLCut.
    InternalError,
}

/// Typed FFHN-facing extraction error document owned by HTMLCut.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct FfhnError {
    /// Frozen schema identity.
    pub schema_name: String,
    /// Frozen schema version.
    pub schema_version: u32,
    /// Frozen interoperability profile identifier.
    pub interop_profile: String,
    /// Digest of the exact validated FFHN plan document.
    pub plan_digest_sha256: String,
    /// Digest of this exact error document with this field omitted.
    pub error_digest_sha256: String,
    /// Frozen FFHN-facing error code.
    pub error_code: FfhnErrorCode,
    /// Human-readable error summary.
    pub message: String,
    /// Strategy kind when one was known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strategy_kind: Option<FfhnStrategyKind>,
    /// Machine-readable detail object.
    pub details: BTreeMap<String, Value>,
    /// Underlying HTMLCut diagnostics that produced this error.
    pub diagnostics: Vec<Diagnostic>,
    /// Reserved extension object ignored by v1 consumers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<BTreeMap<String, Value>>,
}

impl FfhnError {
    /// Builds one FFHN-facing extraction error with the frozen v1 schema identity.
    pub fn new(
        plan_digest_sha256: impl Into<String>,
        error_code: FfhnErrorCode,
        message: impl Into<String>,
        strategy_kind: Option<FfhnStrategyKind>,
        details: BTreeMap<String, Value>,
        diagnostics: Vec<Diagnostic>,
    ) -> Self {
        Self {
            schema_name: FFHN_ERROR_SCHEMA_NAME.to_owned(),
            schema_version: FFHN_ERROR_SCHEMA_VERSION,
            interop_profile: FFHN_V1_INTEROP_PROFILE.to_owned(),
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
    pub fn validate(&self) -> Result<(), FfhnInteropError> {
        validate_schema_identity(
            &self.schema_name,
            FFHN_ERROR_SCHEMA_NAME,
            self.schema_version,
            FFHN_ERROR_SCHEMA_VERSION,
            &self.interop_profile,
            FFHN_V1_INTEROP_PROFILE,
        )
    }

    /// Serializes this error with the frozen stable JSON profile.
    pub fn stable_json(&self) -> Result<String, FfhnInteropError> {
        self.validate()?;
        stable_json_v1(self)
    }

    /// Computes the SHA-256 digest of this error with `error_digest_sha256` omitted.
    pub fn digest_sha256(&self) -> Result<String, FfhnInteropError> {
        self.validate()?;
        digest_stable_json_omitting_field(self, "error_digest_sha256")
    }

    /// Computes and stores `error_digest_sha256` on this error document.
    pub fn with_computed_digest(mut self) -> Result<Self, FfhnInteropError> {
        self.error_digest_sha256 = self.digest_sha256()?;
        Ok(self)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ProjectedStructuredMatch {
    candidate_index: NonZeroUsize,
    selected_html: String,
    comparison_input_text: String,
    inner_html: String,
    outer_html: String,
    metadata: FfhnSelectedMatchMetadata,
}

/// Validates one FFHN plan and returns a typed FFHN interop error on failure.
pub fn validate_ffhn_plan(plan: &FfhnPlan) -> Result<(), Box<FfhnError>> {
    let plan_digest_sha256 = exact_plan_digest_sha256(plan);
    plan.validate()
        .map_err(|error| Box::new(plan_invalid_error(plan, &plan_digest_sha256, error)))
}

/// Executes one FFHN plan directly against FFHN-owned in-memory HTML input.
pub fn execute_ffhn_plan(
    source: &FfhnSourceInput,
    plan: &FfhnPlan,
) -> Result<FfhnResult, Box<FfhnError>> {
    let plan_digest_sha256 = exact_plan_digest_sha256(plan);
    plan.validate()
        .map_err(|error| Box::new(plan_invalid_error(plan, &plan_digest_sha256, error)))?;

    let request = compile_ffhn_request(source, plan);
    let runtime = ffhn_runtime_options(source);
    let extraction = extract(&request, &runtime);

    if !extraction.ok {
        return Err(Box::new(core_execution_error(
            plan,
            &plan_digest_sha256,
            &extraction.diagnostics,
        )));
    }

    let strategy_kind = plan.strategy.kind();
    let Some(selected) = extraction.matches.first() else {
        let mut details = BTreeMap::new();
        details.insert(
            "match_count".to_owned(),
            Value::from(extraction.matches.len() as u64),
        );
        return Err(Box::new(internal_adapter_error(
            &plan_digest_sha256,
            Some(strategy_kind),
            "successful extraction did not produce a selected match",
            details,
            extraction.diagnostics,
        )));
    };

    if extraction.matches.len() != 1 {
        let mut details = BTreeMap::new();
        details.insert(
            "match_count".to_owned(),
            Value::from(extraction.matches.len() as u64),
        );
        details.insert(
            "candidate_count".to_owned(),
            Value::from(extraction.stats.candidate_count as u64),
        );
        return Err(Box::new(internal_adapter_error(
            &plan_digest_sha256,
            Some(strategy_kind),
            "successful FFHN execution must produce exactly one selected match",
            details,
            extraction.diagnostics,
        )));
    }

    let projected = project_structured_match(
        selected,
        strategy_kind,
        &plan_digest_sha256,
        &extraction.diagnostics,
    )?;
    let source_summary = FfhnResultSource {
        input_base_url: source.input_base_url.clone(),
        effective_base_url: parse_optional_url(
            extraction.source.effective_base_url.as_deref(),
            &plan_digest_sha256,
            strategy_kind,
            "effective_base_url",
            &extraction.diagnostics,
        )?,
        document_title: extraction.document_title.clone(),
    };
    let selected_match = FfhnSelectedMatch {
        candidate_index: projected.candidate_index,
        value_kind: plan.output.kind,
        value: match plan.output.kind {
            FfhnOutputKind::Text => projected.comparison_input_text.clone(),
            FfhnOutputKind::InnerHtml => projected.selected_html.clone(),
            FfhnOutputKind::OuterHtml => projected.outer_html.clone(),
        },
        comparison_input_text: projected.comparison_input_text,
        inner_html: Some(projected.inner_html),
        outer_html: Some(projected.outer_html),
        metadata: projected.metadata,
    };
    let execution = FfhnResultExecution::new(
        plan_digest_sha256,
        strategy_kind,
        plan.selection.mode(),
        extraction.stats.candidate_count,
    );

    Ok(finalize_ffhn_result(FfhnResult::new(
        execution,
        source_summary,
        selected_match,
        extraction.diagnostics,
    )))
}

/// Serializes one value with the frozen `stable_json_v1` profile.
pub fn stable_json_v1<T: Serialize>(value: &T) -> Result<String, FfhnInteropError> {
    let value = serde_json::to_value(value)?;
    let mut output = String::new();
    write_stable_json_value(&value, &mut output)?;
    Ok(output)
}

fn digest_stable_json<T: Serialize>(value: &T) -> Result<String, FfhnInteropError> {
    let stable_json = stable_json_v1(value)?;
    Ok(sha256_hex(stable_json.as_bytes()))
}

fn exact_plan_digest_sha256(plan: &FfhnPlan) -> String {
    digest_stable_json(plan).expect("FFHN plans should always serialize to stable JSON")
}

fn ffhn_runtime_options(source: &FfhnSourceInput) -> RuntimeOptions {
    RuntimeOptions {
        max_bytes: source.html.len(),
        fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
    }
}

fn compile_ffhn_request(source: &FfhnSourceInput, plan: &FfhnPlan) -> ExtractionRequest {
    let extraction = match &plan.strategy {
        FfhnPlanStrategy::CssSelector { selector } => ExtractionSpec::selector(selector.clone()),
        FfhnPlanStrategy::DelimiterPair {
            start,
            end,
            mode,
            include_start,
            include_end,
            flags,
        } => ExtractionSpec::slice(SliceSpec {
            pattern: match mode {
                FfhnDelimiterMode::Literal => SlicePatternSpec::literal(start.clone(), end.clone()),
                FfhnDelimiterMode::Regex => {
                    SlicePatternSpec::regex(start.clone(), end.clone(), compile_regex_flags(flags))
                }
            },
            include_start: *include_start,
            include_end: *include_end,
        }),
    }
    .with_selection(compile_selection(&plan.selection))
    .with_value(ValueSpec::Structured);

    let mut request = ExtractionRequest::new(source.to_source_request(), extraction);
    request.normalization = NormalizationOptions {
        whitespace: match plan.normalization.whitespace {
            FfhnWhitespaceMode::Preserve => WhitespaceMode::Preserve,
            FfhnWhitespaceMode::Normalize => WhitespaceMode::Normalize,
        },
        rewrite_urls: plan.normalization.rewrite_urls,
    };
    request.output = OutputOptions {
        include_source_text: false,
        include_html: false,
        include_text: false,
        ..OutputOptions::default()
    };
    request
}

fn compile_selection(selection: &FfhnSelection) -> SelectionSpec {
    match selection {
        FfhnSelection::Single => SelectionSpec::single(),
        FfhnSelection::First => SelectionSpec::First,
        FfhnSelection::Nth { index } => SelectionSpec::nth(*index),
    }
}

fn compile_regex_flags(flags: &[FfhnRegexFlag]) -> String {
    let mut compiled = DEFAULT_REGEX_FLAGS.to_owned();
    for flag in flags {
        compiled.push(match flag {
            FfhnRegexFlag::CaseInsensitive => 'i',
            FfhnRegexFlag::MultiLine => 'm',
            FfhnRegexFlag::DotMatchesNewLine => 's',
            FfhnRegexFlag::SwapGreed => 'U',
            FfhnRegexFlag::IgnoreWhitespace => 'x',
        });
    }
    compiled
}

fn project_structured_match(
    matched: &ExtractionMatch,
    strategy_kind: FfhnStrategyKind,
    plan_digest_sha256: &str,
    diagnostics: &[Diagnostic],
) -> Result<ProjectedStructuredMatch, Box<FfhnError>> {
    let structured = matched.value.as_object().ok_or_else(|| {
        let mut details = BTreeMap::new();
        details.insert("value_type".to_owned(), Value::from("structured"));
        Box::new(internal_adapter_error(
            plan_digest_sha256,
            Some(strategy_kind),
            "FFHN execution expected a structured core match payload",
            details,
            diagnostics.to_vec(),
        ))
    })?;

    match &matched.metadata {
        ExtractionMatchMetadata::Selector(metadata) => {
            let candidate_index = non_zero_candidate_index(
                metadata.candidate_index,
                plan_digest_sha256,
                strategy_kind,
                diagnostics,
            )?;
            Ok(ProjectedStructuredMatch {
                candidate_index,
                selected_html: required_string_field(
                    structured,
                    "html",
                    plan_digest_sha256,
                    strategy_kind,
                    diagnostics,
                )?,
                comparison_input_text: required_string_field(
                    structured,
                    "text",
                    plan_digest_sha256,
                    strategy_kind,
                    diagnostics,
                )?,
                inner_html: required_string_field(
                    structured,
                    "html",
                    plan_digest_sha256,
                    strategy_kind,
                    diagnostics,
                )?,
                outer_html: required_string_field(
                    structured,
                    "outerHtml",
                    plan_digest_sha256,
                    strategy_kind,
                    diagnostics,
                )?,
                metadata: FfhnSelectedMatchMetadata::CssSelector {
                    candidate_count: metadata.candidate_count,
                    candidate_index,
                    path: metadata.path.clone(),
                    tag_name: metadata.tag_name.clone(),
                },
            })
        }
        ExtractionMatchMetadata::DelimiterPair(metadata) => {
            let candidate_index = non_zero_candidate_index(
                metadata.candidate_index,
                plan_digest_sha256,
                strategy_kind,
                diagnostics,
            )?;
            Ok(ProjectedStructuredMatch {
                candidate_index,
                selected_html: required_string_field(
                    structured,
                    "html",
                    plan_digest_sha256,
                    strategy_kind,
                    diagnostics,
                )?,
                comparison_input_text: required_string_field(
                    structured,
                    "text",
                    plan_digest_sha256,
                    strategy_kind,
                    diagnostics,
                )?,
                inner_html: required_string_field(
                    structured,
                    "innerHtml",
                    plan_digest_sha256,
                    strategy_kind,
                    diagnostics,
                )?,
                outer_html: required_string_field(
                    structured,
                    "outerHtml",
                    plan_digest_sha256,
                    strategy_kind,
                    diagnostics,
                )?,
                metadata: FfhnSelectedMatchMetadata::DelimiterPair {
                    candidate_count: metadata.candidate_count,
                    candidate_index,
                    selected_range: metadata.selected_range.clone(),
                    inner_range: metadata.inner_range.clone(),
                    outer_range: metadata.outer_range.clone(),
                    include_start: metadata.include_start,
                    include_end: metadata.include_end,
                },
            })
        }
    }
}

fn required_string_field(
    structured: &serde_json::Map<String, Value>,
    field: &'static str,
    plan_digest_sha256: &str,
    strategy_kind: FfhnStrategyKind,
    diagnostics: &[Diagnostic],
) -> Result<String, Box<FfhnError>> {
    structured
        .get(field)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| {
            let mut details = BTreeMap::new();
            details.insert("field".to_owned(), Value::from(field));
            Box::new(internal_adapter_error(
                plan_digest_sha256,
                Some(strategy_kind),
                format!("FFHN execution could not project structured field {field:?}"),
                details,
                diagnostics.to_vec(),
            ))
        })
}

fn non_zero_candidate_index(
    candidate_index: usize,
    plan_digest_sha256: &str,
    strategy_kind: FfhnStrategyKind,
    diagnostics: &[Diagnostic],
) -> Result<NonZeroUsize, Box<FfhnError>> {
    NonZeroUsize::new(candidate_index).ok_or_else(|| {
        let mut details = BTreeMap::new();
        details.insert(
            "candidate_index".to_owned(),
            Value::from(candidate_index as u64),
        );
        Box::new(internal_adapter_error(
            plan_digest_sha256,
            Some(strategy_kind),
            "FFHN execution received an invalid zero candidate index from core metadata",
            details,
            diagnostics.to_vec(),
        ))
    })
}

fn parse_optional_url(
    value: Option<&str>,
    plan_digest_sha256: &str,
    strategy_kind: FfhnStrategyKind,
    field: &'static str,
    diagnostics: &[Diagnostic],
) -> Result<Option<Url>, Box<FfhnError>> {
    value
        .map(|value| {
            Url::parse(value).map_err(|_| {
                let mut details = BTreeMap::new();
                details.insert("field".to_owned(), Value::from(field));
                details.insert("value".to_owned(), Value::from(value));
                Box::new(internal_adapter_error(
                    plan_digest_sha256,
                    Some(strategy_kind),
                    format!("FFHN execution produced an invalid URL in {field}"),
                    details,
                    diagnostics.to_vec(),
                ))
            })
        })
        .transpose()
}

fn plan_invalid_error(
    plan: &FfhnPlan,
    plan_digest_sha256: &str,
    error: FfhnInteropError,
) -> FfhnError {
    let mut details = BTreeMap::new();
    details.insert("interop_error".to_owned(), Value::from(error.to_string()));
    finalize_ffhn_error(FfhnError::new(
        plan_digest_sha256.to_owned(),
        FfhnErrorCode::PlanInvalid,
        error.to_string(),
        Some(plan.strategy.kind()),
        details,
        Vec::new(),
    ))
}

fn core_execution_error(
    plan: &FfhnPlan,
    plan_digest_sha256: &str,
    diagnostics: &[Diagnostic],
) -> FfhnError {
    let Some(primary) = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.level == crate::DiagnosticLevel::Error)
    else {
        return internal_adapter_error(
            plan_digest_sha256,
            Some(plan.strategy.kind()),
            "FFHN execution failed without an error diagnostic",
            BTreeMap::new(),
            diagnostics.to_vec(),
        );
    };

    let error_code = match primary.code.as_str() {
        "UNSUPPORTED_SPEC_VERSION" | "INVALID_SELECTOR" | "INVALID_SLICE_PATTERN" => {
            FfhnErrorCode::PlanInvalid
        }
        "NO_MATCH" | "MATCH_INDEX_OUT_OF_RANGE" => FfhnErrorCode::NoMatch,
        "AMBIGUOUS_MATCH" => FfhnErrorCode::AmbiguousMatch,
        _ => FfhnErrorCode::InternalError,
    };
    let mut details = BTreeMap::new();
    details.insert(
        "core_diagnostic_code".to_owned(),
        Value::from(primary.code.clone()),
    );
    if let Some(core_details) = &primary.details {
        details.insert("core_details".to_owned(), core_details.clone());
    }

    finalize_ffhn_error(FfhnError::new(
        plan_digest_sha256.to_owned(),
        error_code,
        primary.message.clone(),
        Some(plan.strategy.kind()),
        details,
        diagnostics.to_vec(),
    ))
}

fn internal_adapter_error(
    plan_digest_sha256: &str,
    strategy_kind: Option<FfhnStrategyKind>,
    message: impl Into<String>,
    details: BTreeMap<String, Value>,
    diagnostics: Vec<Diagnostic>,
) -> FfhnError {
    finalize_ffhn_error(FfhnError::new(
        plan_digest_sha256.to_owned(),
        FfhnErrorCode::InternalError,
        message,
        strategy_kind,
        details,
        diagnostics,
    ))
}

fn finalize_ffhn_result(result: FfhnResult) -> FfhnResult {
    result
        .with_computed_digest()
        .expect("FFHN results should always validate and serialize")
}

fn finalize_ffhn_error(error: FfhnError) -> FfhnError {
    error
        .with_computed_digest()
        .expect("FFHN errors should always validate and serialize")
}

fn digest_stable_json_omitting_field<T: Serialize>(
    value: &T,
    field: &str,
) -> Result<String, FfhnInteropError> {
    let mut value = serde_json::to_value(value)?;
    if let Value::Object(map) = &mut value {
        map.remove(field);
    }

    let mut output = String::new();
    write_stable_json_value(&value, &mut output)?;
    Ok(sha256_hex(output.as_bytes()))
}

fn write_stable_json_value(value: &Value, output: &mut String) -> Result<(), FfhnInteropError> {
    match value {
        Value::Null => output.push_str("null"),
        Value::Bool(value) => output.push_str(if *value { "true" } else { "false" }),
        Value::Number(value) => output.push_str(&value.to_string()),
        Value::String(value) => output.push_str(&serde_json::to_string(value)?),
        Value::Array(values) => {
            output.push('[');
            for (index, value) in values.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                write_stable_json_value(value, output)?;
            }
            output.push(']');
        }
        Value::Object(map) => {
            output.push('{');
            let mut entries = map.iter().collect::<Vec<_>>();
            entries.sort_unstable_by(|(left, _), (right, _)| left.cmp(right));
            for (index, (key, value)) in entries.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                output.push_str(&serde_json::to_string(key)?);
                output.push(':');
                write_stable_json_value(value, output)?;
            }
            output.push('}');
        }
    }

    Ok(())
}

fn validate_schema_identity(
    schema_name: &str,
    expected_schema_name: &'static str,
    schema_version: u32,
    expected_schema_version: u32,
    interop_profile: &str,
    expected_interop_profile: &'static str,
) -> Result<(), FfhnInteropError> {
    if schema_name != expected_schema_name {
        return Err(FfhnInteropError::InvalidIdentity {
            field: "schema_name",
            expected: expected_schema_name,
            received: schema_name.to_owned(),
        });
    }

    if schema_version != expected_schema_version {
        return Err(FfhnInteropError::InvalidVersion {
            field: "schema_version",
            expected: expected_schema_version,
            received: schema_version,
        });
    }

    if interop_profile != expected_interop_profile {
        return Err(FfhnInteropError::InvalidIdentity {
            field: "interop_profile",
            expected: expected_interop_profile,
            received: interop_profile.to_owned(),
        });
    }

    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(digest.len() * 2);
    for byte in digest {
        let _ = write!(output, "{byte:02x}");
    }
    output
}
