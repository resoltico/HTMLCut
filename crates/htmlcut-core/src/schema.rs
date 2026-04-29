#[cfg(test)]
use std::collections::BTreeSet;
use std::sync::LazyLock;

use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::any::type_name;
use thiserror::Error;

use crate::contracts::{
    CORE_RESULT_SCHEMA_NAME, CORE_RESULT_SCHEMA_VERSION, CORE_SOURCE_INSPECTION_SCHEMA_NAME,
    CORE_SOURCE_INSPECTION_SCHEMA_VERSION, CORE_SPEC_VERSION, ExtractionDefinition,
    ExtractionRequest, ExtractionResult, InspectionOptions, RuntimeOptions, SourceInspectionResult,
    SourceRequest,
};
use crate::interop::v1::{
    ERROR_SCHEMA_NAME, ERROR_SCHEMA_VERSION, InteropError, InteropResult, PLAN_SCHEMA_NAME,
    PLAN_SCHEMA_VERSION, Plan, RESULT_SCHEMA_NAME, RESULT_SCHEMA_VERSION,
};

/// Versioned schema-registry profile exported by HTMLCut.
pub const HTMLCUT_JSON_SCHEMA_PROFILE: &str = "htmlcut-json-schema-v1";
/// Frozen schema name for [`crate::SourceRequest`].
pub const SOURCE_REQUEST_SCHEMA_NAME: &str = "htmlcut.source_request";
/// Frozen schema name for [`crate::RuntimeOptions`].
pub const RUNTIME_OPTIONS_SCHEMA_NAME: &str = "htmlcut.runtime_options";
/// Frozen schema name for [`crate::InspectionOptions`].
pub const INSPECTION_OPTIONS_SCHEMA_NAME: &str = "htmlcut.inspection_options";
/// Frozen schema name for [`crate::ExtractionRequest`].
pub const EXTRACTION_REQUEST_SCHEMA_NAME: &str = "htmlcut.extraction_request";
/// Frozen schema name for [`crate::ExtractionDefinition`].
pub const EXTRACTION_DEFINITION_SCHEMA_NAME: &str = "htmlcut.extraction_definition";
/// Schema version for request-side core contracts.
pub const CORE_REQUEST_SCHEMA_VERSION: u32 = CORE_SPEC_VERSION;
/// Schema version for reusable extraction definitions.
pub const EXTRACTION_DEFINITION_SCHEMA_VERSION: u32 = 1;

/// Stable reference to one versioned schema document.
#[derive(
    Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub struct SchemaRef {
    /// Stable schema name.
    pub schema_name: &'static str,
    /// Stable schema version.
    pub schema_version: u32,
}

impl SchemaRef {
    /// Builds one stable schema reference.
    pub const fn new(schema_name: &'static str, schema_version: u32) -> Self {
        Self {
            schema_name,
            schema_version,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_ref_constructor_keeps_name_and_version() {
        let schema_ref = SchemaRef::new("htmlcut.fixture", 7);

        assert_eq!(schema_ref.schema_name, "htmlcut.fixture");
        assert_eq!(schema_ref.schema_version, 7);
    }
}

/// Stability class for one exported schema.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum SchemaStability {
    /// Generic HTMLCut contract that is versioned and may hard-break by version.
    Versioned,
    /// Frozen interop contract that must not mutate in place.
    Frozen,
}

/// Runtime descriptor for one exported schema document.
#[derive(Clone, Copy, Debug)]
pub struct SchemaDescriptor {
    /// Stable schema identity.
    pub schema_ref: SchemaRef,
    /// Surface that owns the contract.
    pub owner_surface: &'static str,
    /// Rust type or type composition that maps to this schema.
    pub rust_shape: &'static str,
    /// Stability class for the schema.
    pub stability: SchemaStability,
    /// Lazy builder for the JSON Schema document.
    pub json_schema: fn() -> Result<Value, SchemaExportError>,
}

/// Typed schema-export failure returned when JSON Schema materialization fails.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum SchemaExportError {
    /// The derived JSON Schema document could not be serialized into JSON.
    #[error("Could not serialize JSON schema {schema_name}@{schema_version}: {message}")]
    Serialize {
        /// Stable schema name being exported.
        schema_name: &'static str,
        /// Stable schema version being exported.
        schema_version: u32,
        /// Serializer failure message.
        message: String,
    },
}

const SOURCE_REQUEST_SCHEMA_REF: SchemaRef =
    SchemaRef::new(SOURCE_REQUEST_SCHEMA_NAME, CORE_REQUEST_SCHEMA_VERSION);
const RUNTIME_OPTIONS_SCHEMA_REF: SchemaRef =
    SchemaRef::new(RUNTIME_OPTIONS_SCHEMA_NAME, CORE_REQUEST_SCHEMA_VERSION);
const INSPECTION_OPTIONS_SCHEMA_REF: SchemaRef =
    SchemaRef::new(INSPECTION_OPTIONS_SCHEMA_NAME, CORE_REQUEST_SCHEMA_VERSION);
const EXTRACTION_REQUEST_SCHEMA_REF: SchemaRef =
    SchemaRef::new(EXTRACTION_REQUEST_SCHEMA_NAME, CORE_REQUEST_SCHEMA_VERSION);
const EXTRACTION_DEFINITION_SCHEMA_REF: SchemaRef = SchemaRef::new(
    EXTRACTION_DEFINITION_SCHEMA_NAME,
    EXTRACTION_DEFINITION_SCHEMA_VERSION,
);
const EXTRACTION_RESULT_SCHEMA_REF: SchemaRef =
    SchemaRef::new(CORE_RESULT_SCHEMA_NAME, CORE_RESULT_SCHEMA_VERSION);
const SOURCE_INSPECTION_RESULT_SCHEMA_REF: SchemaRef = SchemaRef::new(
    CORE_SOURCE_INSPECTION_SCHEMA_NAME,
    CORE_SOURCE_INSPECTION_SCHEMA_VERSION,
);
const INTEROP_PLAN_SCHEMA_REF: SchemaRef = SchemaRef::new(PLAN_SCHEMA_NAME, PLAN_SCHEMA_VERSION);
const INTEROP_RESULT_SCHEMA_REF: SchemaRef =
    SchemaRef::new(RESULT_SCHEMA_NAME, RESULT_SCHEMA_VERSION);
const INTEROP_ERROR_SCHEMA_REF: SchemaRef = SchemaRef::new(ERROR_SCHEMA_NAME, ERROR_SCHEMA_VERSION);

static SCHEMA_CATALOG: LazyLock<Vec<SchemaDescriptor>> = LazyLock::new(|| {
    vec![
        schema_descriptor_entry::<SourceRequest>(
            SOURCE_REQUEST_SCHEMA_REF,
            "htmlcut-core",
            SchemaStability::Versioned,
            source_request_schema,
        ),
        schema_descriptor_entry::<RuntimeOptions>(
            RUNTIME_OPTIONS_SCHEMA_REF,
            "htmlcut-core",
            SchemaStability::Versioned,
            runtime_options_schema,
        ),
        schema_descriptor_entry::<InspectionOptions>(
            INSPECTION_OPTIONS_SCHEMA_REF,
            "htmlcut-core",
            SchemaStability::Versioned,
            inspection_options_schema,
        ),
        schema_descriptor_entry::<ExtractionRequest>(
            EXTRACTION_REQUEST_SCHEMA_REF,
            "htmlcut-core",
            SchemaStability::Versioned,
            extraction_request_schema,
        ),
        schema_descriptor_entry::<ExtractionDefinition>(
            EXTRACTION_DEFINITION_SCHEMA_REF,
            "htmlcut-core",
            SchemaStability::Versioned,
            extraction_definition_schema,
        ),
        schema_descriptor_entry::<ExtractionResult>(
            EXTRACTION_RESULT_SCHEMA_REF,
            "htmlcut-core",
            SchemaStability::Versioned,
            extraction_result_schema,
        ),
        schema_descriptor_entry::<SourceInspectionResult>(
            SOURCE_INSPECTION_RESULT_SCHEMA_REF,
            "htmlcut-core",
            SchemaStability::Versioned,
            source_inspection_result_schema,
        ),
        schema_descriptor_entry::<Plan>(
            INTEROP_PLAN_SCHEMA_REF,
            "htmlcut_core::interop::v1",
            SchemaStability::Versioned,
            interop_plan_schema,
        ),
        schema_descriptor_entry::<InteropResult>(
            INTEROP_RESULT_SCHEMA_REF,
            "htmlcut_core::interop::v1",
            SchemaStability::Versioned,
            interop_result_schema,
        ),
        schema_descriptor_entry::<InteropError>(
            INTEROP_ERROR_SCHEMA_REF,
            "htmlcut_core::interop::v1",
            SchemaStability::Versioned,
            interop_error_schema,
        ),
    ]
});

/// Returns the exported core-side schema catalog.
pub fn schema_catalog() -> &'static [SchemaDescriptor] {
    SCHEMA_CATALOG.as_slice()
}

/// Returns one exported schema descriptor by exact name and version.
pub fn schema_descriptor(
    schema_name: &str,
    schema_version: u32,
) -> Option<&'static SchemaDescriptor> {
    schema_catalog().iter().find(|descriptor| {
        descriptor.schema_ref.schema_name == schema_name
            && descriptor.schema_ref.schema_version == schema_version
    })
}

fn schema_json_for<T: JsonSchema>(schema_ref: SchemaRef) -> Result<Value, SchemaExportError> {
    serde_json::to_value(schema_for!(T))
        .map_err(|error| schema_export_serialize_error(schema_ref, error))
}

fn schema_export_serialize_error(
    schema_ref: SchemaRef,
    error: serde_json::Error,
) -> SchemaExportError {
    SchemaExportError::Serialize {
        schema_name: schema_ref.schema_name,
        schema_version: schema_ref.schema_version,
        message: error.to_string(),
    }
}

#[cfg(test)]
pub(crate) fn schema_export_serialize_error_for_tests(schema_ref: SchemaRef) -> SchemaExportError {
    schema_export_serialize_error(
        schema_ref,
        serde_json::Error::io(std::io::Error::other(
            "synthetic schema serialization failure",
        )),
    )
}

fn source_request_schema() -> Result<Value, SchemaExportError> {
    schema_json_for::<SourceRequest>(SOURCE_REQUEST_SCHEMA_REF)
}

fn runtime_options_schema() -> Result<Value, SchemaExportError> {
    schema_json_for::<RuntimeOptions>(RUNTIME_OPTIONS_SCHEMA_REF)
}

fn inspection_options_schema() -> Result<Value, SchemaExportError> {
    schema_json_for::<InspectionOptions>(INSPECTION_OPTIONS_SCHEMA_REF)
}

fn extraction_request_schema() -> Result<Value, SchemaExportError> {
    schema_json_for::<ExtractionRequest>(EXTRACTION_REQUEST_SCHEMA_REF)
}

fn extraction_definition_schema() -> Result<Value, SchemaExportError> {
    schema_json_for::<ExtractionDefinition>(EXTRACTION_DEFINITION_SCHEMA_REF)
}

fn extraction_result_schema() -> Result<Value, SchemaExportError> {
    schema_json_for::<ExtractionResult>(EXTRACTION_RESULT_SCHEMA_REF)
}

fn source_inspection_result_schema() -> Result<Value, SchemaExportError> {
    schema_json_for::<SourceInspectionResult>(SOURCE_INSPECTION_RESULT_SCHEMA_REF)
}

fn interop_plan_schema() -> Result<Value, SchemaExportError> {
    schema_json_for::<Plan>(INTEROP_PLAN_SCHEMA_REF)
}

fn interop_result_schema() -> Result<Value, SchemaExportError> {
    schema_json_for::<InteropResult>(INTEROP_RESULT_SCHEMA_REF)
}

fn interop_error_schema() -> Result<Value, SchemaExportError> {
    schema_json_for::<InteropError>(INTEROP_ERROR_SCHEMA_REF)
}

#[cfg(test)]
pub(crate) fn schema_catalog_contract_string_errors_for_tests() -> Vec<String> {
    schema_catalog_contract_string_errors(schema_catalog())
}

#[cfg(test)]
pub(crate) fn schema_catalog_contract_string_errors_for_tests_with(
    catalog: &[SchemaDescriptor],
) -> Vec<String> {
    schema_catalog_contract_string_errors(catalog)
}

#[cfg(test)]
pub(crate) fn assert_schema_catalog_contract_strings_for_tests(catalog: &[SchemaDescriptor]) {
    let errors = schema_catalog_contract_string_errors(catalog);
    assert!(
        errors.is_empty(),
        "schema catalog contract strings drifted:\n- {}",
        errors.join("\n- ")
    );
}

#[cfg(test)]
pub(crate) fn expected_schema_rust_shape_for_tests(schema_ref: SchemaRef) -> Option<&'static str> {
    schema_catalog()
        .iter()
        .find(|descriptor| descriptor.schema_ref == schema_ref)
        .map(|descriptor| descriptor.rust_shape)
}

#[cfg(test)]
fn schema_catalog_contract_string_errors(catalog: &[SchemaDescriptor]) -> Vec<String> {
    let mut errors = Vec::new();
    let mut seen_refs = BTreeSet::new();
    let expected_refs = [
        SOURCE_REQUEST_SCHEMA_REF,
        RUNTIME_OPTIONS_SCHEMA_REF,
        INSPECTION_OPTIONS_SCHEMA_REF,
        EXTRACTION_REQUEST_SCHEMA_REF,
        EXTRACTION_DEFINITION_SCHEMA_REF,
        EXTRACTION_RESULT_SCHEMA_REF,
        SOURCE_INSPECTION_RESULT_SCHEMA_REF,
        INTEROP_PLAN_SCHEMA_REF,
        INTEROP_RESULT_SCHEMA_REF,
        INTEROP_ERROR_SCHEMA_REF,
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();

    for descriptor in catalog {
        if !seen_refs.insert(descriptor.schema_ref) {
            errors.push(format!(
                "{}@{} appears more than once in schema_catalog()",
                descriptor.schema_ref.schema_name, descriptor.schema_ref.schema_version
            ));
        }

        if !expected_refs.contains(&descriptor.schema_ref) {
            errors.push(format!(
                "{}@{} is not part of the maintained schema inventory",
                descriptor.schema_ref.schema_name, descriptor.schema_ref.schema_version
            ));
        }
    }

    for schema_ref in expected_refs {
        if !seen_refs.contains(&schema_ref) {
            errors.push(format!(
                "{}@{} is missing from schema_catalog()",
                schema_ref.schema_name, schema_ref.schema_version
            ));
        }
    }

    errors
}
fn schema_descriptor_entry<T>(
    schema_ref: SchemaRef,
    owner_surface: &'static str,
    stability: SchemaStability,
    json_schema: fn() -> Result<Value, SchemaExportError>,
) -> SchemaDescriptor {
    SchemaDescriptor {
        schema_ref,
        owner_surface,
        rust_shape: short_type_name::<T>(),
        stability,
        json_schema,
    }
}

fn short_type_name<T>() -> &'static str {
    type_name::<T>()
        .rsplit("::")
        .next()
        .unwrap_or(type_name::<T>())
}
