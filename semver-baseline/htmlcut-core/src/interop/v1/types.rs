use std::collections::BTreeMap;
use std::num::NonZeroUsize;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use url::Url;

use crate::{Diagnostic, SelectorQuery, SliceBoundary, SourceRequest, result::Range};

use super::{
    stable_json::{digest_stable_json, digest_stable_json_omitting_field},
    validate_schema_identity,
};

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
        strategy_kind: StrategyKind,
        /// Strategy kind declared by the selected match metadata.
        metadata_kind: StrategyKind,
    },
    /// The source label was blank.
    #[error("source label must not be empty")]
    EmptySourceLabel,
}

/// Strategy family available in v1.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StrategyKind {
    /// Select one DOM node candidate set with a CSS selector.
    CssSelector,
    /// Slice raw source text between two explicit boundaries.
    DelimiterPair,
}

/// Delimiter matching mode for delimiter-pair extraction.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DelimiterMode {
    /// Treat `start` and `end` as literal substrings.
    Literal,
    /// Treat `start` and `end` as regular expressions.
    Regex,
}

/// Supported regex flags for delimiter-pair extraction.
#[derive(
    Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
#[serde(rename_all = "snake_case")]
pub enum RegexFlag {
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

/// v1 strategy union.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PlanStrategy {
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
        mode: DelimiterMode,
        /// Whether the selected payload includes the matched start boundary.
        include_start: bool,
        /// Whether the selected payload includes the matched end boundary.
        include_end: bool,
        /// Regex flags when `mode = "regex"`.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        flags: Vec<RegexFlag>,
    },
}

impl PlanStrategy {
    /// Builds a CSS-selector plan strategy.
    pub fn css_selector(selector: SelectorQuery) -> Self {
        Self::CssSelector { selector }
    }

    /// Builds a delimiter-pair plan strategy.
    pub fn delimiter_pair(
        start: SliceBoundary,
        end: SliceBoundary,
        mode: DelimiterMode,
        include_start: bool,
        include_end: bool,
        flags: Vec<RegexFlag>,
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
    pub const fn kind(&self) -> StrategyKind {
        match self {
            Self::CssSelector { .. } => StrategyKind::CssSelector,
            Self::DelimiterPair { .. } => StrategyKind::DelimiterPair,
        }
    }

    pub(super) fn validate(&self) -> Result<(), ContractError> {
        if let Self::DelimiterPair {
            mode: DelimiterMode::Literal,
            flags,
            ..
        } = self
            && !flags.is_empty()
        {
            return Err(ContractError::LiteralDelimiterFlags);
        }

        Ok(())
    }
}

/// Candidate selection mode available in v1.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SelectionMode {
    /// Require exactly one candidate.
    Single,
    /// Select the first candidate.
    First,
    /// Select one explicit 1-based candidate.
    Nth,
}

/// v1 candidate selection contract.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum Selection {
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

impl Selection {
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
    pub const fn mode(&self) -> SelectionMode {
        match self {
            Self::Single => SelectionMode::Single,
            Self::First => SelectionMode::First,
            Self::Nth { .. } => SelectionMode::Nth,
        }
    }
}

/// Output payload kind available in v1.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OutputKind {
    /// Extract normalized or preserved text.
    Text,
    /// Extract inner HTML.
    InnerHtml,
    /// Extract outer HTML.
    OuterHtml,
}

/// v1 output selection object.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct Output {
    /// Requested output payload kind.
    pub kind: OutputKind,
}

impl Output {
    /// Builds one output selection.
    pub const fn new(kind: OutputKind) -> Self {
        Self { kind }
    }
}

/// Whitespace normalization mode available in v1.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TextWhitespace {
    /// Preserve source whitespace.
    Preserve,
    /// Normalize whitespace for human-readable text.
    Normalize,
}

/// v1 extraction-time normalization contract.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct Normalization {
    /// Whitespace handling for text generation.
    pub whitespace: TextWhitespace,
    /// Whether relative URLs should be rewritten against the effective base URL.
    pub rewrite_urls: bool,
}

impl Normalization {
    /// Builds one normalization contract.
    pub const fn new(whitespace: TextWhitespace, rewrite_urls: bool) -> Self {
        Self {
            whitespace,
            rewrite_urls,
        }
    }
}

/// Versioned extraction plan owned by HTMLCut.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct Plan {
    /// Frozen schema identity.
    pub schema_name: String,
    /// Frozen schema version.
    pub schema_version: u32,
    /// Frozen interoperability profile identifier.
    pub interop_profile: String,
    /// Requested extraction strategy.
    pub strategy: PlanStrategy,
    /// Requested candidate selection.
    pub selection: Selection,
    /// Requested output payload.
    pub output: Output,
    /// Extraction-time normalization.
    pub normalization: Normalization,
    /// Reserved extension object ignored by v1 consumers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<BTreeMap<String, Value>>,
}

impl Plan {
    /// Builds one extraction plan with the frozen v1 schema identity.
    pub fn new(
        strategy: PlanStrategy,
        selection: Selection,
        output: Output,
        normalization: Normalization,
    ) -> Self {
        Self {
            schema_name: PLAN_SCHEMA_NAME.to_owned(),
            schema_version: PLAN_SCHEMA_VERSION,
            interop_profile: INTEROP_V1_PROFILE.to_owned(),
            strategy,
            selection,
            output,
            normalization,
            extensions: None,
        }
    }

    /// Validates the schema identity and semantic invariants for this plan.
    pub fn validate(&self) -> Result<(), ContractError> {
        validate_schema_identity(
            &self.schema_name,
            PLAN_SCHEMA_NAME,
            self.schema_version,
            PLAN_SCHEMA_VERSION,
            &self.interop_profile,
            INTEROP_V1_PROFILE,
        )?;
        self.strategy.validate()
    }

    /// Serializes this plan with the frozen stable JSON profile.
    pub fn stable_json(&self) -> Result<String, ContractError> {
        self.validate()?;
        super::stable_json::stable_json_v1(self)
    }

    /// Computes the SHA-256 digest of this exact plan document.
    pub fn digest_sha256(&self) -> Result<String, ContractError> {
        self.validate()?;
        digest_stable_json(self)
    }
}

/// HTML source input handed into HTMLCut after fetch and decode.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HtmlInput {
    /// Logical label for the fetched HTML source.
    pub label: String,
    /// Decoded HTML.
    pub html: String,
    /// Input base URL that HTMLCut should use before any document `<base href>`.
    pub input_base_url: Option<Url>,
}

impl HtmlInput {
    /// Builds a new HTML input from a logical label and decoded HTML.
    pub fn new(label: impl Into<String>, html: impl Into<String>) -> Result<Self, ContractError> {
        let label = label.into();
        if label.trim().is_empty() {
            return Err(ContractError::EmptySourceLabel);
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

    /// Builds the existing generic HTMLCut source request for this HTML input.
    pub fn to_source_request(&self) -> SourceRequest {
        let mut source = SourceRequest::memory(self.label.clone(), self.html.clone());
        if let Some(base_url) = &self.input_base_url {
            source = source.with_base_url(base_url.clone());
        }

        source
    }

    /// Consumes this HTML input and produces the generic HTMLCut source request.
    pub fn into_source_request(self) -> SourceRequest {
        let mut source = SourceRequest::memory(self.label, self.html);
        if let Some(base_url) = self.input_base_url {
            source = source.with_base_url(base_url);
        }

        source
    }
}

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
            .any(|diagnostic| diagnostic.level == crate::DiagnosticLevel::Error)
        {
            return Err(ContractError::ErrorDiagnosticsInSuccess);
        }

        Ok(())
    }

    /// Serializes this result with the frozen stable JSON profile.
    pub fn stable_json(&self) -> Result<String, ContractError> {
        self.validate()?;
        super::stable_json::stable_json_v1(self)
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
        super::stable_json::stable_json_v1(self)
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
