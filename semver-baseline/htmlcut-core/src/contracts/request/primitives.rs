use std::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

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

crate::cli_choice::impl_cli_choice!(ValueType {
    ValueType::Text => "text",
    ValueType::InnerHtml => "inner-html",
    ValueType::OuterHtml => "outer-html",
    ValueType::Attribute => "attribute",
    ValueType::Structured => "structured",
});

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

crate::cli_choice::impl_cli_choice!(WhitespaceMode {
    WhitespaceMode::Preserve => "preserve",
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
