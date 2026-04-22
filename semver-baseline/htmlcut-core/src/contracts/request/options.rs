use std::num::NonZeroUsize;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::super::constants::{
    DEFAULT_FETCH_TIMEOUT_MS, DEFAULT_INSPECTION_SAMPLE_LIMIT, DEFAULT_MAX_BYTES,
};
use super::super::{
    default_fetch_timeout_ms, default_inspection_sample_limit, default_max_bytes,
    default_preview_chars_non_zero, default_true,
};
use super::{FetchPreflightMode, WhitespaceMode};

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
