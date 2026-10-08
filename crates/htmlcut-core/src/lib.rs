// SPDX-License-Identifier: MPL-2.0
//! Immutable, bounded extraction over caller-supplied UTF-8 snapshots.
#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod compilation;
mod data;
mod discovery;
#[cfg(any(test, doctest))]
mod doctests;
mod encoding;
mod execution;
mod extraction;
mod failure_cause;
mod json;
mod limits;
mod plan;
mod projection;
mod result;
mod schemas;
mod snapshot;
#[cfg(test)]
#[path = "tests/extraction_contract.rs"]
mod tests;

pub use compilation::CompiledPlan;
pub use data::{ExtractionData, FieldValue};
pub use discovery::{
    InspectionResult, InspectionSample, SurveyElement, SurveyGroup, SurveyResult, SurveySample,
    TableShape,
};
pub use encoding::canonical_json;
pub use extraction::{ExtractionResult, FieldCount};
pub use failure_cause::{
    ConfigurationProblem, ConfigurationRole, FailureCause, IoOperation, IoProblem,
};
pub use json::parse_closed_json;
pub use limits::{
    ExecutionLimits, MAX_DATA_BYTES, MAX_PLAN_BYTES, MAX_RECORD_FIELDS, MAX_SOURCE_BYTES,
    PreparationLimits,
};
pub use plan::{ExtractionPlan, FieldMatch, Guard, GuardScope, Match, Reading, RecordField};
pub use result::{ErrorCode, ErrorEvidence, ExtractionError, SCHEMA_VERSION, SEMANTICS_VERSION};
pub use schemas::{SCHEMA_NAMES, schema};
pub use snapshot::{PreparedDocument, SnapshotMetadata, SourceSnapshot};
