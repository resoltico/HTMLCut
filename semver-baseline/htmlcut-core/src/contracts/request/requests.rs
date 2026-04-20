use std::path::PathBuf;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use url::Url;

use super::super::constants::CORE_SPEC_VERSION;
use super::super::default_spec_version;
use super::{ExtractionSpec, NormalizationOptions, OutputOptions, RuntimeOptions, SourceKind};

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
