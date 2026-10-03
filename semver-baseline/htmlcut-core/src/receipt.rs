//! Deterministic execution evidence, separate from requested data and delivery.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{SCHEMA_VERSION, SEMANTICS_VERSION};

/// Interpretation of bare data, including an otherwise ambiguous empty array.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DataKind {
    /// A flat array of strings.
    Values,
    /// An array of named-field records.
    Records,
}

/// Half-open UTF-8 byte range in the exact accepted source.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SourceRange {
    /// Inclusive starting offset.
    pub start: usize,
    /// Exclusive ending offset.
    pub end: usize,
}

/// Aggregate counts for one declared record field, not duplicated labels or values.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FieldCount {
    /// Positive one-based declaration position.
    pub field_index: u32,
    /// Complete matched candidates across selected rows.
    pub candidate_count: u64,
    /// Strings actually projected, excluding null/array containers.
    pub projected_count: u64,
    /// Rows whose optional field was absent.
    pub absent_count: u64,
}

/// Evidence of execution and integrity; neither origin authentication nor delivery proof.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExecutionReceipt {
    /// Current receipt role.
    #[schemars(extend("const" = "htmlcut.extraction.receipt"))]
    pub schema: String,
    /// Current extraction wire family.
    #[schemars(extend("const" = SCHEMA_VERSION))]
    pub version: u32,
    /// Current extraction semantics.
    #[schemars(extend("const" = SEMANTICS_VERSION))]
    pub semantics: u32,
    /// Interpretation of bare data.
    pub data_kind: DataKind,
    /// SHA-256 of exact accepted source bytes.
    pub source_sha256: String,
    /// Identity of the normalized plan.
    pub plan_sha256: String,
    /// Identity binding source, plan, metadata, preparation and semantics.
    pub extraction_sha256: String,
    /// SHA-256 of compact canonical data bytes, excluding framing LF.
    pub data_sha256: String,
    /// Complete root candidate count.
    pub candidate_count: u32,
    /// Complete selected root count.
    pub selected_count: u32,
    /// Per-field aggregates in declaration order; empty for flat extraction.
    pub fields: Vec<FieldCount>,
    /// Exact selected source ranges; absent for DOM extraction.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ranges: Option<Vec<SourceRange>>,
}

/// Requested data plus independently consumable execution evidence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExtractionResult {
    /// Bare requested values or records.
    pub data: crate::ExtractionData,
    /// Deterministic execution facts, not a successful publication claim.
    pub receipt: ExecutionReceipt,
}
