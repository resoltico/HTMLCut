//! Named schema generation from the maintained serializable contract.

use serde_json::Value;

use crate::{ErrorCode, ExtractionError, ExtractionPlan, ExtractionResult};

/// Individually retrievable core schema names; discovery need not load a catalog of schemas.
pub const SCHEMA_NAMES: &[&str] = &[
    "htmlcut.extraction.plan",
    "htmlcut.extraction.result",
    "htmlcut.extraction.error",
    "htmlcut.inspection",
    "htmlcut.preview",
    "htmlcut.element_descriptor",
    "htmlcut.selector.proposal",
];

/// Generates one named schema; runtime validation owns cross-field invariants.
pub fn schema(name: &str) -> Result<Value, ExtractionError> {
    let schema = match name {
        "htmlcut.extraction.plan" => schemars::schema_for!(ExtractionPlan),
        "htmlcut.extraction.result" => schemars::schema_for!(ExtractionResult),
        "htmlcut.extraction.error" => schemars::schema_for!(ExtractionError),
        "htmlcut.inspection" => schemars::schema_for!(crate::InspectionResult),
        "htmlcut.preview" => schemars::schema_for!(crate::PreviewResult),
        "htmlcut.element_descriptor" => schemars::schema_for!(crate::ElementDescriptor),
        "htmlcut.selector.proposal" => schemars::schema_for!(crate::SelectorProposal),
        _ => {
            return Err(ExtractionError::new(
                ErrorCode::InvalidSchema,
                "schema",
                "The requested schema name is unsupported.",
            ));
        }
    };
    serde_json::to_value(schema).map_err(|_| {
        ExtractionError::new(
            ErrorCode::InternalInvariant,
            "schema",
            "The maintained schema could not be serialized.",
        )
    })
}
