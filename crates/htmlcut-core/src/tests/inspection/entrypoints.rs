use super::*;

#[test]
fn parse_document_and_preview_cover_public_entrypoints() {
    let request = selector_request("<article>Hello</article>");
    let parsed = parse_document(&request.source, &RuntimeOptions::default());
    assert!(parsed.ok);
    assert_eq!(parsed.operation_id, OperationId::DocumentParse);
    assert!(parsed.document.is_some());

    let inspection = inspect_source(
        &request.source,
        &RuntimeOptions::default(),
        &InspectionOptions::default(),
    );
    assert!(inspection.ok);
    assert_eq!(inspection.operation_id, OperationId::SourceInspect);
    assert!(inspection.document.is_some());

    let preview = preview_extraction(&request, &RuntimeOptions::default());
    assert!(preview.ok);
    assert_eq!(preview.operation_id, OperationId::SelectPreview);

    let missing = file_source("/definitely/missing.html");
    let parsed_error = parse_document(&missing, &RuntimeOptions::default());
    assert!(!parsed_error.ok);
    assert_eq!(parsed_error.operation_id, OperationId::DocumentParse);
    assert_eq!(parsed_error.diagnostics[0].code, "SOURCE_LOAD_FAILED");

    let inspection_error = inspect_source(
        &missing,
        &RuntimeOptions::default(),
        &InspectionOptions::default(),
    );
    assert!(!inspection_error.ok);
    assert_eq!(inspection_error.operation_id, OperationId::SourceInspect);
    assert_eq!(inspection_error.diagnostics[0].code, "SOURCE_LOAD_FAILED");

    let mut invalid = selector_request("<article>Hello</article>");
    invalid.spec_version = 0;
    let invalid_result = extract(&invalid, &RuntimeOptions::default());
    assert!(!invalid_result.ok);
    assert_eq!(invalid_result.operation_id, OperationId::SelectExtract);
    assert_eq!(invalid_result.stats.match_count, 0);
    assert_eq!(invalid_result.source.bytes_read, 0);
    assert_eq!(
        invalid_result.diagnostics[0].code,
        "UNSUPPORTED_SPEC_VERSION"
    );
}

#[test]
fn unresolved_effective_base_is_reported_for_inspection_and_rewrite_requests() {
    let source = memory_source(
        "inline",
        "<html><head><base href=\"../content/\"></head><body><a href=\"guide.html\">Guide</a></body></html>",
    );

    let inspection = inspect_source(
        &source,
        &RuntimeOptions::default(),
        &InspectionOptions::default(),
    );
    assert!(inspection.ok);
    assert_eq!(
        inspection.diagnostics[0].code,
        "EFFECTIVE_BASE_URL_UNRESOLVED"
    );
    assert!(inspection.source.effective_base_url.is_none());

    let mut selector_request = ExtractionRequest::new(
        source.clone(),
        ExtractionSpec::selector(selector_query("a")).with_value(attribute_value("href")),
    );
    selector_request.output.rendering.rewrite_urls = true;
    let selector_result = extract(&selector_request, &RuntimeOptions::default());
    assert!(selector_result.ok);
    assert_eq!(
        selector_result.matches[0].value,
        Value::String("guide.html".to_owned())
    );
    assert_eq!(
        selector_result.diagnostics[0].code,
        "EFFECTIVE_BASE_URL_UNRESOLVED"
    );

    let mut slice_request = ExtractionRequest::new(
        source,
        ExtractionSpec::slice(
            slice_spec("<a ", "</a>").with_boundary_retention(BoundaryRetention::IncludeBoth),
        )
        .with_value(attribute_value("href")),
    );
    slice_request.output.rendering.rewrite_urls = true;
    let slice_result = extract(&slice_request, &RuntimeOptions::default());
    assert!(slice_result.ok);
    assert_eq!(
        slice_result.matches[0].value,
        Value::String("guide.html".to_owned())
    );
    assert_eq!(
        slice_result.diagnostics[0].code,
        "EFFECTIVE_BASE_URL_UNRESOLVED"
    );
}
