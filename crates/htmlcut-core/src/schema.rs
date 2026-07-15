#[cfg(test)]
use std::collections::BTreeSet;

use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use thiserror::Error;

use crate::contracts::{
    CORE_RESULT_SCHEMA_NAME, CORE_RESULT_SCHEMA_VERSION, CORE_SOURCE_INSPECTION_SCHEMA_NAME,
    CORE_SOURCE_INSPECTION_SCHEMA_VERSION, CORE_SPEC_VERSION,
};
use crate::interop::v1::{
    ERROR_SCHEMA_NAME, ERROR_SCHEMA_VERSION, InteropError, InteropResult, PLAN_SCHEMA_NAME,
    PLAN_SCHEMA_VERSION, Plan, RESULT_SCHEMA_NAME, RESULT_SCHEMA_VERSION,
};
use crate::wire::v1::{
    ExtractionDefinitionDocument, ExtractionRequestDocument, ExtractionResultDocument,
    InspectionOptionsDocument, RuntimeOptionsDocument, SourceInspectionResultDocument,
    SourceRequestDocument,
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
pub const EXTRACTION_DEFINITION_SCHEMA_VERSION: u32 = 4;

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

    #[test]
    fn semantic_constraint_patching_appends_and_rejects_malformed_derived_schemas() {
        let schema_ref = SchemaRef::new("htmlcut.fixture", 7);
        let mut schema = json!({ "allOf": [] });
        append_semantic_constraints(&mut schema, schema_ref, [json!({ "const": true })])
            .expect("array allOf accepts additional constraints");
        assert_eq!(schema["allOf"], json!([{ "const": true }]));

        for schema in [json!(null), json!({ "allOf": { "unexpected": true } })] {
            let error = apply_plan_semantic_constraints(schema)
                .expect_err("malformed derived schemas must fail closed");
            assert_eq!(
                error,
                SchemaExportError::DerivedSchemaShape {
                    schema_name: PLAN_SCHEMA_NAME,
                    schema_version: PLAN_SCHEMA_VERSION,
                }
            );
        }

        let error = apply_result_semantic_constraints(json!(null))
            .expect_err("result schema patching must also fail closed");
        assert_eq!(
            error,
            SchemaExportError::DerivedSchemaShape {
                schema_name: RESULT_SCHEMA_NAME,
                schema_version: RESULT_SCHEMA_VERSION,
            }
        );
    }
}

/// Stability class for one exported schema.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum SchemaStability {
    /// Generic HTMLCut contract that is versioned and may hard-break by version.
    Versioned,
}

/// Runtime descriptor for one exported schema document.
#[derive(Clone, Copy, Debug)]
pub struct SchemaDescriptor {
    /// Stable schema identity.
    pub schema_ref: SchemaRef,
    /// Public owner label for the contract family.
    pub owner: &'static str,
    /// Public contract family name exposed to operators and embedders.
    pub contract_family: &'static str,
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
    /// The derived JSON Schema document did not have the root shape required for an HTMLCut
    /// contract constraint.
    #[error(
        "Derived JSON schema {schema_name}@{schema_version} has an incompatible root shape for HTMLCut contract constraints"
    )]
    DerivedSchemaShape {
        /// Stable schema name being exported.
        schema_name: &'static str,
        /// Stable schema version being exported.
        schema_version: u32,
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

const SCHEMA_CATALOG: &[SchemaDescriptor] = &[
    catalog_schema_descriptor(
        SOURCE_REQUEST_SCHEMA_REF,
        "core",
        "source request",
        source_request_schema,
    ),
    catalog_schema_descriptor(
        RUNTIME_OPTIONS_SCHEMA_REF,
        "core",
        "runtime options",
        runtime_options_schema,
    ),
    catalog_schema_descriptor(
        INSPECTION_OPTIONS_SCHEMA_REF,
        "core",
        "inspection options",
        inspection_options_schema,
    ),
    catalog_schema_descriptor(
        EXTRACTION_REQUEST_SCHEMA_REF,
        "core",
        "extraction request",
        extraction_request_schema,
    ),
    catalog_schema_descriptor(
        EXTRACTION_DEFINITION_SCHEMA_REF,
        "core",
        "extraction definition",
        extraction_definition_schema,
    ),
    catalog_schema_descriptor(
        EXTRACTION_RESULT_SCHEMA_REF,
        "core",
        "extraction result",
        extraction_result_schema,
    ),
    catalog_schema_descriptor(
        SOURCE_INSPECTION_RESULT_SCHEMA_REF,
        "core",
        "source inspection result",
        source_inspection_result_schema,
    ),
    catalog_schema_descriptor(
        INTEROP_PLAN_SCHEMA_REF,
        "interop-v1",
        "execution plan",
        interop_plan_schema,
    ),
    catalog_schema_descriptor(
        INTEROP_RESULT_SCHEMA_REF,
        "interop-v1",
        "execution result",
        interop_result_schema,
    ),
    catalog_schema_descriptor(
        INTEROP_ERROR_SCHEMA_REF,
        "interop-v1",
        "execution error",
        interop_error_schema,
    ),
];

/// Returns the exported core-side schema catalog.
pub fn schema_catalog() -> &'static [SchemaDescriptor] {
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
    schema_json_for::<SourceRequestDocument>(SOURCE_REQUEST_SCHEMA_REF)
}

fn runtime_options_schema() -> Result<Value, SchemaExportError> {
    schema_json_for::<RuntimeOptionsDocument>(RUNTIME_OPTIONS_SCHEMA_REF)
}

fn inspection_options_schema() -> Result<Value, SchemaExportError> {
    schema_json_for::<InspectionOptionsDocument>(INSPECTION_OPTIONS_SCHEMA_REF)
}

fn extraction_request_schema() -> Result<Value, SchemaExportError> {
    schema_json_for::<ExtractionRequestDocument>(EXTRACTION_REQUEST_SCHEMA_REF)
}

fn extraction_definition_schema() -> Result<Value, SchemaExportError> {
    schema_json_for::<ExtractionDefinitionDocument>(EXTRACTION_DEFINITION_SCHEMA_REF)
}

fn extraction_result_schema() -> Result<Value, SchemaExportError> {
    schema_json_for::<ExtractionResultDocument>(EXTRACTION_RESULT_SCHEMA_REF)
}

fn source_inspection_result_schema() -> Result<Value, SchemaExportError> {
    schema_json_for::<SourceInspectionResultDocument>(SOURCE_INSPECTION_RESULT_SCHEMA_REF)
}

fn interop_plan_schema() -> Result<Value, SchemaExportError> {
    schema_json_for::<Plan>(INTEROP_PLAN_SCHEMA_REF).and_then(apply_plan_semantic_constraints)
}

fn apply_plan_semantic_constraints(mut schema: Value) -> Result<Value, SchemaExportError> {
    append_semantic_constraints(
        &mut schema,
        INTEROP_PLAN_SCHEMA_REF,
        [plan_dom_canonicalization_constraint()],
    )?;
    Ok(schema)
}

fn interop_result_schema() -> Result<Value, SchemaExportError> {
    schema_json_for::<InteropResult>(INTEROP_RESULT_SCHEMA_REF)
        .and_then(apply_result_semantic_constraints)
}

fn apply_result_semantic_constraints(mut schema: Value) -> Result<Value, SchemaExportError> {
    append_semantic_constraints(
        &mut schema,
        INTEROP_RESULT_SCHEMA_REF,
        [
            result_comparison_text_output_constraint(),
            result_output_value_constraint(&["text"], json!({ "type": "string" })),
            result_output_value_constraint(
                &["structured"],
                json!({
                    "allOf": [
                        { "type": "object" },
                        { "not": { "required": ["comparisonTextOutput"] } }
                    ]
                }),
            ),
            result_output_value_constraint(
                &["inner_html", "outer_html", "selected_html", "attribute"],
                json!({ "type": "string" }),
            ),
        ],
    )?;
    Ok(schema)
}

fn interop_error_schema() -> Result<Value, SchemaExportError> {
    schema_json_for::<InteropError>(INTEROP_ERROR_SCHEMA_REF)
}

/// Adds the H5 relationship that a canonicalization policy denotes a CSS text-comparison plan.
///
/// The ignored-attribute versus measured-attribute equality is intentionally not represented
/// here: standard JSON Schema cannot compare a dynamic array member with a sibling property.
fn plan_dom_canonicalization_constraint() -> Value {
    json!({
        "if": {
            "required": ["dom_canonicalization"],
            "properties": {
                "dom_canonicalization": { "$ref": "#/$defs/DomCanonicalization" }
            }
        },
        "then": {
            "required": ["strategy", "output"],
            "properties": {
                "strategy": {
                    "type": "object",
                    "required": ["kind"],
                    "properties": { "kind": { "const": "css_selector" } }
                },
                "output": {
                    "type": "object",
                    "required": ["kind"],
                    "properties": { "kind": { "enum": ["text", "structured"] } }
                }
            }
        }
    })
}

/// Restricts clone-text evidence to CSS text and structured results.
fn result_comparison_text_output_constraint() -> Value {
    json!({
        "if": {
            "allOf": [
                {
                    "required": ["strategy_kind"],
                    "properties": { "strategy_kind": { "const": "css_selector" } }
                },
                output_kind_constraint(&["text", "structured"])
            ]
        },
        "else": {
            "properties": {
                "selected_matches": {
                    "items": {
                        "properties": {
                            "comparison_text_output": { "type": "null" }
                        }
                    }
                }
            }
        }
    })
}

/// Constrains the representation of each selected output payload for one or more output kinds.
fn result_output_value_constraint(output_kinds: &[&str], output_value_schema: Value) -> Value {
    json!({
        "if": output_kind_constraint(output_kinds),
        "then": {
            "properties": {
                "selected_matches": {
                    "items": {
                        "properties": { "output_value": output_value_schema }
                    }
                }
            }
        }
    })
}

/// Builds a discriminator check for the tagged interop output union.
fn output_kind_constraint(output_kinds: &[&str]) -> Value {
    json!({
        "required": ["output"],
        "properties": {
            "output": {
                "type": "object",
                "required": ["kind"],
                "properties": { "kind": { "enum": output_kinds } }
            }
        }
    })
}

/// Appends HTMLCut-owned semantic constraints to a derived root schema.
fn append_semantic_constraints(
    schema: &mut Value,
    schema_ref: SchemaRef,
    constraints: impl IntoIterator<Item = Value>,
) -> Result<(), SchemaExportError> {
    let Some(schema_object) = schema.as_object_mut() else {
        return Err(SchemaExportError::DerivedSchemaShape {
            schema_name: schema_ref.schema_name,
            schema_version: schema_ref.schema_version,
        });
    };

    match schema_object.get_mut("allOf") {
        Some(Value::Array(existing_constraints)) => {
            existing_constraints.extend(constraints);
        }
        Some(_) => {
            return Err(SchemaExportError::DerivedSchemaShape {
                schema_name: schema_ref.schema_name,
                schema_version: schema_ref.schema_version,
            });
        }
        None => {
            schema_object.insert(
                "allOf".to_owned(),
                Value::Array(constraints.into_iter().collect()),
            );
        }
    }

    Ok(())
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
pub(crate) fn expected_schema_contract_family_for_tests(
    schema_ref: SchemaRef,
) -> Option<&'static str> {
    schema_catalog()
        .iter()
        .find(|descriptor| descriptor.schema_ref == schema_ref)
        .map(|descriptor| descriptor.contract_family)
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
const fn catalog_schema_descriptor(
    schema_ref: SchemaRef,
    owner: &'static str,
    contract_family: &'static str,
    json_schema: fn() -> Result<Value, SchemaExportError>,
) -> SchemaDescriptor {
    SchemaDescriptor {
        schema_ref,
        owner,
        contract_family,
        stability: SchemaStability::Versioned,
        json_schema,
    }
}

#[cfg(test)]
pub(crate) fn catalog_schema_descriptor_for_tests(
    schema_ref: SchemaRef,
    owner: &'static str,
    contract_family: &'static str,
    json_schema: fn() -> Result<Value, SchemaExportError>,
) -> SchemaDescriptor {
    catalog_schema_descriptor(schema_ref, owner, contract_family, json_schema)
}
