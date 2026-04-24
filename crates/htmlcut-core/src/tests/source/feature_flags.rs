use super::*;

#[test]
fn url_loading_without_http_client_feature_fails_cleanly() {
    let request = ExtractionRequest::new(
        url_source("https://example.com/articles"),
        selector_request("<article>Hello</article>").extraction,
    );

    let result = extract(&request, &RuntimeOptions::default());

    assert!(!result.ok);
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(result.diagnostics[0].code, "SOURCE_LOAD_FAILED");
    assert!(
        result.diagnostics[0]
            .message
            .contains("compiled without the `http-client` feature")
    );
    assert_eq!(
        result.diagnostics[0]
            .details
            .as_ref()
            .and_then(|details| details.get("requiredFeature"))
            .and_then(Value::as_str),
        Some("http-client")
    );
    assert_eq!(result.source.kind, SourceKind::Url);
    assert_eq!(result.source.value, "https://example.com/articles");
}
