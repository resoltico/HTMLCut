use std::borrow::Cow;
use std::fmt;
use std::str::FromStr;

use schemars::{JsonSchema, Schema, SchemaGenerator, json_schema};
use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;
use url::Url;

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

/// Error returned when a contract value fails eager validation.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum ContractValueError {
    /// A required string was blank.
    #[error("{field} must not be empty")]
    Empty {
        /// Human-readable field label used in the validation message.
        field: &'static str,
    },
    /// A field contains whitespace where the contract forbids it.
    #[error("{field} must not contain whitespace")]
    ContainsWhitespace {
        /// Human-readable field label used in the validation message.
        field: &'static str,
    },
    /// A URL string could not be parsed or violated HTMLCut's URL contract.
    #[error("{field} is invalid: {message}")]
    InvalidUrl {
        /// Human-readable field label used in the validation message.
        field: &'static str,
        /// Specific parse or validation error detail.
        message: String,
    },
    /// A URL used a scheme outside HTMLCut's supported HTTP(S) contract.
    #[error("{field} must use http or https, got {scheme}")]
    UnsupportedUrlScheme {
        /// Human-readable field label used in the validation message.
        field: &'static str,
        /// Unsupported URL scheme.
        scheme: String,
    },
    /// URL userinfo is forbidden because HTMLCut never stores or reports credential-bearing URLs.
    #[error("{field} must not include URL userinfo")]
    UrlUserInfoUnsupported {
        /// Human-readable field label used in the validation message.
        field: &'static str,
    },
    /// Query strings are forbidden for persisted replayable URL artifacts.
    #[error("{field} must not include a query string")]
    UrlQueryUnsupported {
        /// Human-readable field label used in the validation message.
        field: &'static str,
    },
    /// Public display URL artifacts may only carry the explicit redacted query marker.
    #[error("{field} must not include an unredacted query string")]
    UrlUnredactedQueryUnsupported {
        /// Human-readable field label used in the validation message.
        field: &'static str,
    },
    /// Fragments are forbidden for persisted or display-only URL artifacts.
    #[error("{field} must not include a fragment")]
    UrlFragmentUnsupported {
        /// Human-readable field label used in the validation message.
        field: &'static str,
    },
    /// A numeric contract field must be greater than zero.
    #[error("{field} must be greater than zero")]
    NonPositive {
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
    SliceBoundary,
    "slice boundary",
    "Validated boundary pattern used by slice extraction."
);

/// Validated HTTP or HTTPS URL with a safe redacted display form.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(try_from = "String", into = "String")]
pub struct HttpUrl {
    raw: Url,
    display: String,
}

impl HttpUrl {
    /// Validates and stores one HTTP or HTTPS URL.
    pub fn new(value: Url) -> Result<Self, ContractValueError> {
        validate_http_url("URL", value)
    }

    /// Parses and validates one HTTP or HTTPS URL from text.
    pub fn parse(value: &str) -> Result<Self, ContractValueError> {
        let parsed = Url::parse(value).map_err(|error| ContractValueError::InvalidUrl {
            field: "URL",
            message: error.to_string(),
        })?;
        Self::new(parsed)
    }

    /// Returns the full URL used for actual fetches and joins.
    pub fn as_url(&self) -> &Url {
        &self.raw
    }

    /// Returns the full URL string used for actual fetches and joins.
    pub fn as_fetch_str(&self) -> &str {
        self.raw.as_str()
    }

    /// Returns the redacted URL string safe for diagnostics and reports.
    pub fn display_url(&self) -> &str {
        &self.display
    }
}

impl JsonSchema for HttpUrl {
    fn inline_schema() -> bool {
        true
    }

    fn schema_name() -> Cow<'static, str> {
        "HttpUrl".into()
    }

    fn schema_id() -> Cow<'static, str> {
        concat!(module_path!(), "::HttpUrl").into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        http_url_schema(
            "Absolute HTTP or HTTPS URL without URL userinfo.",
            QueryPolicy::AllowAny,
            false,
        )
    }
}

impl fmt::Display for HttpUrl {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.display_url())
    }
}

impl fmt::Debug for HttpUrl {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("HttpUrl")
            .field(&self.display_url())
            .finish()
    }
}

impl From<HttpUrl> for String {
    fn from(value: HttpUrl) -> Self {
        value.raw.into()
    }
}

impl TryFrom<String> for HttpUrl {
    type Error = ContractValueError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}

impl TryFrom<Url> for HttpUrl {
    type Error = ContractValueError;

    fn try_from(value: Url) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl FromStr for HttpUrl {
    type Err = ContractValueError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

/// Persisted HTTP or HTTPS URL accepted in replayable request documents.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(try_from = "String", into = "String")]
pub struct PersistedHttpUrl(HttpUrl);

impl PersistedHttpUrl {
    /// Parses and validates one persisted replayable URL from text.
    pub fn parse(value: &str) -> Result<Self, ContractValueError> {
        let parsed = Url::parse(value).map_err(|error| ContractValueError::InvalidUrl {
            field: "URL",
            message: error.to_string(),
        })?;
        validate_persisted_http_url("URL", parsed)
    }

    /// Returns the executable HTTP URL this persisted value represents.
    pub fn as_http_url(&self) -> &HttpUrl {
        &self.0
    }
}

impl JsonSchema for PersistedHttpUrl {
    fn inline_schema() -> bool {
        true
    }

    fn schema_name() -> Cow<'static, str> {
        "PersistedHttpUrl".into()
    }

    fn schema_id() -> Cow<'static, str> {
        concat!(module_path!(), "::PersistedHttpUrl").into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        http_url_schema(
            "Absolute replayable HTTP or HTTPS URL without userinfo, query, or fragment.",
            QueryPolicy::ForbidAny,
            true,
        )
    }
}

impl fmt::Display for PersistedHttpUrl {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0.as_fetch_str())
    }
}

impl fmt::Debug for PersistedHttpUrl {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("PersistedHttpUrl")
            .field(&self.0.display_url())
            .finish()
    }
}

impl From<PersistedHttpUrl> for String {
    fn from(value: PersistedHttpUrl) -> Self {
        value.0.as_fetch_str().to_owned()
    }
}

impl From<PersistedHttpUrl> for HttpUrl {
    fn from(value: PersistedHttpUrl) -> Self {
        value.0
    }
}

impl TryFrom<String> for PersistedHttpUrl {
    type Error = ContractValueError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}

impl TryFrom<HttpUrl> for PersistedHttpUrl {
    type Error = ContractValueError;

    fn try_from(value: HttpUrl) -> Result<Self, Self::Error> {
        validate_persisted_http_url("URL", value.raw)
    }
}

impl FromStr for PersistedHttpUrl {
    type Err = ContractValueError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

/// Public display URL safe for reports and interop result artifacts.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(try_from = "String", into = "String")]
pub struct DisplayedHttpUrl(String);

impl DisplayedHttpUrl {
    /// Parses and validates one safe display URL from text.
    pub fn parse(value: &str) -> Result<Self, ContractValueError> {
        let parsed = Url::parse(value).map_err(|error| ContractValueError::InvalidUrl {
            field: "URL",
            message: error.to_string(),
        })?;
        validate_displayed_http_url("URL", parsed)
    }

    /// Returns the stored display string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl JsonSchema for DisplayedHttpUrl {
    fn inline_schema() -> bool {
        true
    }

    fn schema_name() -> Cow<'static, str> {
        "DisplayedHttpUrl".into()
    }

    fn schema_id() -> Cow<'static, str> {
        concat!(module_path!(), "::DisplayedHttpUrl").into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        http_url_schema(
            "Safe display URL for diagnostics and result artifacts. Userinfo and fragments are forbidden, and any query string must be the exact `?[redacted]` marker.",
            QueryPolicy::AllowRedactedOnly,
            true,
        )
    }
}

impl AsRef<str> for DisplayedHttpUrl {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for DisplayedHttpUrl {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl fmt::Debug for DisplayedHttpUrl {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("DisplayedHttpUrl")
            .field(&self.as_str())
            .finish()
    }
}

impl From<DisplayedHttpUrl> for String {
    fn from(value: DisplayedHttpUrl) -> Self {
        value.0
    }
}

impl From<&HttpUrl> for DisplayedHttpUrl {
    fn from(value: &HttpUrl) -> Self {
        Self(value.display_url().to_owned())
    }
}

impl TryFrom<String> for DisplayedHttpUrl {
    type Error = ContractValueError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}

impl FromStr for DisplayedHttpUrl {
    type Err = ContractValueError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

/// Validated attribute name used by attribute extraction.
#[derive(
    Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
#[serde(try_from = "String")]
#[schemars(with = "String")]
pub struct AttributeName(String);

impl AttributeName {
    /// Validates and stores one attribute name.
    pub fn new(value: impl Into<String>) -> Result<Self, ContractValueError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(ContractValueError::Empty {
                field: "attribute name",
            });
        }
        if value.chars().any(char::is_whitespace) {
            return Err(ContractValueError::ContainsWhitespace {
                field: "attribute name",
            });
        }

        Ok(Self(value.to_ascii_lowercase()))
    }

    /// Returns the stored string value.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for AttributeName {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for AttributeName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl TryFrom<String> for AttributeName {
    type Error = ContractValueError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

pub(crate) fn validate_http_url(
    field: &'static str,
    value: Url,
) -> Result<HttpUrl, ContractValueError> {
    validate_http_url_components(field, &value)?;

    Ok(HttpUrl {
        display: redacted_display_url(&value),
        raw: value,
    })
}

fn validate_persisted_http_url(
    field: &'static str,
    value: Url,
) -> Result<PersistedHttpUrl, ContractValueError> {
    let url = validate_http_url(field, value)?;
    if url.raw.query().is_some() {
        return Err(ContractValueError::UrlQueryUnsupported { field });
    }
    if url.raw.fragment().is_some() {
        return Err(ContractValueError::UrlFragmentUnsupported { field });
    }

    Ok(PersistedHttpUrl(url))
}

fn validate_displayed_http_url(
    field: &'static str,
    value: Url,
) -> Result<DisplayedHttpUrl, ContractValueError> {
    validate_http_url_components(field, &value)?;
    if value.query().is_some_and(|query| query != "[redacted]") {
        return Err(ContractValueError::UrlUnredactedQueryUnsupported { field });
    }
    if value.fragment().is_some() {
        return Err(ContractValueError::UrlFragmentUnsupported { field });
    }

    Ok(DisplayedHttpUrl(value.to_string()))
}

fn validate_http_url_components(
    field: &'static str,
    value: &Url,
) -> Result<(), ContractValueError> {
    if !matches!(value.scheme(), "http" | "https") {
        return Err(ContractValueError::UnsupportedUrlScheme {
            field,
            scheme: value.scheme().to_owned(),
        });
    }
    if !value.username().is_empty() || value.password().is_some() {
        return Err(ContractValueError::UrlUserInfoUnsupported { field });
    }

    Ok(())
}

fn redacted_display_url(value: &Url) -> String {
    let mut redacted = value.clone();
    if value.query().is_some() {
        redacted.set_query(Some("[redacted]"));
    }
    redacted.set_fragment(None);
    redacted.to_string()
}

enum QueryPolicy {
    AllowAny,
    ForbidAny,
    AllowRedactedOnly,
}

fn http_url_schema(
    description: &'static str,
    query_policy: QueryPolicy,
    forbid_fragment: bool,
) -> Schema {
    let mut all_of = vec![
        json!({ "pattern": "^https?://" }),
        json!({ "not": { "pattern": "^https?://[^/?#]*@" } }),
    ];
    match query_policy {
        QueryPolicy::AllowAny => {}
        QueryPolicy::ForbidAny => {
            all_of.push(json!({ "not": { "pattern": "\\?" } }));
        }
        QueryPolicy::AllowRedactedOnly => {
            all_of.push(json!({
                "anyOf": [
                    { "not": { "pattern": "\\?" } },
                    { "pattern": "\\?\\[redacted\\]$" }
                ]
            }));
        }
    }
    if forbid_fragment {
        all_of.push(json!({ "not": { "pattern": "#" } }));
    }

    json_schema!({
        "type": "string",
        "format": "uri",
        "description": description,
        "allOf": all_of,
    })
}
