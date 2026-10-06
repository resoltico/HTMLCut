// SPDX-License-Identifier: MPL-2.0
//! Shared public-boundary query setup; expected values stay in the independent scenarios.
use htmlcut_core::*;
use serde_json::Value;
pub(super) fn document(source: &str) -> PreparedDocument {
    PreparedDocument::new(
        SourceSnapshot::new(source, SnapshotMetadata::default()).unwrap(),
        PreparationLimits::default(),
    )
    .unwrap()
}
pub(super) fn compile(value: Value) -> CompiledPlan {
    CompiledPlan::compile(&ExtractionPlan::from_json(&serde_json::to_vec(&value).unwrap()).unwrap())
        .unwrap()
}
pub(super) fn data(source: &str, query: Value) -> Value {
    serde_json::from_slice(document(source).execute(&compile(query)).unwrap().payload()).unwrap()
}
