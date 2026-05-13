use super::*;

#[test]
fn request_documents_without_http_client_feature_reject_url_inputs() {
    let error = serde_json::from_value::<crate::wire::v1::ExtractionDefinitionDocument>(json!({
        "schema_name": crate::EXTRACTION_DEFINITION_SCHEMA_NAME,
        "schema_version": crate::EXTRACTION_DEFINITION_SCHEMA_VERSION,
        "request": {
            "spec_version": crate::CORE_SPEC_VERSION,
            "source": {
                "input": {
                    "type": "url",
                    "href": "https://example.com/articles"
                }
            },
            "extraction": {
                "kind": "selector",
                "selector": "article"
            }
        },
        "runtime": {}
    }))
    .expect_err("url request should be rejected when http-client is disabled");

    assert!(error.to_string().contains("unknown variant `url`"));
}

#[test]
fn source_request_schema_without_http_client_feature_omits_url_variants() {
    let schema = crate::schema_descriptor(
        crate::SOURCE_REQUEST_SCHEMA_NAME,
        crate::CORE_REQUEST_SCHEMA_VERSION,
    )
    .expect("source request schema descriptor");
    let schema_json = (schema.json_schema)().expect("source request schema");
    let source_input_document = &schema_json["$defs"]["SourceInputDocument"];

    assert!(!source_input_document.to_string().contains("\"url\""));
}
