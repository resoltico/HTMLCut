//! Bounded discovery and selector-proposal contracts for prepared documents.

use std::num::NonZeroU32;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{ContractError, CssSelectorText};

/// Stable schema name for exploration results.
pub const EXPLORATION_RESULT_SCHEMA_NAME: &str = "htmlcut.exploration_result";
/// Initial schema version for exploration results.
pub const EXPLORATION_RESULT_SCHEMA_VERSION: u32 = 1;
/// Stable schema name for exploration failures.
pub const EXPLORATION_ERROR_SCHEMA_NAME: &str = "htmlcut.exploration_error";
/// Initial schema version for exploration failures.
pub const EXPLORATION_ERROR_SCHEMA_VERSION: u32 = 1;
/// Stable schema name for target-resolution results.
pub const TARGET_RESOLUTION_RESULT_SCHEMA_NAME: &str = "htmlcut.target_resolution_result";
/// Initial schema version for target-resolution results.
pub const TARGET_RESOLUTION_RESULT_SCHEMA_VERSION: u32 = 1;

const MAX_ELEMENTS: u32 = 1_000;
const MAX_WORK_UNITS: u32 = 5_000_000;
const MAX_PROPOSALS_PER_ELEMENT: u32 = 16;
const MAX_PREVIEW_BYTES: u32 = 512;

/// Page limits for deterministic prepared-document exploration.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExplorationOptions {
    /// Opaque pagination position returned by an earlier exploration page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<ExplorationCursor>,
    /// Maximum descriptors returned by this page.
    pub max_elements: NonZeroU32,
    /// Cumulative discovery and selector-proof work budget for this page.
    pub max_work_units: NonZeroU32,
    /// Maximum fully proved proposals retained for one element.
    pub max_proposals_per_element: NonZeroU32,
    /// Maximum UTF-8 bytes in one element text preview.
    pub preview_bytes: NonZeroU32,
}

impl Default for ExplorationOptions {
    fn default() -> Self {
        Self {
            cursor: None,
            max_elements: NonZeroU32::new(100).expect("non-zero constant"),
            max_work_units: NonZeroU32::new(100_000).expect("non-zero constant"),
            max_proposals_per_element: NonZeroU32::new(4).expect("non-zero constant"),
            preview_bytes: NonZeroU32::new(256).expect("non-zero constant"),
        }
    }
}

impl ExplorationOptions {
    /// Validates this page request against HTMLCut's fixed exploration maxima.
    pub fn validate(&self) -> Result<(), ContractError> {
        if self.max_elements.get() > MAX_ELEMENTS
            || self.max_work_units.get() > MAX_WORK_UNITS
            || self.max_proposals_per_element.get() > MAX_PROPOSALS_PER_ELEMENT
            || self.preview_bytes.get() > MAX_PREVIEW_BYTES
        {
            return Err(ContractError::ExplorationLimitExceeded);
        }
        Ok(())
    }
}

/// Opaque, snapshot-bound pagination state; never a DOM-node identity.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExplorationCursor {
    discovery_identity_sha256: String,
    options_digest_sha256: String,
    next_document_ordinal: NonZeroU32,
}

impl ExplorationCursor {
    pub(crate) fn new(
        discovery_identity_sha256: String,
        options_digest_sha256: String,
        next_document_ordinal: NonZeroU32,
    ) -> Self {
        Self {
            discovery_identity_sha256,
            options_digest_sha256,
            next_document_ordinal,
        }
    }

    pub(crate) fn discovery_identity_sha256(&self) -> &str {
        &self.discovery_identity_sha256
    }

    pub(crate) fn options_digest_sha256(&self) -> &str {
        &self.options_digest_sha256
    }

    pub(crate) fn next_document_ordinal(&self) -> NonZeroU32 {
        self.next_document_ordinal
    }
}

/// Namespace preserved by HTMLCut's parser for one element name.
#[derive(
    Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord,
)]
#[serde(rename_all = "lowercase")]
pub enum ElementNamespace {
    /// HTML namespace.
    Html,
    /// SVG namespace.
    Svg,
    /// MathML namespace.
    Mathml,
}

/// Namespace-aware parser-preserved local name.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct ElementName {
    /// Parser-preserved namespace.
    pub namespace: ElementNamespace,
    /// Parser-preserved local name.
    pub local_name: String,
}

/// One bounded, safe semantic attribute published for discovery.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExplorationAttribute {
    /// Attribute local name.
    pub name: String,
    /// Exact bounded attribute value.
    pub value: String,
    /// Whether `value` was explicitly shortened to the published bound.
    pub value_truncated: bool,
}

/// Closed stability class for a proven selector proposal.
#[derive(
    Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord,
)]
#[serde(rename_all = "snake_case")]
pub enum SelectorStability {
    /// Stable semantic identity such as a non-generated unique ID.
    Semantic,
    /// Stable semantic or accessibility attribute.
    StableAttribute,
    /// Stable class combination.
    ClassBased,
    /// Anchored ancestor and descendant structure.
    Structural,
    /// Exact positional fallback.
    Positional,
}

/// Closed evidence explaining a selector proposal's stability classification.
#[derive(
    Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord,
)]
#[serde(rename_all = "snake_case")]
pub enum SelectorStabilityReason {
    /// A unique, non-generated ID anchored the proposal.
    UniqueId,
    /// A semantic attribute anchored the proposal.
    SemanticAttribute,
    /// An ARIA attribute anchored the proposal.
    AriaAttribute,
    /// A microdata attribute anchored the proposal.
    MicrodataAttribute,
    /// A non-generated class anchored the proposal.
    StableClass,
    /// An ancestor selector anchored a descendant proposal.
    AncestorAnchor,
    /// The proposal uses a complete structural path.
    StructuralPath,
    /// The proposal depends on a positional ordinal.
    NthDependency,
    /// An ID looked generated and was downgraded.
    GeneratedLookingIdentifier,
    /// A class looked generated and was downgraded.
    GeneratedLookingClass,
    /// HTMLCut proved uniqueness against this prepared snapshot.
    UniqueOnSnapshot,
}

/// One same-snapshot-proved CSS selector proposal.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SelectorProposal {
    /// Parsed CSS selector text that selected this element exactly once.
    pub selector: CssSelectorText,
    /// Closed stability class.
    pub stability: SelectorStability,
    /// Ordered, closed evidence for the classification.
    pub reasons: Vec<SelectorStabilityReason>,
    /// Explicit uniqueness proof count, always one for a returned proposal.
    pub match_count: NonZeroU32,
    /// Bounded HTMLCut-owned explanatory prose.
    pub rationale: String,
}

/// Reason that an otherwise visited element was not represented as a descriptor.
#[derive(
    Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord,
)]
#[serde(rename_all = "snake_case")]
pub enum ExplorationOmissionReason {
    /// Building this descriptor would exceed the request's shared work budget.
    WorkLimit,
    /// Its complete DOM path exceeded the published bound.
    PathTooLong,
    /// Its namespace or local name exceeded the published bound.
    ElementNameTooLong,
    /// The parser preserved a namespace outside HTMLCut's closed exploration vocabulary.
    UnsupportedNamespace,
}

/// Count of omitted descriptors for one closed omission reason.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExplorationOmission {
    /// Closed reason descriptors were omitted.
    pub reason: ExplorationOmissionReason,
    /// Number of omitted descriptors.
    pub count: u32,
}

/// One fully bounded explored element.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ElementDescriptor {
    /// Complete namespace-aware DOM path.
    pub path: String,
    /// Namespace-aware element name.
    pub element_name: ElementName,
    /// Explicitly bounded text preview.
    pub text_preview: String,
    /// Whether the text preview was explicitly shortened to its requested byte bound.
    pub text_preview_truncated: bool,
    /// Bounded semantic attributes only.
    pub attributes: Vec<ExplorationAttribute>,
    /// Whether additional safe attributes exceeded the published descriptor bound.
    pub attributes_truncated: bool,
    /// Direct child element count.
    pub child_element_count: u32,
    /// Fully proved proposals in deterministic order.
    pub selector_proposals: Vec<SelectorProposal>,
    /// Proposal proof reached the page work budget after emitting the listed proposals.
    pub proposal_work_truncated: bool,
    /// No bounded unique proposal exists for this snapshot.
    pub no_bounded_unique_selector: bool,
}

/// Explicit page truncation state.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExplorationTruncationReason {
    /// The requested descriptor count was reached.
    ElementLimit,
    /// The cumulative work budget was reached.
    WorkLimit,
}

/// Deterministic page of exploration evidence.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExplorationResult {
    /// Schema identity.
    pub schema_name: String,
    /// Schema version.
    pub schema_version: u32,
    /// Discovery identity of the exact prepared snapshot and discovery semantics.
    pub discovery_identity_sha256: String,
    /// Work consumed by discovery and uniqueness proofs.
    pub consumed_work_units: u32,
    /// Returned descriptors in document order.
    pub elements: Vec<ElementDescriptor>,
    /// Counts of omitted elements by closed reason.
    pub omitted_element_counts: Vec<ExplorationOmission>,
    /// Why bounded page work stopped; it may be present without a continuation if no elements remain.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncation_reason: Option<ExplorationTruncationReason>,
    /// Opaque continuation cursor when additional document-order elements remain.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<ExplorationCursor>,
}

/// Closed exploration failure class.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExplorationErrorCode {
    /// Request limits exceeded HTMLCut's published maxima.
    InvalidLimits,
    /// Cursor identity did not bind this exact snapshot.
    CursorSnapshotMismatch,
    /// Cursor options did not bind this exact traversal contract.
    CursorOptionsMismatch,
    /// Cursor encoding was malformed or outside its bound.
    InvalidCursor,
    /// Target path, name, text, or attributes did not identify an element.
    TargetNotFound,
    /// Target descendant text exceeded the fingerprint byte bound.
    TargetFingerprintLimitExceeded,
    /// Atomic target resolution exhausted its work budget.
    TargetResolutionLimitExceeded,
    /// HTMLCut detected an internal exploration invariant violation.
    InternalInvariantViolation,
}

/// Typed bounded exploration failure.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExplorationError {
    /// Schema identity.
    pub schema_name: String,
    /// Schema version.
    pub schema_version: u32,
    /// Closed error code.
    pub error_code: ExplorationErrorCode,
    /// Discovery identity when it was available.
    pub discovery_identity_sha256: String,
    /// Bounded HTMLCut-owned message.
    pub message: String,
}

/// One complete browser-originated target claim for an HTMLCut prepared document.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ElementTargetHint {
    /// Frozen path grammar identifier.
    pub path_profile: String,
    /// Complete namespace-aware DOM path.
    pub path: String,
    /// Namespace-aware element name, required to agree with the final path segment.
    pub element_name: ElementName,
    /// Lowercase SHA-256 digest of normalized descendant DOM text.
    pub normalized_text_digest_sha256: String,
    /// Optional exact semantic attributes that must all agree when supplied.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub semantic_attributes: Vec<ExplorationAttribute>,
}

/// Atomic work limits for resolving one browser target and proving its proposals.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TargetResolutionOptions {
    /// Cumulative path, fingerprint, and proof work budget.
    pub max_work_units: NonZeroU32,
    /// Maximum fully proved proposals returned for the resolved element.
    pub max_proposals: NonZeroU32,
}

impl Default for TargetResolutionOptions {
    fn default() -> Self {
        Self {
            max_work_units: NonZeroU32::new(100_000).expect("non-zero constant"),
            max_proposals: NonZeroU32::new(4).expect("non-zero constant"),
        }
    }
}

impl TargetResolutionOptions {
    /// Validates this atomic target-resolution request against published maxima.
    pub fn validate(&self) -> Result<(), ContractError> {
        if self.max_work_units.get() > MAX_WORK_UNITS
            || self.max_proposals.get() > MAX_PROPOSALS_PER_ELEMENT
        {
            return Err(ContractError::ExplorationLimitExceeded);
        }
        Ok(())
    }
}

/// Result of one fail-closed target resolution against a prepared snapshot.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TargetResolutionResult {
    /// Schema identity.
    pub schema_name: String,
    /// Schema version.
    pub schema_version: u32,
    /// Discovery identity of the prepared snapshot that resolved the target.
    pub discovery_identity_sha256: String,
    /// The exact resolved descriptor path.
    pub path: String,
    /// Fully proved proposals for the resolved target.
    pub selector_proposals: Vec<SelectorProposal>,
}
