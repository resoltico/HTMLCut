use std::collections::BTreeMap;
use std::num::NonZeroUsize;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use url::Url;

use crate::{SelectorQuery, SliceBoundary, SourceRequest};

use super::super::stable_json::digest_stable_json;
use super::shared::{
    ContractError, INTEROP_V1_PROFILE, PLAN_SCHEMA_NAME, PLAN_SCHEMA_VERSION,
    validate_schema_identity,
};

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
    /// Select every candidate.
    All,
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
    /// Select every candidate.
    All,
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

    /// Builds the all-candidates selection mode.
    pub const fn all() -> Self {
        Self::All
    }

    /// Returns the stable selection mode for this selection contract.
    pub const fn mode(&self) -> SelectionMode {
        match self {
            Self::Single => SelectionMode::Single,
            Self::First => SelectionMode::First,
            Self::Nth { .. } => SelectionMode::Nth,
            Self::All => SelectionMode::All,
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
    /// Schema identity.
    pub schema_name: String,
    /// Schema version.
    pub schema_version: u32,
    /// Interoperability profile identifier.
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
    /// Builds one extraction plan with the v1 schema identity.
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

    /// Serializes this plan with the stable JSON profile.
    pub fn stable_json(&self) -> Result<String, ContractError> {
        self.validate()?;
        super::super::stable_json::stable_json_v1(self)
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
