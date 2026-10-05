// SPDX-License-Identifier: MPL-2.0
//! Named schema generation from the maintained serializable contract.

use serde_json::Value;

use crate::{ErrorCode, ExtractionData, ExtractionError, ExtractionPlan};

/// Individually retrievable core schema names; discovery need not load a catalog of schemas.
pub const SCHEMA_NAMES: &[&str] = &[
    "htmlcut.extraction.plan",
    "htmlcut.extraction.data",
    "htmlcut.extraction.receipt",
    "htmlcut.extraction.error",
    "htmlcut.inspection",
    "htmlcut.inspection.identifiers",
];

/// Generates one named schema; runtime validation owns cross-field invariants.
pub fn schema(name: &str) -> Result<Value, ExtractionError> {
    let schema = match name {
        "htmlcut.extraction.plan" => schemars::schema_for!(ExtractionPlan),
        "htmlcut.extraction.data" => schemars::schema_for!(ExtractionData),
        "htmlcut.extraction.error" => schemars::schema_for!(ExtractionError),
        "htmlcut.inspection" => schemars::schema_for!(crate::InspectionResult),
        "htmlcut.inspection.identifiers" => {
            schemars::schema_for!(crate::IdentifierInspectionResult)
        }
        "htmlcut.extraction.receipt" => schemars::schema_for!(crate::ExecutionReceipt),
        _ => {
            return Err(ExtractionError::new(
                ErrorCode::InvalidSchema,
                "schema",
                "The requested schema name is unsupported.",
            ));
        }
    };
    Ok(schema.as_value().clone())
}
