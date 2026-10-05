// SPDX-License-Identifier: MPL-2.0
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

/// Aggregate counts for one declared record field, not duplicated labels or values.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FieldCount {
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
    /// Aggregate counts keyed by validated field name in lexical order.
    pub fields: std::collections::BTreeMap<String, FieldCount>,
}

/// Immutable requested data, canonical payload, and lazy execution-bound evidence.
pub struct ExtractionResult {
    pub(crate) data: crate::ExtractionData,
    pub(crate) payload: Vec<u8>,
    pub(crate) source: crate::SourceSnapshot,
    pub(crate) query: std::sync::Arc<str>,
    pub(crate) preparation: crate::PreparationLimits,
    pub(crate) candidate_count: u32,
    pub(crate) selected_count: u32,
    pub(crate) fields: std::collections::BTreeMap<String, FieldCount>,
    pub(crate) budget: selectors::work_budget::SelectorWorkBudget,
    pub(crate) evidence: std::cell::OnceCell<Result<Evidence, crate::ExtractionError>>,
}
pub(crate) struct Evidence {
    receipt: ExecutionReceipt,
    bytes: Vec<u8>,
}
impl ExtractionResult {
    /// Borrows immutable typed data.
    pub fn data(&self) -> &crate::ExtractionData {
        &self.data
    }
    /// Consumes the result, dropping encoding and evidence residency.
    pub fn into_data(self) -> crate::ExtractionData {
        self.data
    }
    /// Canonical JSON payload, without the transport framing LF.
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }
    /// Complete original-root candidate count.
    pub fn candidate_count(&self) -> u32 {
        self.candidate_count
    }
    /// Complete selected-root count.
    pub fn selected_count(&self) -> u32 {
        self.selected_count
    }
    /// Retains empty values versus records distinction.
    pub fn data_kind(&self) -> DataKind {
        match self.data {
            crate::ExtractionData::Values(_) => DataKind::Values,
            crate::ExtractionData::Records(_) => DataKind::Records,
        }
    }
    /// Actual accepted source and metadata captured at execution.
    pub fn snapshot(&self) -> &crate::SourceSnapshot {
        &self.source
    }
    /// Actual preparation policy captured at execution.
    pub fn preparation_limits(&self) -> &crate::PreparationLimits {
        &self.preparation
    }
    /// Normalized query bytes captured at compilation and execution.
    pub fn normalized_json(&self) -> &str {
        &self.query
    }
    /// Generates evidence from this execution using its residual budget; caches success and failure.
    pub fn receipt(&self) -> Result<&ExecutionReceipt, crate::ExtractionError> {
        Ok(&self.evidence()?.receipt)
    }
    /// Bounded receipt JSON without LF, retained from the same evidence transition.
    pub fn receipt_payload(&self) -> Result<&[u8], crate::ExtractionError> {
        Ok(&self.evidence()?.bytes)
    }
    fn evidence(&self) -> Result<&Evidence, crate::ExtractionError> {
        self.evidence
            .get_or_init(|| self.build_evidence())
            .as_ref()
            .map_err(Clone::clone)
    }
    fn build_evidence(&self) -> Result<Evidence, crate::ExtractionError> {
        let metadata = crate::canonical_json(self.source.metadata())?;
        let preparation = crate::canonical_json(&self.preparation)?;
        // Cached source/query digests never discount their logical hash work.
        crate::execution::charge(&self.budget, self.source.html().len().div_ceil(64))?;
        let source_sha256 = self.source.source_sha256().to_owned();
        let plan_sha256 = crate::identity::budgeted_framed(
            "htmlcut.plan/6",
            &[self.query.as_bytes()],
            &self.budget,
        )?;
        let semantics = SEMANTICS_VERSION.to_be_bytes();
        let extraction_sha256 = crate::identity::budgeted_framed(
            "htmlcut.extraction/6",
            &[
                source_sha256.as_bytes(),
                plan_sha256.as_bytes(),
                metadata.as_bytes(),
                preparation.as_bytes(),
                &semantics,
            ],
            &self.budget,
        )?;
        crate::execution::charge(&self.budget, self.payload.len().div_ceil(64))?;
        let receipt = ExecutionReceipt {
            schema: "htmlcut.extraction.receipt".into(),
            version: SCHEMA_VERSION,
            semantics: SEMANTICS_VERSION,
            data_kind: self.data_kind(),
            source_sha256,
            plan_sha256,
            extraction_sha256,
            data_sha256: crate::identity::sha256(&self.payload),
            candidate_count: self.candidate_count,
            selected_count: self.selected_count,
            fields: self.fields.clone(),
        };
        let bytes = crate::identity::encoded(&receipt, crate::MAX_RECEIPT_BYTES, &self.budget)?;
        Ok(Evidence { receipt, bytes })
    }
}

impl std::fmt::Debug for ExtractionResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExtractionResult")
            .field("data", &self.data)
            .field("candidate_count", &self.candidate_count)
            .field("selected_count", &self.selected_count)
            .finish_non_exhaustive()
    }
}
