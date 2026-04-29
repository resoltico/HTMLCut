use super::*;

#[test]
fn schema_catalog_is_unique_and_covers_core_and_interop_contracts() {
    let identities = schema_catalog()
        .iter()
        .map(|descriptor| {
            (
                descriptor.schema_ref.schema_name,
                descriptor.schema_ref.schema_version,
            )
        })
        .collect::<BTreeSet<_>>();

    assert_eq!(identities.len(), schema_catalog().len());
    assert!(identities.contains(&(SOURCE_REQUEST_SCHEMA_NAME, CORE_REQUEST_SCHEMA_VERSION)));
    assert!(identities.contains(&(RUNTIME_OPTIONS_SCHEMA_NAME, CORE_REQUEST_SCHEMA_VERSION)));
    assert!(identities.contains(&(EXTRACTION_REQUEST_SCHEMA_NAME, CORE_REQUEST_SCHEMA_VERSION)));
    assert!(identities.contains(&(CORE_RESULT_SCHEMA_NAME, CORE_RESULT_SCHEMA_VERSION)));
    assert!(identities.contains(&(
        CORE_SOURCE_INSPECTION_SCHEMA_NAME,
        CORE_SOURCE_INSPECTION_SCHEMA_VERSION,
    )));
    assert!(identities.contains(&(
        interop::v1::PLAN_SCHEMA_NAME,
        interop::v1::PLAN_SCHEMA_VERSION,
    )));
    assert!(identities.contains(&(
        interop::v1::RESULT_SCHEMA_NAME,
        interop::v1::RESULT_SCHEMA_VERSION,
    )));
    assert!(identities.contains(&(
        interop::v1::ERROR_SCHEMA_NAME,
        interop::v1::ERROR_SCHEMA_VERSION,
    )));

    let extraction_result_schema =
        schema_descriptor(CORE_RESULT_SCHEMA_NAME, CORE_RESULT_SCHEMA_VERSION)
            .expect("extraction result schema");
    assert_eq!(extraction_result_schema.owner_surface, "htmlcut-core");
    assert_eq!(extraction_result_schema.rust_shape, "ExtractionResult");

    let interop_result_schema = schema_descriptor(
        interop::v1::RESULT_SCHEMA_NAME,
        interop::v1::RESULT_SCHEMA_VERSION,
    )
    .expect("interop result schema");
    assert_eq!(
        interop_result_schema.owner_surface,
        "htmlcut_core::interop::v1"
    );
    assert_eq!(interop_result_schema.stability, SchemaStability::Versioned);
}
#[test]
fn schemas_cover_inner_html_and_structured_metadata_variants() {
    let extraction_request_schema =
        (schema_descriptor(EXTRACTION_REQUEST_SCHEMA_NAME, CORE_REQUEST_SCHEMA_VERSION)
            .expect("extraction request schema")
            .json_schema)()
        .expect("request schema json");
    let value_spec_variants = extraction_request_schema["$defs"]["ValueSpec"]["oneOf"]
        .as_array()
        .expect("value spec variants");
    let serialized_value_modes = value_spec_variants
        .iter()
        .filter_map(|variant| variant.pointer("/properties/type/const"))
        .filter_map(Value::as_str)
        .collect::<BTreeSet<_>>();
    assert!(serialized_value_modes.contains("inner-html"));
    assert!(!serialized_value_modes.contains("html"));

    let extraction_result_schema =
        (schema_descriptor(CORE_RESULT_SCHEMA_NAME, CORE_RESULT_SCHEMA_VERSION)
            .expect("extraction result schema")
            .json_schema)()
        .expect("result schema json");
    let metadata_variants = extraction_result_schema["$defs"]["ExtractionMatchMetadata"]["oneOf"]
        .as_array()
        .expect("metadata variants");
    let metadata_kinds = metadata_variants
        .iter()
        .filter_map(|variant| variant.pointer("/properties/kind/const"))
        .filter_map(Value::as_str)
        .collect::<BTreeSet<_>>();
    assert_eq!(
        metadata_kinds,
        BTreeSet::from(["delimiter-pair", "selector"])
    );

    let value_type_variants = extraction_result_schema["$defs"]["ValueType"]["oneOf"]
        .as_array()
        .expect("value type variants");
    let serialized_value_types = value_type_variants
        .iter()
        .filter_map(|variant| variant.get("const"))
        .filter_map(Value::as_str)
        .collect::<BTreeSet<_>>();
    assert!(serialized_value_types.contains("inner-html"));
    assert!(!serialized_value_types.contains("html"));
}

#[test]
fn schema_export_errors_preserve_schema_identity_and_message() {
    let schema_ref = SchemaRef::new("htmlcut.synthetic_schema", 42);
    let error = crate::schema::schema_export_serialize_error_for_tests(schema_ref);

    assert_eq!(
        error,
        SchemaExportError::Serialize {
            schema_name: "htmlcut.synthetic_schema",
            schema_version: 42,
            message: "synthetic schema serialization failure".to_owned(),
        }
    );
    assert!(error.to_string().contains("htmlcut.synthetic_schema@42"));
}
