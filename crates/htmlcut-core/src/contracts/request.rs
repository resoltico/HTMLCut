use std::fmt;
use std::num::NonZeroUsize;
use std::path::PathBuf;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use url::Url;

use super::constants::{
    CORE_SPEC_VERSION, DEFAULT_FETCH_TIMEOUT_MS, DEFAULT_INSPECTION_SAMPLE_LIMIT, DEFAULT_MAX_BYTES,
};
use super::{
    default_fetch_timeout_ms, default_inspection_sample_limit, default_max_bytes,
    default_preview_chars_non_zero, default_regex_flags, default_spec_version, default_true,
};

/// Physical source kind used to load HTML content.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SourceKind {
    /// The source was loaded from an HTTP or HTTPS URL.
    Url,
    /// The source was loaded from the local filesystem.
    File,
    /// The source was loaded from standard input.
    Stdin,
    /// The source text was provided directly in-memory.
    Memory,
}

/// High-level extraction family used by a request.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ExtractionStrategy {
    /// Select DOM nodes with a CSS selector.
    Selector,
    /// Slice raw source text with literal or regex boundaries.
    Slice,
}

/// Value shape extracted from each surviving match.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ValueType {
    /// Return normalized or preserved plain text.
    Text,
    /// Return inner HTML.
    InnerHtml,
    /// Return outer HTML.
    OuterHtml,
    /// Return one named attribute value.
    Attribute,
    /// Return a structured JSON payload with metadata.
    Structured,
}

/// Whitespace treatment applied to text-like values.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WhitespaceMode {
    /// Preserve source whitespace.
    #[default]
    Preserve,
    /// Collapse inline whitespace for human-readable text.
    Normalize,
}

/// Boundary-matching mode for slice extraction.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PatternMode {
    /// Treat `from` and `to` as literal substrings.
    #[default]
    Literal,
    /// Treat `from` and `to` as regular expressions.
    Regex,
}

/// URL-fetch preflight policy used before loading a remote source body.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum FetchPreflightMode {
    /// Probe remote sources with `HEAD` before issuing `GET`.
    #[default]
    HeadFirst,
    /// Skip the `HEAD` probe and load the source directly with `GET`.
    GetOnly,
}

/// Mode-correct pattern contract for slice extraction.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(tag = "mode", rename_all = "lowercase")]
pub enum SlicePatternSpec {
    /// Treat `from` and `to` as literal substrings.
    Literal {
        /// Starting boundary pattern.
        from: SliceBoundary,
        /// Ending boundary pattern.
        to: SliceBoundary,
    },
    /// Treat `from` and `to` as regular expressions.
    Regex {
        /// Starting boundary pattern.
        from: SliceBoundary,
        /// Ending boundary pattern.
        to: SliceBoundary,
        #[serde(default = "default_regex_flags")]
        /// Regex flags applied to both boundaries.
        flags: String,
    },
}

impl SlicePatternSpec {
    /// Builds a literal slice pattern.
    pub const fn literal(from: SliceBoundary, to: SliceBoundary) -> Self {
        Self::Literal { from, to }
    }

    /// Builds a regex slice pattern.
    pub fn regex(from: SliceBoundary, to: SliceBoundary, flags: impl Into<String>) -> Self {
        Self::Regex {
            from,
            to,
            flags: flags.into(),
        }
    }

    /// Returns the slice boundary interpretation mode.
    pub const fn mode(&self) -> PatternMode {
        match self {
            Self::Literal { .. } => PatternMode::Literal,
            Self::Regex { .. } => PatternMode::Regex,
        }
    }

    /// Returns the starting boundary pattern.
    pub const fn from(&self) -> &SliceBoundary {
        match self {
            Self::Literal { from, .. } | Self::Regex { from, .. } => from,
        }
    }

    /// Returns the ending boundary pattern.
    pub const fn to(&self) -> &SliceBoundary {
        match self {
            Self::Literal { to, .. } | Self::Regex { to, .. } => to,
        }
    }

    /// Returns regex flags when regex mode is used.
    pub fn flags(&self) -> Option<&str> {
        match self {
            Self::Literal { .. } => None,
            Self::Regex { flags, .. } => Some(flags.as_str()),
        }
    }
}

/// Error returned when a contract value fails eager validation.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum ContractValueError {
    /// A required string was blank.
    #[error("{field} must not be empty")]
    Empty {
        /// Human-readable field label used in the validation message.
        field: &'static str,
    },
}

macro_rules! non_empty_string_type {
    ($name:ident, $field:literal, $doc:literal) => {
        #[doc = $doc]
        #[derive(
            Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord, Hash,
        )]
        #[serde(try_from = "String")]
        #[schemars(with = "String")]
        pub struct $name(String);

        impl $name {
            /// Validates and stores a non-empty string value.
            pub fn new(value: impl Into<String>) -> Result<Self, ContractValueError> {
                let value = value.into();
                if value.trim().is_empty() {
                    return Err(ContractValueError::Empty { field: $field });
                }

                Ok(Self(value))
            }

            /// Returns the stored string value.
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
            type Error = ContractValueError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }
    };
}

non_empty_string_type!(
    SelectorQuery,
    "selector",
    "Validated CSS selector text used by selector extraction."
);
non_empty_string_type!(
    AttributeName,
    "attribute name",
    "Validated attribute name used by attribute extraction."
);
non_empty_string_type!(
    SliceBoundary,
    "slice boundary",
    "Validated boundary pattern used by slice extraction."
);

/// Match-retention policy for one extraction request.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum SelectionSpec {
    /// Require exactly one candidate and fail on ambiguity.
    Single,
    /// Keep the first candidate only.
    #[default]
    First,
    /// Keep one 1-based candidate by index.
    Nth {
        /// 1-based index of the retained candidate.
        index: NonZeroUsize,
    },
    /// Keep every discovered candidate.
    All,
}

impl SelectionSpec {
    /// Builds an exact-one selection spec.
    pub const fn single() -> Self {
        Self::Single
    }

    /// Builds an nth-selection spec.
    pub const fn nth(index: NonZeroUsize) -> Self {
        Self::Nth { index }
    }

    /// Returns the optional retained index for nth-selection mode.
    pub const fn index(&self) -> Option<NonZeroUsize> {
        match self {
            Self::Nth { index } => Some(*index),
            Self::Single | Self::First | Self::All => None,
        }
    }
}

/// Value shape requested for each surviving match.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ValueSpec {
    /// Return normalized or preserved plain text.
    #[default]
    Text,
    /// Return inner HTML.
    InnerHtml,
    /// Return outer HTML.
    OuterHtml,
    /// Return one named attribute value.
    Attribute {
        /// Attribute name to extract from each retained match.
        name: AttributeName,
    },
    /// Return a structured JSON payload with metadata.
    Structured,
}

impl ValueSpec {
    /// Returns the stable value kind for this extraction mode.
    pub const fn value_type(&self) -> ValueType {
        match self {
            Self::Text => ValueType::Text,
            Self::InnerHtml => ValueType::InnerHtml,
            Self::OuterHtml => ValueType::OuterHtml,
            Self::Attribute { .. } => ValueType::Attribute,
            Self::Structured => ValueType::Structured,
        }
    }

    /// Returns the configured attribute name when attribute extraction is requested.
    pub fn attribute_name(&self) -> Option<&AttributeName> {
        match self {
            Self::Attribute { name } => Some(name),
            Self::Text | Self::InnerHtml | Self::OuterHtml | Self::Structured => None,
        }
    }
}

/// Literal or regex slice configuration for slice extraction.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct SliceSpec {
    #[serde(flatten)]
    /// Literal or regex boundary pattern configuration.
    pub pattern: SlicePatternSpec,
    /// Whether to include the matched start boundary in the selected fragment.
    #[serde(default)]
    pub include_start: bool,
    /// Whether to include the matched end boundary in the selected fragment.
    #[serde(default)]
    pub include_end: bool,
}

impl SliceSpec {
    /// Creates a slice definition with literal defaults that exclude both boundaries.
    pub fn new(from: SliceBoundary, to: SliceBoundary) -> Self {
        Self {
            pattern: SlicePatternSpec::literal(from, to),
            include_start: false,
            include_end: false,
        }
    }

    /// Creates a regex slice definition with defaults that exclude both boundaries.
    pub fn regex(from: SliceBoundary, to: SliceBoundary, flags: impl Into<String>) -> Self {
        Self {
            pattern: SlicePatternSpec::regex(from, to, flags),
            include_start: false,
            include_end: false,
        }
    }

    /// Sets whether the selected fragment should include the matched boundaries.
    pub fn with_boundary_inclusion(mut self, include_start: bool, include_end: bool) -> Self {
        self.include_start = include_start;
        self.include_end = include_end;
        self
    }

    /// Returns the slice boundary interpretation mode.
    pub const fn mode(&self) -> PatternMode {
        self.pattern.mode()
    }

    /// Returns the starting boundary pattern.
    pub const fn from(&self) -> &SliceBoundary {
        self.pattern.from()
    }

    /// Returns the ending boundary pattern.
    pub const fn to(&self) -> &SliceBoundary {
        self.pattern.to()
    }

    /// Returns regex flags when regex mode is used.
    pub fn flags(&self) -> Option<&str> {
        self.pattern.flags()
    }
}

/// Core extraction strategy plus value-shaping configuration.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ExtractionSpec {
    /// Select DOM nodes with a CSS selector.
    Selector {
        /// CSS selector used to find candidate nodes.
        selector: SelectorQuery,
        /// Candidate-retention policy.
        #[serde(default)]
        selection: SelectionSpec,
        /// Value shape produced for each retained candidate.
        #[serde(default)]
        value: ValueSpec,
    },
    /// Slice raw source text with literal or regex boundaries.
    Slice {
        /// Slice boundary configuration.
        #[serde(flatten)]
        slice: SliceSpec,
        /// Candidate-retention policy.
        #[serde(default)]
        selection: SelectionSpec,
        /// Value shape produced for each retained candidate.
        #[serde(default)]
        value: ValueSpec,
    },
}

impl ExtractionSpec {
    /// Builds a selector extraction spec with default selection and value behavior.
    pub fn selector(selector: SelectorQuery) -> Self {
        Self::Selector {
            selector,
            selection: SelectionSpec::default(),
            value: ValueSpec::default(),
        }
    }

    /// Builds a slice extraction spec with default selection and value behavior.
    pub fn slice(slice: SliceSpec) -> Self {
        Self::Slice {
            slice,
            selection: SelectionSpec::default(),
            value: ValueSpec::default(),
        }
    }

    /// Replaces the selection policy for this extraction spec.
    pub fn with_selection(self, selection: SelectionSpec) -> Self {
        match self {
            Self::Selector {
                selector, value, ..
            } => Self::Selector {
                selector,
                selection,
                value,
            },
            Self::Slice { slice, value, .. } => Self::Slice {
                slice,
                selection,
                value,
            },
        }
    }

    /// Replaces the extracted value shape for this extraction spec.
    pub fn with_value(self, value: ValueSpec) -> Self {
        match self {
            Self::Selector {
                selector,
                selection,
                ..
            } => Self::Selector {
                selector,
                selection,
                value,
            },
            Self::Slice {
                slice, selection, ..
            } => Self::Slice {
                slice,
                selection,
                value,
            },
        }
    }

    /// Returns the extraction family represented by this spec.
    pub const fn strategy(&self) -> ExtractionStrategy {
        match self {
            Self::Selector { .. } => ExtractionStrategy::Selector,
            Self::Slice { .. } => ExtractionStrategy::Slice,
        }
    }

    /// Returns the selection policy for this extraction spec.
    pub const fn selection(&self) -> &SelectionSpec {
        match self {
            Self::Selector { selection, .. } | Self::Slice { selection, .. } => selection,
        }
    }

    /// Returns the value shape for this extraction spec.
    pub const fn value(&self) -> &ValueSpec {
        match self {
            Self::Selector { value, .. } | Self::Slice { value, .. } => value,
        }
    }

    /// Returns the selector query when selector extraction is requested.
    pub fn selector_query(&self) -> Option<&SelectorQuery> {
        match self {
            Self::Selector { selector, .. } => Some(selector),
            Self::Slice { .. } => None,
        }
    }

    /// Returns the slice definition when slice extraction is requested.
    pub const fn slice_spec(&self) -> Option<&SliceSpec> {
        match self {
            Self::Slice { slice, .. } => Some(slice),
            Self::Selector { .. } => None,
        }
    }
}

/// Output normalization rules applied after raw value extraction.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct NormalizationOptions {
    #[serde(default)]
    /// Whitespace treatment for text-like outputs.
    pub whitespace: WhitespaceMode,
    #[serde(default)]
    /// Whether relative URLs should be rewritten against the effective base URL.
    pub rewrite_urls: bool,
}

impl Default for NormalizationOptions {
    fn default() -> Self {
        Self {
            whitespace: WhitespaceMode::Preserve,
            rewrite_urls: false,
        }
    }
}

/// Structured-report toggles for extraction and preview flows.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct OutputOptions {
    #[serde(default)]
    /// Include the full input source text in structured output.
    pub include_source_text: bool,
    #[serde(default = "default_true")]
    /// Include HTML payloads when they are available.
    pub include_html: bool,
    #[serde(default = "default_true")]
    /// Include text payloads when they are available.
    pub include_text: bool,
    #[serde(default = "default_preview_chars_non_zero")]
    /// Preview length stored in structured output.
    pub preview_chars: NonZeroUsize,
}

impl Default for OutputOptions {
    fn default() -> Self {
        Self {
            include_source_text: false,
            include_html: true,
            include_text: true,
            preview_chars: default_preview_chars_non_zero(),
        }
    }
}

/// Source locator for the HTML being loaded.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct SourceRequest {
    /// Typed source input that determines how HTMLCut loads the document.
    pub input: SourceInput,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional input base URL used for relative-link resolution.
    pub base_url: Option<Url>,
}

impl SourceRequest {
    /// Creates a request for an HTTP or HTTPS source.
    pub fn url(href: Url) -> Self {
        Self {
            input: SourceInput::Url { href },
            base_url: None,
        }
    }

    /// Creates a request for a local file source.
    pub fn file(path: impl Into<PathBuf>) -> Self {
        Self {
            input: SourceInput::File { path: path.into() },
            base_url: None,
        }
    }

    /// Creates a request for standard input.
    pub const fn stdin() -> Self {
        Self {
            input: SourceInput::Stdin,
            base_url: None,
        }
    }

    /// Creates a request for preloaded in-memory HTML.
    pub fn memory(label: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            input: SourceInput::Memory {
                label: label.into(),
                text: text.into(),
            },
            base_url: None,
        }
    }

    /// Sets the input base URL used for relative-link resolution.
    pub fn with_base_url(mut self, base_url: Url) -> Self {
        self.base_url = Some(base_url);
        self
    }

    /// Returns the concrete source kind implied by this request.
    pub const fn kind(&self) -> SourceKind {
        self.input.kind()
    }
}

/// Typed source input that determines the loader HTMLCut should use.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum SourceInput {
    /// Load HTML from an HTTP or HTTPS URL.
    Url {
        /// Absolute HTTP or HTTPS URL to fetch.
        href: Url,
    },
    /// Load HTML from a local file path.
    File {
        /// Local path to the HTML source file.
        path: PathBuf,
    },
    /// Load HTML from standard input.
    Stdin,
    /// Use preloaded in-memory HTML without any external I/O.
    Memory {
        /// Logical label used in metadata for the in-memory source.
        label: String,
        /// HTML source text.
        text: String,
    },
}

impl SourceInput {
    /// Returns the concrete source kind for this input.
    pub const fn kind(&self) -> SourceKind {
        match self {
            Self::Url { .. } => SourceKind::Url,
            Self::File { .. } => SourceKind::File,
            Self::Stdin => SourceKind::Stdin,
            Self::Memory { .. } => SourceKind::Memory,
        }
    }
}

/// Full extraction request consumed by [`crate::extract`] and [`crate::preview_extraction`].
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ExtractionRequest {
    #[serde(default = "default_spec_version")]
    /// Version of the request contract.
    pub spec_version: u32,
    /// Source locator for the HTML input.
    pub source: SourceRequest,
    /// Extraction family and value-shaping configuration.
    pub extraction: ExtractionSpec,
    #[serde(default)]
    /// Post-extraction normalization rules.
    pub normalization: NormalizationOptions,
    #[serde(default)]
    /// Structured-output toggles.
    pub output: OutputOptions,
}

impl ExtractionRequest {
    /// Creates a new extraction request with default normalization and output behavior.
    pub fn new(source: SourceRequest, extraction: ExtractionSpec) -> Self {
        Self {
            spec_version: CORE_SPEC_VERSION,
            source,
            extraction,
            normalization: NormalizationOptions::default(),
            output: OutputOptions::default(),
        }
    }
}

/// Runtime limits used while loading and extracting a source.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct RuntimeOptions {
    #[serde(default = "default_max_bytes")]
    /// Maximum number of bytes that may be read for one source.
    pub max_bytes: usize,
    #[serde(default = "default_fetch_timeout_ms")]
    /// HTTP fetch timeout in milliseconds.
    pub fetch_timeout_ms: u64,
    #[serde(default)]
    /// URL-fetch preflight policy used before reading a remote response body.
    pub fetch_preflight: FetchPreflightMode,
}

impl Default for RuntimeOptions {
    fn default() -> Self {
        Self {
            max_bytes: DEFAULT_MAX_BYTES,
            fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
            fetch_preflight: FetchPreflightMode::default(),
        }
    }
}

/// Versioned reusable extraction definition for repeatable HTMLCut runs.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ExtractionDefinition {
    #[serde(default = "default_extraction_definition_schema_name")]
    /// Stable schema name for this reusable definition file.
    pub schema_name: String,
    #[serde(default = "default_extraction_definition_schema_version")]
    /// Schema version for this reusable definition file.
    pub schema_version: u32,
    /// Extraction request executed by this definition.
    pub request: ExtractionRequest,
    #[serde(default)]
    /// Runtime loading limits and fetch policy applied to this definition.
    pub runtime: RuntimeOptions,
}

impl ExtractionDefinition {
    /// Creates a reusable extraction definition with the current schema identity.
    pub fn new(request: ExtractionRequest) -> Self {
        Self {
            schema_name: default_extraction_definition_schema_name(),
            schema_version: default_extraction_definition_schema_version(),
            request,
            runtime: RuntimeOptions::default(),
        }
    }
}

fn default_extraction_definition_schema_name() -> String {
    crate::EXTRACTION_DEFINITION_SCHEMA_NAME.to_owned()
}

const fn default_extraction_definition_schema_version() -> u32 {
    crate::EXTRACTION_DEFINITION_SCHEMA_VERSION
}

/// Options controlling document-level inspection output.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct InspectionOptions {
    #[serde(default)]
    /// Include the full source text in the inspection result.
    pub include_source_text: bool,
    #[serde(default = "default_inspection_sample_limit")]
    /// Maximum number of sampled headings, links, tags, and classes.
    pub sample_limit: usize,
}

impl Default for InspectionOptions {
    fn default() -> Self {
        Self {
            include_source_text: false,
            sample_limit: DEFAULT_INSPECTION_SAMPLE_LIMIT,
        }
    }
}
