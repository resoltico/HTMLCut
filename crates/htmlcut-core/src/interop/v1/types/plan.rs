use std::collections::BTreeMap;
use std::fmt;
use std::num::NonZeroUsize;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::super::stable_json::digest_stable_json;
use super::shared::{
    ContractError, INTEROP_V1_PROFILE, PLAN_SCHEMA_NAME, PLAN_SCHEMA_VERSION,
    validate_schema_identity,
};
use crate::{AttributeName, HttpUrl};

macro_rules! non_empty_string_type {
    ($name:ident, $error_variant:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(
            Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord, Hash,
        )]
        #[serde(try_from = "String")]
        #[schemars(with = "String")]
        pub struct $name(String);

        impl $name {
            /// Validates and stores one non-empty value.
            pub fn new(value: impl Into<String>) -> Result<Self, ContractError> {
                let value = value.into();
                if value.trim().is_empty() {
                    return Err(ContractError::$error_variant);
                }

                Ok(Self(value))
            }

            /// Returns the stored text.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(self.as_str())
            }
        }

        impl TryFrom<String> for $name {
            type Error = ContractError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }
    };
}

non_empty_string_type!(
    CssSelectorText,
    EmptyCssSelector,
    "Validated CSS selector text owned by htmlcut-v1."
);
non_empty_string_type!(
    DelimiterBoundaryText,
    EmptyDelimiterBoundary,
    "Validated delimiter boundary text owned by htmlcut-v1."
);

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

/// Which matched boundaries become part of the selected delimiter-pair fragment.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DelimiterBoundaryRetention {
    /// Exclude both matched boundaries from the selected fragment.
    ExcludeBoth,
    /// Include only the matched start boundary.
    IncludeStart,
    /// Include only the matched end boundary.
    IncludeEnd,
    /// Include both matched boundaries.
    IncludeBoth,
}

impl DelimiterBoundaryRetention {
    /// Builds one retention mode from explicit start/end inclusion flags.
    pub const fn from_flags(include_start: bool, include_end: bool) -> Self {
        match (include_start, include_end) {
            (false, false) => Self::ExcludeBoth,
            (true, false) => Self::IncludeStart,
            (false, true) => Self::IncludeEnd,
            (true, true) => Self::IncludeBoth,
        }
    }

    /// Returns whether the matched start boundary is retained.
    pub const fn includes_start(self) -> bool {
        matches!(self, Self::IncludeStart | Self::IncludeBoth)
    }

    /// Returns whether the matched end boundary is retained.
    pub const fn includes_end(self) -> bool {
        matches!(self, Self::IncludeEnd | Self::IncludeBoth)
    }
}

/// v1 strategy union.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PlanStrategy {
    /// Select candidates with a CSS selector.
    CssSelector {
        /// Non-empty CSS selector text.
        selector: CssSelectorText,
    },
    /// Slice raw source text between two explicit boundaries.
    DelimiterPair {
        /// Non-empty start boundary.
        start: DelimiterBoundaryText,
        /// Non-empty end boundary.
        end: DelimiterBoundaryText,
        /// Literal or regex boundary semantics.
        mode: DelimiterMode,
        /// Which matched boundaries become part of the selected payload.
        boundary_retention: DelimiterBoundaryRetention,
        /// Regex flags when `mode = "regex"`.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        flags: Vec<RegexFlag>,
    },
}

impl PlanStrategy {
    /// Builds a CSS-selector plan strategy.
    pub fn css_selector(selector: CssSelectorText) -> Self {
        Self::CssSelector { selector }
    }

    /// Builds a delimiter-pair plan strategy.
    pub fn delimiter_pair(
        start: DelimiterBoundaryText,
        end: DelimiterBoundaryText,
        mode: DelimiterMode,
        boundary_retention: DelimiterBoundaryRetention,
        flags: Vec<RegexFlag>,
    ) -> Self {
        Self::DelimiterPair {
            start,
            end,
            mode,
            boundary_retention,
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
#[serde(deny_unknown_fields)]
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
    /// Extract true inner HTML.
    InnerHtml,
    /// Extract outer HTML.
    OuterHtml,
    /// Extract the exact selected slice fragment.
    SelectedHtml,
    /// Extract one named attribute.
    Attribute,
    /// Extract the full structured HTMLCut payload.
    Structured,
}

impl OutputKind {
    /// Returns the stable string form of this output kind.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::InnerHtml => "inner_html",
            Self::OuterHtml => "outer_html",
            Self::SelectedHtml => "selected_html",
            Self::Attribute => "attribute",
            Self::Structured => "structured",
        }
    }
}

impl fmt::Display for OutputKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// v1 output selection object.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Output {
    /// Extract normalized or preserved text.
    Text,
    /// Extract true inner HTML.
    InnerHtml,
    /// Extract outer HTML.
    OuterHtml,
    /// Extract the exact selected slice fragment.
    SelectedHtml,
    /// Extract one named attribute.
    Attribute {
        /// Attribute name to recover from the selected element or fragment root.
        name: AttributeName,
    },
    /// Extract the full structured HTMLCut payload.
    Structured,
}

impl Output {
    /// Builds a text output selection.
    pub const fn text() -> Self {
        Self::Text
    }

    /// Builds an inner-HTML output selection.
    pub const fn inner_html() -> Self {
        Self::InnerHtml
    }

    /// Builds an outer-HTML output selection.
    pub const fn outer_html() -> Self {
        Self::OuterHtml
    }

    /// Builds a selected-slice-HTML output selection.
    pub const fn selected_html() -> Self {
        Self::SelectedHtml
    }

    /// Builds an attribute output selection.
    pub const fn attribute(name: AttributeName) -> Self {
        Self::Attribute { name }
    }

    /// Builds a structured output selection.
    pub const fn structured() -> Self {
        Self::Structured
    }

    /// Returns the stable output kind for this output contract.
    pub const fn kind(&self) -> OutputKind {
        match self {
            Self::Text => OutputKind::Text,
            Self::InnerHtml => OutputKind::InnerHtml,
            Self::OuterHtml => OutputKind::OuterHtml,
            Self::SelectedHtml => OutputKind::SelectedHtml,
            Self::Attribute { .. } => OutputKind::Attribute,
            Self::Structured => OutputKind::Structured,
        }
    }

    pub(super) fn validate_for_strategy(
        &self,
        strategy_kind: StrategyKind,
    ) -> Result<(), ContractError> {
        if matches!(
            (strategy_kind, self),
            (StrategyKind::CssSelector, Self::SelectedHtml)
        ) {
            return Err(ContractError::UnsupportedOutputForStrategy {
                strategy_kind,
                output_kind: self.kind(),
            });
        }

        Ok(())
    }
}

/// Text-whitespace mode available in v1.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TextWhitespace {
    /// Preserve the rendered text layout after HTML-aware rendering.
    Rendered,
    /// Normalize whitespace for human-readable text.
    Normalize,
}

/// v1 rendering contract.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Rendering {
    /// Whitespace handling for text generation.
    pub whitespace: TextWhitespace,
    /// Whether relative URLs should be rewritten against the effective base URL.
    pub rewrite_urls: bool,
}

impl Rendering {
    /// Builds one rendering contract.
    pub const fn new(whitespace: TextWhitespace, rewrite_urls: bool) -> Self {
        Self {
            whitespace,
            rewrite_urls,
        }
    }
}

/// Versioned extraction plan owned by HTMLCut.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
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
    /// Rendering policy for extracted values.
    pub rendering: Rendering,
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
        rendering: Rendering,
    ) -> Self {
        Self {
            schema_name: PLAN_SCHEMA_NAME.to_owned(),
            schema_version: PLAN_SCHEMA_VERSION,
            interop_profile: INTEROP_V1_PROFILE.to_owned(),
            strategy,
            selection,
            output,
            rendering,
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
        self.strategy.validate()?;
        self.output.validate_for_strategy(self.strategy.kind())
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
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct HtmlInput {
    /// Logical label for the fetched HTML source.
    pub label: String,
    /// Decoded HTML.
    pub html: String,
    /// Input base URL that HTMLCut should use before any document `<base href>`.
    pub input_base_url: Option<HttpUrl>,
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
    pub fn with_input_base_url(mut self, input_base_url: HttpUrl) -> Self {
        self.input_base_url = Some(input_base_url);
        self
    }

    /// Computes the canonical SHA-256 identity for this complete input and one plan.
    ///
    /// The identity binds every field on this `HtmlInput`, including the decoded HTML bytes and
    /// optional input base URL, the complete plan, and
    /// [`HTMLCUT_EXTRACTION_SEMANTICS_VERSION`](super::super::HTMLCUT_EXTRACTION_SEMANTICS_VERSION).
    /// It is the identity a downstream consumer persists when it needs to determine whether a
    /// fixed extraction would have the same projection and diagnostics.
    pub fn extraction_identity_sha256(&self, plan: &Plan) -> Result<String, ContractError> {
        digest_stable_json(&ExtractionIdentity {
            html_input: self,
            plan,
            htmlcut_extraction_semantics_version:
                super::super::HTMLCUT_EXTRACTION_SEMANTICS_VERSION,
        })
    }
}

#[derive(Serialize)]
struct ExtractionIdentity<'a> {
    html_input: &'a HtmlInput,
    plan: &'a Plan,
    htmlcut_extraction_semantics_version: u32,
}
