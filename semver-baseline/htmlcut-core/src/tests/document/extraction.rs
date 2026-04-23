use super::*;

#[test]
fn selector_and_slice_runs_collect_builder_errors() {
    let selector_request = selector_request("<article data-id=\"7\">Hello</article>");
    let selector_loaded =
        load_source(&selector_request.source, &RuntimeOptions::default()).expect("loaded");
    let mut invalid_selector_request = selector_request.clone();
    invalid_selector_request.extraction = ExtractionSpec::selector(selector_query("["));
    let selector_run = run_selector_extraction(&invalid_selector_request, &selector_loaded);
    assert!(selector_run.matches.is_empty());
    assert_eq!(selector_run.diagnostics[0].code, "INVALID_SELECTOR");

    let slice_request = ExtractionRequest::new(
        memory_source_with_base(
            "inline",
            "<a href=\"/x\">Hello</a>",
            "https://example.com/base/",
        ),
        ExtractionSpec::slice(slice_spec("<a", "</a>").with_boundary_inclusion(true, true))
            .with_value(attribute_value("title")),
    );
    let selector_loaded =
        load_source(&slice_request.source, &RuntimeOptions::default()).expect("loaded");
    let slice_run = run_slice_extraction(&slice_request, &selector_loaded);
    assert!(slice_run.matches.is_empty());
    assert_eq!(slice_run.diagnostics[0].code, "MISSING_ATTRIBUTE");

    let selector_missing_attribute_request = ExtractionRequest::new(
        memory_source("inline", "<article data-id=\"7\">Hello</article>"),
        ExtractionSpec::selector(selector_query("article")).with_value(attribute_value("title")),
    );
    let selector_missing_attribute_loaded = load_source(
        &selector_missing_attribute_request.source,
        &RuntimeOptions::default(),
    )
    .expect("loaded");
    let selector_missing_attribute_run = run_selector_extraction(
        &selector_missing_attribute_request,
        &selector_missing_attribute_loaded,
    );
    assert!(selector_missing_attribute_run.matches.is_empty());
    assert_eq!(
        selector_missing_attribute_run.diagnostics[0].code,
        "MISSING_ATTRIBUTE"
    );
}
