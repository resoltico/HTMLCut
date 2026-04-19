//! Versioned extraction interop contracts (v1).

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
    DEFAULT_FETCH_TIMEOUT_MS, DEFAULT_REGEX_FLAGS, Diagnostic, DiagnosticCode, ExtractionRequest,
    ExtractionSpec, NormalizationOptions, OutputOptions, RuntimeOptions, SelectionSpec,
    SelectorQuery, SliceBoundary, SlicePatternSpec, SliceSpec, SourceRequest, ValueSpec,
    WhitespaceMode, extract,
    result::{ExtractionMatch, ExtractionMatchMetadata, Range},
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

    fn validate(&self) -> Result<(), ContractError> {
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
        stable_json_v1(self)
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
        stable_json_v1(self)
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
        stable_json_v1(self)
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

#[derive(Clone, Debug, PartialEq, Eq)]
struct ProjectedStructuredMatch {
    candidate_index: NonZeroUsize,
    selected_html: String,
    comparison_input_text: String,
    inner_html: String,
    outer_html: String,
    metadata: SelectedMatchMetadata,
}

/// Validates one plan and returns a typed interop error on failure.
pub fn validate_plan(plan: &Plan) -> Result<(), Box<InteropError>> {
    let plan_digest_sha256 = exact_plan_digest_sha256(plan);
    plan.validate()
        .map_err(|error| Box::new(plan_invalid_error(plan, &plan_digest_sha256, error)))
}

/// Executes one plan directly against in-memory HTML input.
pub fn execute_plan(source: &HtmlInput, plan: &Plan) -> Result<InteropResult, Box<InteropError>> {
    let plan_digest_sha256 = exact_plan_digest_sha256(plan);
    plan.validate()
        .map_err(|error| Box::new(plan_invalid_error(plan, &plan_digest_sha256, error)))?;

    let request = compile_request(source, plan);
    let runtime = runtime_options(source);
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
            "successful execution must produce exactly one selected match",
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
    let source_summary = ResultSource {
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
    let selected_match = SelectedMatch {
        candidate_index: projected.candidate_index,
        value_kind: plan.output.kind,
        value: match plan.output.kind {
            OutputKind::Text => projected.comparison_input_text.clone(),
            OutputKind::InnerHtml => projected.selected_html.clone(),
            OutputKind::OuterHtml => projected.outer_html.clone(),
        },
        comparison_input_text: projected.comparison_input_text,
        inner_html: Some(projected.inner_html),
        outer_html: Some(projected.outer_html),
        metadata: projected.metadata,
    };
    let execution = ResultExecution::new(
        plan_digest_sha256,
        strategy_kind,
        plan.selection.mode(),
        extraction.stats.candidate_count,
    );

    Ok(finalize_result(InteropResult::new(
        execution,
        source_summary,
        selected_match,
        extraction.diagnostics,
    )))
}

/// Serializes one value with the frozen `stable_json_v1` profile.
pub fn stable_json_v1<T: Serialize>(value: &T) -> Result<String, ContractError> {
    let value = serde_json::to_value(value)?;
    let mut output = String::new();
    write_stable_json_value(&value, &mut output)?;
    Ok(output)
}

fn digest_stable_json<T: Serialize>(value: &T) -> Result<String, ContractError> {
    let stable_json = stable_json_v1(value)?;
    Ok(sha256_hex(stable_json.as_bytes()))
}

fn exact_plan_digest_sha256(plan: &Plan) -> String {
    digest_stable_json(plan).expect("plans should always serialize to stable JSON")
}

fn runtime_options(source: &HtmlInput) -> RuntimeOptions {
    RuntimeOptions {
        max_bytes: source.html.len(),
        fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
        fetch_preflight: crate::FetchPreflightMode::HeadFirst,
    }
}

fn compile_request(source: &HtmlInput, plan: &Plan) -> ExtractionRequest {
    let extraction = match &plan.strategy {
        PlanStrategy::CssSelector { selector } => ExtractionSpec::selector(selector.clone()),
        PlanStrategy::DelimiterPair {
            start,
            end,
            mode,
            include_start,
            include_end,
            flags,
        } => ExtractionSpec::slice(SliceSpec {
            pattern: match mode {
                DelimiterMode::Literal => SlicePatternSpec::literal(start.clone(), end.clone()),
                DelimiterMode::Regex => {
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
            TextWhitespace::Preserve => WhitespaceMode::Preserve,
            TextWhitespace::Normalize => WhitespaceMode::Normalize,
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

fn compile_selection(selection: &Selection) -> SelectionSpec {
    match selection {
        Selection::Single => SelectionSpec::single(),
        Selection::First => SelectionSpec::First,
        Selection::Nth { index } => SelectionSpec::nth(*index),
    }
}

fn compile_regex_flags(flags: &[RegexFlag]) -> String {
    let mut compiled = DEFAULT_REGEX_FLAGS.to_owned();
    for flag in flags {
        compiled.push(match flag {
            RegexFlag::CaseInsensitive => 'i',
            RegexFlag::MultiLine => 'm',
            RegexFlag::DotMatchesNewLine => 's',
            RegexFlag::SwapGreed => 'U',
            RegexFlag::IgnoreWhitespace => 'x',
        });
    }
    compiled
}

fn project_structured_match(
    matched: &ExtractionMatch,
    strategy_kind: StrategyKind,
    plan_digest_sha256: &str,
    diagnostics: &[Diagnostic],
) -> Result<ProjectedStructuredMatch, Box<InteropError>> {
    let structured = matched.value.as_object().ok_or_else(|| {
        let mut details = BTreeMap::new();
        details.insert("value_type".to_owned(), Value::from("structured"));
        Box::new(internal_adapter_error(
            plan_digest_sha256,
            Some(strategy_kind),
            "execution expected a structured core match payload",
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
                metadata: SelectedMatchMetadata::CssSelector {
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
                metadata: SelectedMatchMetadata::DelimiterPair {
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
    strategy_kind: StrategyKind,
    diagnostics: &[Diagnostic],
) -> Result<String, Box<InteropError>> {
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
                format!("execution could not project structured field {field:?}"),
                details,
                diagnostics.to_vec(),
            ))
        })
}

fn non_zero_candidate_index(
    candidate_index: usize,
    plan_digest_sha256: &str,
    strategy_kind: StrategyKind,
    diagnostics: &[Diagnostic],
) -> Result<NonZeroUsize, Box<InteropError>> {
    NonZeroUsize::new(candidate_index).ok_or_else(|| {
        let mut details = BTreeMap::new();
        details.insert(
            "candidate_index".to_owned(),
            Value::from(candidate_index as u64),
        );
        Box::new(internal_adapter_error(
            plan_digest_sha256,
            Some(strategy_kind),
            "execution received an invalid zero candidate index from core metadata",
            details,
            diagnostics.to_vec(),
        ))
    })
}

fn parse_optional_url(
    value: Option<&str>,
    plan_digest_sha256: &str,
    strategy_kind: StrategyKind,
    field: &'static str,
    diagnostics: &[Diagnostic],
) -> Result<Option<Url>, Box<InteropError>> {
    value
        .map(|value| {
            Url::parse(value).map_err(|_| {
                let mut details = BTreeMap::new();
                details.insert("field".to_owned(), Value::from(field));
                details.insert("value".to_owned(), Value::from(value));
                Box::new(internal_adapter_error(
                    plan_digest_sha256,
                    Some(strategy_kind),
                    format!("execution produced an invalid URL in {field}"),
                    details,
                    diagnostics.to_vec(),
                ))
            })
        })
        .transpose()
}

fn plan_invalid_error(plan: &Plan, plan_digest_sha256: &str, error: ContractError) -> InteropError {
    let mut details = BTreeMap::new();
    details.insert("contract_error".to_owned(), Value::from(error.to_string()));
    finalize_error(InteropError::new(
        plan_digest_sha256.to_owned(),
        ErrorCode::PlanInvalid,
        error.to_string(),
        Some(plan.strategy.kind()),
        details,
        Vec::new(),
    ))
}

fn core_execution_error(
    plan: &Plan,
    plan_digest_sha256: &str,
    diagnostics: &[Diagnostic],
) -> InteropError {
    let Some(primary) = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.level == crate::DiagnosticLevel::Error)
    else {
        return internal_adapter_error(
            plan_digest_sha256,
            Some(plan.strategy.kind()),
            "execution failed without an error diagnostic",
            BTreeMap::new(),
            diagnostics.to_vec(),
        );
    };

    let error_code = match primary.code.parse::<DiagnosticCode>() {
        Ok(
            DiagnosticCode::UnsupportedSpecVersion
            | DiagnosticCode::InvalidSelector
            | DiagnosticCode::InvalidSlicePattern
            | DiagnosticCode::InvalidRequest,
        ) => ErrorCode::PlanInvalid,
        Ok(DiagnosticCode::NoMatch | DiagnosticCode::MatchIndexOutOfRange) => ErrorCode::NoMatch,
        Ok(DiagnosticCode::AmbiguousMatch) => ErrorCode::AmbiguousMatch,
        _ => ErrorCode::InternalError,
    };
    let mut details = BTreeMap::new();
    details.insert(
        "core_diagnostic_code".to_owned(),
        Value::from(primary.code.clone()),
    );
    if let Some(core_details) = &primary.details {
        details.insert("core_details".to_owned(), core_details.clone());
    }

    finalize_error(InteropError::new(
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
    strategy_kind: Option<StrategyKind>,
    message: impl Into<String>,
    details: BTreeMap<String, Value>,
    diagnostics: Vec<Diagnostic>,
) -> InteropError {
    finalize_error(InteropError::new(
        plan_digest_sha256.to_owned(),
        ErrorCode::InternalError,
        message,
        strategy_kind,
        details,
        diagnostics,
    ))
}

fn finalize_result(result: InteropResult) -> InteropResult {
    result
        .with_computed_digest()
        .expect("results should always validate and serialize")
}

fn finalize_error(error: InteropError) -> InteropError {
    error
        .with_computed_digest()
        .expect("errors should always validate and serialize")
}

fn digest_stable_json_omitting_field<T: Serialize>(
    value: &T,
    field: &str,
) -> Result<String, ContractError> {
    let mut value = serde_json::to_value(value)?;
    if let Value::Object(map) = &mut value {
        map.remove(field);
    }

    let mut output = String::new();
    write_stable_json_value(&value, &mut output)?;
    Ok(sha256_hex(output.as_bytes()))
}

fn write_stable_json_value(value: &Value, output: &mut String) -> Result<(), ContractError> {
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
            entries.sort_unstable_by_key(|(key, _)| *key);
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

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(digest.len() * 2);
    for byte in digest {
        let _ = write!(output, "{byte:02x}");
    }
    output
}
