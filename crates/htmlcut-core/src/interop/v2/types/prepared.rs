//! Opaque prepared-document contracts for the `htmlcut-v2` interop profile.

use std::fmt;
use std::num::NonZeroU32;

use ego_tree::NodeId;
use schemars::JsonSchema;
use scraper::{ElementRef, Html};
use serde::{Deserialize, Serialize};

use super::{ContractError, HtmlInput};
/// Stable schema name for document-preparation failures.
pub const PREPARATION_ERROR_SCHEMA_NAME: &str = "htmlcut.preparation_error";
/// Stable schema version for document-preparation failures.
pub const PREPARATION_ERROR_SCHEMA_VERSION: u32 = 1;

/// Immutable limits applied before one HTML input becomes a prepared document.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PreparationLimits {
    /// Maximum decoded source bytes accepted before parsing.
    pub max_input_bytes: NonZeroU32,
    /// Maximum parsed HTML element count.
    pub max_elements: NonZeroU32,
    /// Maximum parsed DOM depth.
    pub max_dom_depth: NonZeroU32,
}

impl Default for PreparationLimits {
    fn default() -> Self {
        Self {
            max_input_bytes: NonZeroU32::new(50 * 1024 * 1024).expect("non-zero constant"),
            max_elements: NonZeroU32::new(250_000).expect("non-zero constant"),
            max_dom_depth: NonZeroU32::new(2_048).expect("non-zero constant"),
        }
    }
}

impl PreparationLimits {
    /// Builds one validated preparation limit set within the v2 hard maxima.
    pub fn new(
        max_input_bytes: NonZeroU32,
        max_elements: NonZeroU32,
        max_dom_depth: NonZeroU32,
    ) -> Result<Self, ContractError> {
        const MAX_INPUT_BYTES: u32 = 50 * 1024 * 1024;
        const MAX_ELEMENTS: u32 = 1_000_000;
        const MAX_DOM_DEPTH: u32 = 4_096;
        if max_input_bytes.get() > MAX_INPUT_BYTES
            || max_elements.get() > MAX_ELEMENTS
            || max_dom_depth.get() > MAX_DOM_DEPTH
        {
            return Err(ContractError::PreparationLimitExceeded);
        }
        Ok(Self {
            max_input_bytes,
            max_elements,
            max_dom_depth,
        })
    }

    /// Validates this value against the v2 hard maxima.
    pub fn validate(&self) -> Result<(), ContractError> {
        Self::new(self.max_input_bytes, self.max_elements, self.max_dom_depth).map(|_| ())
    }
}

/// Closed preparation failure vocabulary.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PreparationErrorCode {
    /// The decoded source exceeded the configured byte limit.
    InputTooLarge,
    /// The parsed document exceeded the configured element limit.
    DocumentTooComplex,
    /// The parsed document exceeded the configured depth limit.
    DocumentTooDeep,
    /// HTMLCut detected an invariant violation while preparing the document.
    InternalInvariantViolation,
}

/// Typed error produced before a source has any extraction-plan identity.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PreparationError {
    /// Schema identity.
    pub schema_name: String,
    /// Schema version.
    pub schema_version: u32,
    /// Closed preparation failure code.
    pub error_code: PreparationErrorCode,
    /// SHA-256 identity of the exact input and preparation limits.
    pub input_digest_sha256: String,
    /// Bounded HTMLCut-owned explanation.
    pub message: String,
}

impl PreparationError {
    pub(crate) fn new(
        error_code: PreparationErrorCode,
        input_digest_sha256: String,
        message: impl Into<String>,
    ) -> Self {
        Self {
            schema_name: PREPARATION_ERROR_SCHEMA_NAME.to_owned(),
            schema_version: PREPARATION_ERROR_SCHEMA_VERSION,
            error_code,
            input_digest_sha256,
            message: message.into(),
        }
    }
}

/// One immutable parsed HTML document prepared from exact caller input.
pub struct PreparedDocument {
    input: HtmlInput,
    input_digest_sha256: String,
    document: Html,
    element_ids: Vec<NodeId>,
    effective_base_url: Option<String>,
}

impl fmt::Debug for PreparedDocument {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("PreparedDocument")
            .field("input_digest_sha256", &self.input_digest_sha256)
            .field("effective_base_url", &self.effective_base_url)
            .finish_non_exhaustive()
    }
}

impl PreparedDocument {
    /// Returns the exact caller-owned input identity bound to these preparation limits.
    pub fn input_digest_sha256(&self) -> &str {
        &self.input_digest_sha256
    }

    /// Returns the exact input base URL supplied before document-base resolution.
    pub fn input_base_url(&self) -> Option<&super::HttpUrl> {
        self.input.input_base_url.as_ref()
    }

    /// Returns the effective document base URL after `<base href>` resolution.
    pub fn effective_base_url(&self) -> Option<&str> {
        self.effective_base_url.as_deref()
    }

    pub(crate) fn document(&self) -> &Html {
        &self.document
    }

    pub(crate) fn input(&self) -> &HtmlInput {
        &self.input
    }

    /// Returns the internal element at one document-order ordinal.
    ///
    /// This private index keeps cursor resumption from rescanning skipped DOM elements. Node IDs
    /// never cross the public prepared-document boundary or enter any serialized contract.
    pub(crate) fn element_at_ordinal(&self, ordinal: NonZeroU32) -> ElementRef<'_> {
        let index = usize::try_from(ordinal.get() - 1)
            .expect("published exploration ordinals fit every supported usize");
        let id = self
            .element_ids
            .get(index)
            .expect("caller validated the prepared element ordinal");
        ElementRef::wrap(
            self.document
                .tree
                .get(*id)
                .expect("prepared element index must reference the owned DOM"),
        )
        .expect("prepared element index contains only element node IDs")
    }

    /// Returns the bounded number of internally indexed elements.
    pub(crate) fn element_count(&self) -> u32 {
        u32::try_from(self.element_ids.len())
            .expect("preparation validates the element count against the v2 u32 maximum")
    }

    pub(crate) fn from_prepared_parts(
        input: HtmlInput,
        input_digest_sha256: String,
        document: Html,
        element_ids: Vec<NodeId>,
        effective_base_url: Option<String>,
    ) -> Self {
        Self {
            input,
            input_digest_sha256,
            document,
            element_ids,
            effective_base_url,
        }
    }
}
