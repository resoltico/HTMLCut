use std::borrow::Cow;
use std::fmt;
use std::str::FromStr;

use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Serialize};
use url::Url;

use super::ContractValueError;
use super::policy::{
    QueryPolicy, http_url_schema, validate_displayed_http_url, validate_http_url,
    validate_persisted_http_url,
};

const PRIMITIVES_SCHEMA_MODULE: &str = "htmlcut_core::contracts::request::primitives";

/// Validated HTTP or HTTPS URL with a safe redacted display form.
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(try_from = "String", into = "String")]
pub struct HttpUrl {
    pub(super) raw: Url,
    pub(super) display: String,
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
        format!("{PRIMITIVES_SCHEMA_MODULE}::HttpUrl").into()
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
pub struct PersistedHttpUrl(pub(super) HttpUrl);

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
        format!("{PRIMITIVES_SCHEMA_MODULE}::PersistedHttpUrl").into()
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
pub struct DisplayedHttpUrl(pub(super) String);

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
        format!("{PRIMITIVES_SCHEMA_MODULE}::DisplayedHttpUrl").into()
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
