//! Test-only schema catalog constructors and failure probes.

use super::*;

pub(crate) fn schema_export_serialize_error_for_tests(schema_ref: SchemaRef) -> SchemaExportError {
    schema_export_serialize_error(
        schema_ref,
        serde_json::Error::io(std::io::Error::other(
            "synthetic schema serialization failure",
        )),
    )
}

pub(crate) fn expected_schema_contract_family_for_tests(
    schema_ref: SchemaRef,
) -> Option<&'static str> {
    schema_catalog()
        .iter()
        .find(|descriptor| descriptor.schema_ref == schema_ref)
        .map(|descriptor| descriptor.contract_family)
}

pub(crate) fn catalog_schema_descriptor_for_tests(
    schema_ref: SchemaRef,
    owner: &'static str,
    contract_family: &'static str,
    json_schema: fn() -> Result<Value, SchemaExportError>,
) -> SchemaDescriptor {
    catalog_schema_descriptor(schema_ref, owner, contract_family, json_schema)
}

pub(crate) fn assert_schema_catalog_contract_strings_for_tests(catalog: &[SchemaDescriptor]) {
    let errors = schema_catalog_contract_string_errors(catalog);
    assert!(
        errors.is_empty(),
        "schema catalog contract strings drifted:\n- {}",
        errors.join("\n- ")
    );
}
