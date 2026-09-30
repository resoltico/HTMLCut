//! Immutable, bounded extraction over caller-supplied UTF-8 snapshots.
#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod compilation;
mod discovery;
#[cfg(any(test, doctest))]
mod doctests;
mod execution;
mod identity;
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
pub use discovery::{
    AttributePreview, ElementDescriptor, InspectionResult, PreviewResult, SelectorProposal,
};
pub use identity::canonical_json;
pub use json::parse_closed_json;
pub use limits::{ExecutionLimits, PreparationLimits};
pub use plan::{
    Boundary, ExtractionPlan, Guard, GuardRead, GuardScope, Predicate, Projection, Selection,
    Strategy, Transform,
};
pub use result::{
    ErrorCode, ErrorEvidence, ExtractionError, ExtractionResult, SCHEMA_VERSION, SEMANTICS_VERSION,
    SourceRange,
};
pub use schemas::{SCHEMA_NAMES, schema};
pub use snapshot::{PreparedDocument, SnapshotMetadata, SourceSnapshot};
