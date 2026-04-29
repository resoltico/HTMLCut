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
    selector_request.normalization.rewrite_urls = true;
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
        ExtractionSpec::slice(slice_spec("<a ", "</a>").with_boundary_inclusion(true, true))
            .with_value(attribute_value("href")),
    );
    slice_request.normalization.rewrite_urls = true;
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

#[test]
fn inspect_source_summarizes_document_structure() {
    let source = memory_source_with_base(
        "fixture.html",
        "<!DOCTYPE html><html><head><title>Fixture</title><base href=\"../content/\"></head><body><main><article class=\"story card\"><h1>Hello</h1><p>World</p><a href=\"../guide.html\">Guide</a><img src=\"hero.png\" alt=\"Hero\"><table><tr><td>A</td></tr></table></article><section class=\"card\"><h2>More</h2><a href=\"/docs\">Docs</a></section></main></body></html>",
        "https://example.test/docs/start.html",
    );
    let inspection = inspect_source(
        &source,
        &RuntimeOptions::default(),
        &InspectionOptions {
            include_source_text: true,
            sample_limit: 4,
        },
    );

    assert!(inspection.ok);
    assert_eq!(
        inspection.source.text.as_deref(),
        Some(
            "<!DOCTYPE html><html><head><title>Fixture</title><base href=\"../content/\"></head><body><main><article class=\"story card\"><h1>Hello</h1><p>World</p><a href=\"../guide.html\">Guide</a><img src=\"hero.png\" alt=\"Hero\"><table><tr><td>A</td></tr></table></article><section class=\"card\"><h2>More</h2><a href=\"/docs\">Docs</a></section></main></body></html>"
        )
    );
    assert_eq!(
        inspection.source.input_base_url.as_deref(),
        Some("https://example.test/docs/start.html")
    );
    assert_eq!(
        inspection.source.effective_base_url.as_deref(),
        Some("https://example.test/content/")
    );
    let document = inspection.document.expect("document inspection");
    assert_eq!(document.title.as_deref(), Some("Fixture"));
    assert_eq!(document.document_base_href.as_deref(), Some("../content/"));
    assert_eq!(document.root_tag, "html");
    assert!(document.element_count >= 10);
    assert_eq!(document.link_count, 2);
    assert_eq!(document.image_count, 1);
    assert_eq!(document.table_count, 1);
    assert_eq!(document.top_tags[0].name, "a");
    assert_eq!(document.top_tags[0].count, 2);
    assert_eq!(document.top_classes[0].name, "card");
    assert_eq!(document.top_classes[0].count, 2);
    assert_eq!(document.headings[0].level, 1);
    assert_eq!(document.headings[0].text, "Hello");
    assert_eq!(document.links[0].href.as_deref(), Some("../guide.html"));
    assert_eq!(
        document.links[0].resolved_href.as_deref(),
        Some("https://example.test/guide.html")
    );
    assert!(document.text_char_count > 0);
}

#[test]
fn inspect_source_honors_zero_sample_limit_without_collecting_previews() {
    let source = memory_source_with_base(
        "fixture.html",
        "<html><body><h1>Hello</h1><a href=\"/guide\">Guide</a><a>No href</a></body></html>",
        "https://example.test/start.html",
    );

    let inspection = inspect_source(
        &source,
        &RuntimeOptions::default(),
        &InspectionOptions {
            include_source_text: false,
            sample_limit: 0,
        },
    );

    assert!(inspection.ok);
    let document = inspection.document.expect("document inspection");
    assert_eq!(document.link_count, 2);
    assert!(document.headings.is_empty());
    assert!(document.links.is_empty());
    assert!(document.top_tags.is_empty());
    assert!(document.top_classes.is_empty());
}

#[test]
fn validate_request_reports_unsupported_versions_and_invalid_selectors() {
    let mut request = selector_request("");
    request.spec_version = 99;
    request.extraction = ExtractionSpec::selector(selector_query("["));

    let diagnostics = validate_request(&request).expect_err("invalid request");
    assert!(has_errors(&diagnostics));
    assert!(
        diagnostics
            .iter()
            .any(|item| item.code == "UNSUPPORTED_SPEC_VERSION")
    );
    assert!(
        diagnostics
            .iter()
            .any(|item| item.code == "INVALID_SELECTOR")
    );
}

#[test]
fn validate_request_accepts_current_requests() {
    let selector = selector_request("<article>Hello</article>");
    assert!(validate_request(&selector).is_ok());

    let mut slice = slice_request(
        "<section data-id=\"7\">Hello</section>",
        "<section",
        "</section>",
    );
    slice.extraction = ExtractionSpec::slice(SliceSpec {
        pattern: SlicePatternSpec::literal(
            slice_boundary("<section"),
            slice_boundary("</section>"),
        ),
        include_start: true,
        include_end: true,
    })
    .with_selection(nth_selection(1))
    .with_value(attribute_value("data-id"));
    slice.output.preview_chars = NonZeroUsize::new(32).expect("preview chars");

    assert!(validate_request(&slice).is_ok());
}

#[test]
fn extract_rejects_invalid_requests_before_loading_the_source() {
    let missing_file_selector = ExtractionRequest::new(
        file_source("/definitely/missing.html"),
        ExtractionSpec::selector(selector_query("[")),
    );
    let selector_result = extract(&missing_file_selector, &RuntimeOptions::default());
    assert!(!selector_result.ok);
    assert_eq!(selector_result.source.bytes_read, 0);
    assert_eq!(selector_result.diagnostics[0].code, "INVALID_SELECTOR");

    let missing_file_slice = ExtractionRequest::new(
        file_source("/definitely/missing.html"),
        ExtractionSpec::slice(regex_slice_spec("[", "</article>")),
    );
    let slice_result = extract(&missing_file_slice, &RuntimeOptions::default());
    assert!(!slice_result.ok);
    assert_eq!(slice_result.source.bytes_read, 0);
    assert_eq!(slice_result.diagnostics[0].code, "INVALID_SLICE_PATTERN");
}
