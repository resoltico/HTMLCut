use super::*;

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

    let mut selected_html_request = selector_request("<article>Hello</article>");
    selected_html_request.extraction = selected_html_request
        .extraction
        .clone()
        .with_value(ValueSpec::SelectedHtml);
    let diagnostics = validate_request(&selected_html_request)
        .expect_err("selected html on selector should fail");
    assert!(
        diagnostics
            .iter()
            .any(|item| item.code == "UNSUPPORTED_VALUE_TYPE")
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
        boundary_retention: BoundaryRetention::IncludeBoth,
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
