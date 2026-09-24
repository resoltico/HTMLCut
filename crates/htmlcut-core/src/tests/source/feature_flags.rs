use super::*;

#[test]
fn request_documents_without_http_client_feature_keep_url_shape_stable() {
    let definition =
        serde_json::from_value::<crate::wire::v2::ExtractionDefinitionDocument>(json!({
            "wire_profile": crate::HTMLCUT_JSON_SCHEMA_PROFILE,
            "schema_name": crate::EXTRACTION_DEFINITION_SCHEMA_NAME,
            "schema_version": crate::EXTRACTION_DEFINITION_SCHEMA_VERSION,
            "request": {
                "wire_profile": crate::HTMLCUT_JSON_SCHEMA_PROFILE,
                "schema_name": crate::EXTRACTION_REQUEST_SCHEMA_NAME,
                "schema_version": crate::CORE_REQUEST_SCHEMA_VERSION,
                "spec_version": crate::CORE_SPEC_VERSION,
                "source": {
                    "wire_profile": crate::HTMLCUT_JSON_SCHEMA_PROFILE,
                    "schema_name": crate::SOURCE_REQUEST_SCHEMA_NAME,
                    "schema_version": crate::CORE_REQUEST_SCHEMA_VERSION,
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
            "runtime": {
                "wire_profile": crate::HTMLCUT_JSON_SCHEMA_PROFILE,
                "schema_name": crate::RUNTIME_OPTIONS_SCHEMA_NAME,
                "schema_version": crate::CORE_REQUEST_SCHEMA_VERSION
            }
        }))
        .expect("url request document");

    let request: crate::ExtractionDefinition = definition
        .try_into()
        .expect("current extraction definition document");
    assert!(matches!(
        request.request.source.input,
        crate::SourceInput::Url { .. }
    ));
}

#[test]
fn source_request_schema_without_http_client_feature_keeps_url_variants() {
    let schema = crate::schema_descriptor(
        crate::SOURCE_REQUEST_SCHEMA_NAME,
        crate::CORE_REQUEST_SCHEMA_VERSION,
    )
    .expect("source request schema descriptor");
    let schema_json = (schema.json_schema)().expect("source request schema");
    let source_input_document = &schema_json["$defs"]["SourceInputDocument"];

    assert!(source_input_document.to_string().contains("\"url\""));
}

#[test]
fn url_requests_without_http_client_fail_at_execution_time() {
    let request = crate::ExtractionRequest::new(
        crate::SourceRequest::url(
            crate::HttpUrl::parse("https://example.com/articles").expect("http url"),
        ),
        crate::ExtractionSpec::selector(crate::SelectorQuery::new("article").expect("selector")),
    );

    let result = crate::extract(&request, &crate::RuntimeOptions::default());

    assert!(!result.ok);
    assert_eq!(result.source.kind, crate::SourceKind::Url);
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(
        result.diagnostics[0].code,
        crate::DiagnosticCode::SourceLoadFailed
    );
    assert!(
        result.diagnostics[0]
            .message
            .contains("does not include the built-in HTTP(S) source loader")
    );
    assert_eq!(
        result.diagnostics[0]
            .details
            .as_ref()
            .and_then(|details| details.get("requiredFeature")),
        Some(&json!("http-client"))
    );
}
