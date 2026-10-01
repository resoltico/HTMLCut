//! One compact success and typed failure family.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Wire-family version, independent of extraction semantics.
pub const SCHEMA_VERSION: u32 = 1;
/// Version of projection, selection and identity semantics.
pub const SEMANTICS_VERSION: u32 = 1;

/// Closed failure codes shared by CLI and Rust callers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    /// Invalid closed JSON document.
    InvalidJson,
    /// Unknown schema family or version.
    InvalidSchema,
    /// Invalid combination of plan options.
    InvalidPlan,
    /// Invalid CSS grammar.
    InvalidSelector,
    /// Invalid regular-expression grammar or flags.
    InvalidRegex,
    /// Invalid limit policy.
    InvalidLimit,
    /// Invalid explicit base metadata.
    InvalidBaseUrl,
    /// No selected node or source pair.
    NoMatch,
    /// More than one candidate for single selection.
    AmbiguousSelection,
    /// Selection cardinality differs from the declared bounds.
    Cardinality,
    /// Requested attribute is absent.
    MissingAttribute,
    /// Declared original-DOM guard failed.
    GuardFailed,
    /// A source opening has no following closing boundary.
    MissingBoundary,
    /// A regular-expression boundary matched zero bytes.
    EmptyBoundaryMatch,
    /// Finite source, preparation, work or output budget exhausted.
    ResourceLimit,
    /// Adapter input options are incompatible.
    InvalidOptions,
    /// Adapter could not acquire the source.
    Acquisition,
    /// Strict decoding failed.
    Decoding,
    /// Adapter could not publish the complete staged result.
    Publication,
    /// An internal invariant failed.
    InternalInvariant,
}

impl ErrorCode {
    /// Fixed CLI exit class for this machine code.
    pub const fn exit_class(self) -> u8 {
        match self {
            Self::InvalidJson
            | Self::InvalidSchema
            | Self::InvalidPlan
            | Self::InvalidSelector
            | Self::InvalidRegex
            | Self::InvalidLimit
            | Self::InvalidBaseUrl
            | Self::InvalidOptions => 2,
            Self::NoMatch
            | Self::AmbiguousSelection
            | Self::Cardinality
            | Self::MissingAttribute
            | Self::GuardFailed
            | Self::MissingBoundary
            | Self::EmptyBoundaryMatch => 3,
            Self::ResourceLimit => 4,
            Self::Acquisition | Self::Decoding | Self::Publication => 5,
            Self::InternalInvariant => 6,
        }
    }
}

/// A bounded error document; messages never include source or transport error chains.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema, thiserror::Error)]
#[error("{message}")]
#[serde(deny_unknown_fields)]
pub struct ExtractionError {
    /// Error role name.
    #[schemars(extend("const" = "htmlcut.extraction.error"))]
    pub schema: String,
    /// Wire-family version.
    #[schemars(extend("const" = SCHEMA_VERSION))]
    pub version: u32,
    /// Stable failure code.
    pub code: ErrorCode,
    /// Owning execution stage.
    pub stage: String,
    /// Bounded safe explanation.
    pub message: String,
    /// Sparse identity/count evidence, boxed to keep the returned error family small.
    #[serde(flatten)]
    pub evidence: Box<ErrorEvidence>,
}

/// Applicable failure evidence; absent fields are never fabricated or emitted as null.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ErrorEvidence {
    /// An observed lower bound, never presented as an exact count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observed_at_least: Option<u32>,
    /// Accepted source identity when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_sha256: Option<String>,
    /// Normalized compiled-plan identity when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_sha256: Option<String>,
    /// Deterministic execution-input identity when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extraction_sha256: Option<String>,
    /// Exact candidate count, only after complete enumeration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidate_count: Option<u32>,
    /// Exact selected count when selection completed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_count: Option<u32>,
}

impl ExtractionError {
    /// Attaches identities and exact counts from an already validated result to an adapter failure.
    pub fn with_result(mut self, result: &ExtractionResult) -> Self {
        self.source_sha256 = Some(result.source_sha256.clone());
        self.plan_sha256 = Some(result.plan_sha256.clone());
        self.extraction_sha256 = Some(result.extraction_sha256.clone());
        self.candidate_count = Some(result.candidate_count);
        self.selected_count = Some(result.selected_count);
        self
    }
    /// Constructs a safe error from an owned static explanation.
    pub fn new(code: ErrorCode, stage: &'static str, message: &'static str) -> Self {
        Self {
            schema: "htmlcut.extraction.error".into(),
            version: SCHEMA_VERSION,
            code,
            stage: stage.into(),
            message: message.into(),
            evidence: Box::default(),
        }
    }

    pub(crate) fn limit(stage: &'static str) -> Self {
        Self::new(
            ErrorCode::ResourceLimit,
            stage,
            "The operation exceeded its configured resource limit.",
        )
    }
}

impl std::ops::Deref for ExtractionError {
    type Target = ErrorEvidence;
    fn deref(&self) -> &Self::Target {
        &self.evidence
    }
}

impl std::ops::DerefMut for ExtractionError {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.evidence
    }
}

/// Half-open UTF-8 byte range in the original accepted snapshot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SourceRange {
    /// Inclusive starting byte offset.
    pub start: usize,
    /// Exclusive ending byte offset.
    pub end: usize,
}

/// Complete deterministic requested values, without parallel representations.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExtractionResult {
    /// Success role name.
    #[schemars(extend("const" = "htmlcut.extraction.result"))]
    pub schema: String,
    /// Wire-family version.
    #[schemars(extend("const" = SCHEMA_VERSION))]
    pub version: u32,
    /// Extraction-semantics version.
    #[schemars(extend("const" = SEMANTICS_VERSION))]
    pub semantics: u32,
    /// SHA-256 of exact accepted UTF-8 bytes.
    pub source_sha256: String,
    /// SHA-256 identity of the normalized plan.
    pub plan_sha256: String,
    /// Domain-separated execution identity.
    pub extraction_sha256: String,
    /// Complete number of candidates.
    pub candidate_count: u32,
    /// Complete number of selected values.
    pub selected_count: u32,
    /// Requested values in document/source order.
    pub values: Vec<String>,
    /// Source ranges corresponding to slice values, absent for DOM selection.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ranges: Option<Vec<SourceRange>>,
}
