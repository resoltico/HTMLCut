use std::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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
    /// Return the exact selected HTML fragment.
    SelectedHtml,
    /// Return inner HTML.
    InnerHtml,
    /// Return outer HTML.
    OuterHtml,
    /// Return one named attribute value.
    Attribute,
    /// Return a structured JSON payload with metadata.
    Structured,
}

crate::cli_choice::impl_cli_choice!(ValueType {
    ValueType::Text => "text",
    ValueType::SelectedHtml => "selected-html",
    ValueType::InnerHtml => "inner-html",
    ValueType::OuterHtml => "outer-html",
    ValueType::Attribute => "attribute",
    ValueType::Structured => "structured",
});

/// Whitespace treatment applied to text-like values.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WhitespaceMode {
    /// Preserve the rendered text layout after HTML-aware rendering.
    #[default]
    Rendered,
    /// Collapse inline whitespace for human-readable text.
    Normalize,
}

crate::cli_choice::impl_cli_choice!(WhitespaceMode {
    WhitespaceMode::Rendered => "rendered",
    WhitespaceMode::Normalize => "normalize",
});

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

crate::cli_choice::impl_cli_choice!(PatternMode {
    PatternMode::Literal => "literal",
    PatternMode::Regex => "regex",
});

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

crate::cli_choice::impl_cli_choice!(FetchPreflightMode {
    FetchPreflightMode::HeadFirst => "head-first",
    FetchPreflightMode::GetOnly => "get-only",
});
