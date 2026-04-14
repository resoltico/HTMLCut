use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
    pub json_schema: fn() -> Value,
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

const SCHEMA_CATALOG: &[SchemaDescriptor] = &[
    SchemaDescriptor {
        schema_ref: SOURCE_REQUEST_SCHEMA_REF,
        owner_surface: "htmlcut-core",
        rust_shape: "SourceRequest",
        stability: SchemaStability::Versioned,
        json_schema: source_request_schema,
    },
    SchemaDescriptor {
        schema_ref: RUNTIME_OPTIONS_SCHEMA_REF,
        owner_surface: "htmlcut-core",
        rust_shape: "RuntimeOptions",
        stability: SchemaStability::Versioned,
        json_schema: runtime_options_schema,
    },
    SchemaDescriptor {
        schema_ref: INSPECTION_OPTIONS_SCHEMA_REF,
        owner_surface: "htmlcut-core",
        rust_shape: "InspectionOptions",
        stability: SchemaStability::Versioned,
        json_schema: inspection_options_schema,
    },
    SchemaDescriptor {
        schema_ref: EXTRACTION_REQUEST_SCHEMA_REF,
        owner_surface: "htmlcut-core",
        rust_shape: "ExtractionRequest",
        stability: SchemaStability::Versioned,
        json_schema: extraction_request_schema,
    },
    SchemaDescriptor {
        schema_ref: EXTRACTION_DEFINITION_SCHEMA_REF,
        owner_surface: "htmlcut-core",
        rust_shape: "ExtractionDefinition",
        stability: SchemaStability::Versioned,
        json_schema: extraction_definition_schema,
    },
    SchemaDescriptor {
        schema_ref: EXTRACTION_RESULT_SCHEMA_REF,
        owner_surface: "htmlcut-core",
        rust_shape: "ExtractionResult",
        stability: SchemaStability::Versioned,
        json_schema: extraction_result_schema,
    },
    SchemaDescriptor {
        schema_ref: SOURCE_INSPECTION_RESULT_SCHEMA_REF,
        owner_surface: "htmlcut-core",
        rust_shape: "SourceInspectionResult",
        stability: SchemaStability::Versioned,
        json_schema: source_inspection_result_schema,
    },
    SchemaDescriptor {
        schema_ref: INTEROP_PLAN_SCHEMA_REF,
        owner_surface: "htmlcut_core::interop::v1",
        rust_shape: "Plan",
        stability: SchemaStability::Frozen,
        json_schema: interop_plan_schema,
    },
    SchemaDescriptor {
        schema_ref: INTEROP_RESULT_SCHEMA_REF,
        owner_surface: "htmlcut_core::interop::v1",
        rust_shape: "InteropResult",
        stability: SchemaStability::Frozen,
        json_schema: interop_result_schema,
    },
    SchemaDescriptor {
        schema_ref: INTEROP_ERROR_SCHEMA_REF,
        owner_surface: "htmlcut_core::interop::v1",
        rust_shape: "InteropError",
        stability: SchemaStability::Frozen,
        json_schema: interop_error_schema,
    },
];

/// Returns the exported core-side schema catalog.
pub const fn schema_catalog() -> &'static [SchemaDescriptor] {
    SCHEMA_CATALOG
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

fn schema_json_for<T: JsonSchema>() -> Value {
    serde_json::to_value(schema_for!(T)).expect("JSON Schema documents should always serialize")
}

fn source_request_schema() -> Value {
    schema_json_for::<SourceRequest>()
}

fn runtime_options_schema() -> Value {
    schema_json_for::<RuntimeOptions>()
}

fn inspection_options_schema() -> Value {
    schema_json_for::<InspectionOptions>()
}

fn extraction_request_schema() -> Value {
    schema_json_for::<ExtractionRequest>()
}

fn extraction_definition_schema() -> Value {
    schema_json_for::<ExtractionDefinition>()
}

fn extraction_result_schema() -> Value {
    schema_json_for::<ExtractionResult>()
}

fn source_inspection_result_schema() -> Value {
    schema_json_for::<SourceInspectionResult>()
}

fn interop_plan_schema() -> Value {
    schema_json_for::<Plan>()
}

fn interop_result_schema() -> Value {
    schema_json_for::<InteropResult>()
}

fn interop_error_schema() -> Value {
    schema_json_for::<InteropError>()
}
