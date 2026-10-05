// SPDX-License-Identifier: MPL-2.0
//! Immutable, bounded extraction over caller-supplied UTF-8 snapshots.
#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod compilation;
mod data;
mod discovery;
#[cfg(any(test, doctest))]
mod doctests;
mod execution;
mod failure_cause;
mod identity;
mod json;
mod limits;
mod plan;
mod projection;
mod receipt;
mod result;
mod schemas;
mod snapshot;
#[cfg(test)]
#[path = "tests/extraction_contract.rs"]
mod tests;

pub use compilation::CompiledPlan;
pub use data::{ExtractionData, FieldValue};
pub use discovery::{
    IdentifierInspectionResult, IdentifierInspectionSample, InspectionResult, InspectionSample,
    OutlineElement, OutlineGroup, OutlineResult, OutlineSample, TableShape,
};
pub use failure_cause::{
    ConfigurationProblem, ConfigurationRole, FailureCause, IoOperation, IoProblem,
};
pub use identity::canonical_json;
pub use json::parse_closed_json;
pub use limits::{
    ExecutionLimits, MAX_DATA_BYTES, MAX_PLAN_BYTES, MAX_RECEIPT_BYTES, MAX_RECORD_FIELDS,
    MAX_SOURCE_BYTES, PreparationLimits,
};
pub use plan::{
    Boundary, ExtractionPlan, FieldSelection, Guard, GuardRead, GuardScope, Predicate, Projection,
    RecordField, Selection, Strategy, Transform, ValueProjection,
};
pub use receipt::{DataKind, ExecutionReceipt, ExtractionResult, FieldCount, SourceRange};
pub use result::{ErrorCode, ErrorEvidence, ExtractionError, SCHEMA_VERSION, SEMANTICS_VERSION};
pub use schemas::{SCHEMA_NAMES, schema};
pub use snapshot::{PreparedDocument, SnapshotMetadata, SourceSnapshot};
