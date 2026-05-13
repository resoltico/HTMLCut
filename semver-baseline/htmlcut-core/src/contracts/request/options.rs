use std::num::{NonZeroU64, NonZeroUsize};
use std::path::PathBuf;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::super::constants::DEFAULT_INSPECTION_SAMPLE_LIMIT;
use super::super::{
    default_fetch_connect_timeout_ms, default_fetch_timeout_ms, default_inspection_sample_limit,
    default_max_bytes, default_preview_chars_non_zero, default_true,
};
use super::{ContractValueError, FetchPreflightMode, WhitespaceMode};

/// Maximum number of bytes HTMLCut may read for one source.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(try_from = "usize", into = "usize")]
#[schemars(with = "usize")]
pub struct MaxBytes(NonZeroUsize);

impl MaxBytes {
    /// Validates and stores one maximum-byte limit.
    pub fn new(value: usize) -> Result<Self, ContractValueError> {
        NonZeroUsize::new(value)
            .map(Self)
            .ok_or(ContractValueError::NonPositive { field: "max_bytes" })
    }

    /// Returns the raw byte limit.
    pub const fn get(self) -> usize {
        self.0.get()
    }
}

impl TryFrom<usize> for MaxBytes {
    type Error = ContractValueError;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<MaxBytes> for usize {
    fn from(value: MaxBytes) -> Self {
        value.get()
    }
}

/// HTTP fetch timeout in milliseconds.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(try_from = "u64", into = "u64")]
#[schemars(with = "u64")]
pub struct FetchTimeoutMs(NonZeroU64);

impl FetchTimeoutMs {
    /// Validates and stores one fetch timeout.
    pub fn new(value: u64) -> Result<Self, ContractValueError> {
        NonZeroU64::new(value)
            .map(Self)
            .ok_or(ContractValueError::NonPositive {
                field: "fetch_timeout_ms",
            })
    }

    /// Returns the raw timeout value in milliseconds.
    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

impl TryFrom<u64> for FetchTimeoutMs {
    type Error = ContractValueError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<FetchTimeoutMs> for u64 {
    fn from(value: FetchTimeoutMs) -> Self {
        value.get()
    }
}

/// HTTP connect timeout in milliseconds.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(try_from = "u64", into = "u64")]
#[schemars(with = "u64")]
pub struct FetchConnectTimeoutMs(NonZeroU64);

impl FetchConnectTimeoutMs {
    /// Validates and stores one connect timeout.
    pub fn new(value: u64) -> Result<Self, ContractValueError> {
        NonZeroU64::new(value)
            .map(Self)
            .ok_or(ContractValueError::NonPositive {
                field: "fetch_connect_timeout_ms",
            })
    }

    /// Returns the raw timeout value in milliseconds.
    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

impl TryFrom<u64> for FetchConnectTimeoutMs {
    type Error = ContractValueError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<FetchConnectTimeoutMs> for u64 {
    fn from(value: FetchConnectTimeoutMs) -> Self {
        value.get()
    }
}

/// TLS trust roots used for built-in HTTP fetching.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum TlsTrustPolicy {
    /// Use Mozilla's bundled WebPKI root store.
    #[default]
    WebPki,
    /// Use the host platform's verifier and root store.
    Platform,
    /// Load one explicit PEM bundle as the sole trust root set.
    CustomCaBundle {
        /// Path to the PEM-encoded root certificate bundle.
        path: PathBuf,
    },
}

/// Rendering policy applied after raw value extraction.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct RenderingOptions {
    #[serde(default)]
    /// Whitespace treatment for text-like outputs.
    pub whitespace: WhitespaceMode,
    #[serde(default)]
    /// Whether relative URLs should be rewritten against the effective base URL.
    pub rewrite_urls: bool,
}

impl Default for RenderingOptions {
    fn default() -> Self {
        Self {
            whitespace: WhitespaceMode::Rendered,
            rewrite_urls: false,
        }
    }
}

/// Structured-report toggles and rendering policy for extraction and preview flows.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct OutputOptions {
    #[serde(default)]
    /// Rendering policy for extracted values.
    pub rendering: RenderingOptions,
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
            rendering: RenderingOptions::default(),
            include_source_text: false,
            include_html: true,
            include_text: true,
            preview_chars: default_preview_chars_non_zero(),
        }
    }
}

/// Runtime limits used while loading and extracting a source.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct RuntimeOptions {
    #[serde(default = "default_max_bytes_limit")]
    /// Maximum number of bytes that may be read for one source.
    pub max_bytes: MaxBytes,
    #[serde(default = "default_fetch_timeout_limit")]
    /// HTTP fetch timeout in milliseconds.
    pub fetch_timeout: FetchTimeoutMs,
    #[serde(default = "default_fetch_connect_timeout_limit")]
    /// HTTP connect timeout in milliseconds.
    pub fetch_connect_timeout: FetchConnectTimeoutMs,
    #[serde(default)]
    /// URL-fetch preflight policy used before reading a remote response body.
    pub fetch_preflight: FetchPreflightMode,
    #[serde(default)]
    /// TLS trust policy used for built-in HTTP fetches.
    pub tls_trust: TlsTrustPolicy,
}

impl Default for RuntimeOptions {
    fn default() -> Self {
        Self {
            max_bytes: default_max_bytes_limit(),
            fetch_timeout: default_fetch_timeout_limit(),
            fetch_connect_timeout: default_fetch_connect_timeout_limit(),
            fetch_preflight: FetchPreflightMode::default(),
            tls_trust: TlsTrustPolicy::default(),
        }
    }
}

/// Options controlling document-level inspection output.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct InspectionOptions {
    #[serde(default)]
    /// Include the full source text in the inspection result.
    pub include_source_text: bool,
    #[serde(default = "default_inspection_sample_limit")]
    /// Maximum number of sampled headings, links, tags, classes, and content candidates.
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

fn default_max_bytes_limit() -> MaxBytes {
    MaxBytes::new(default_max_bytes()).expect("default max-bytes limit must be non-zero")
}

fn default_fetch_timeout_limit() -> FetchTimeoutMs {
    FetchTimeoutMs::new(default_fetch_timeout_ms()).expect("default fetch timeout must be non-zero")
}

fn default_fetch_connect_timeout_limit() -> FetchConnectTimeoutMs {
    FetchConnectTimeoutMs::new(default_fetch_connect_timeout_ms())
        .expect("default connect timeout must be non-zero")
}
