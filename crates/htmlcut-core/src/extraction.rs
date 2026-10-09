// SPDX-License-Identifier: MPL-2.0
//! Owned query values and complete counts, independent of source and encoding.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
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

/// Complete typed query output. JSON encoding and delivery are separate operations.
#[derive(Debug)]
pub struct ExtractionResult {
    pub(crate) data: crate::ExtractionData,
    pub(crate) candidate_count: u32,
    pub(crate) selected_count: u32,
    pub(crate) fields: std::collections::BTreeMap<String, FieldCount>,
}
impl ExtractionResult {
    /// Borrows the complete typed values.
    pub fn data(&self) -> &crate::ExtractionData {
        &self.data
    }
    /// Consumes the result into its typed values.
    pub fn into_data(self) -> crate::ExtractionData {
        self.data
    }
    /// Complete root candidate count.
    pub fn candidate_count(&self) -> u32 {
        self.candidate_count
    }
    /// Selected root count.
    pub fn selected_count(&self) -> u32 {
        self.selected_count
    }
    /// Complete field counts in lexical name order.
    pub fn field_counts(&self) -> &std::collections::BTreeMap<String, FieldCount> {
        &self.fields
    }
}
