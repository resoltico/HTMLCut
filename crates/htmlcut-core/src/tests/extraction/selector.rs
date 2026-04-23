use super::*;

#[test]
fn selector_match_builder_covers_value_modes_and_output_toggles() {
    let mut request = selector_request("<article data-id=\"7\"><p>Hello</p></article>");
    let document = parse_document_node("<article data-id=\"7\"><p>Hello</p></article>");
    let node = select_first(&document, "article").expect("selector");
    let loaded = load_source(&request.source, &RuntimeOptions::default()).expect("loaded");
    let effective_base_url = resolve_document_base_url(&document, loaded.input_base_url.as_deref());

    request.extraction = request.extraction.clone().with_value(ValueSpec::InnerHtml);
    let html_match =
        build_selector_match(&request, effective_base_url.as_deref(), &node, 1, 1, 1, 1)
            .expect("html match");
    assert_eq!(html_match.value.as_str(), Some("<p>Hello</p>"));

    request.extraction = request.extraction.clone().with_value(ValueSpec::OuterHtml);
    let outer_match =
        build_selector_match(&request, effective_base_url.as_deref(), &node, 1, 1, 1, 1)
            .expect("outer match");
    assert!(
        outer_match
            .value
            .as_str()
            .is_some_and(|html| html.contains("article"))
    );

    request.extraction = request
        .extraction
        .clone()
        .with_value(attribute_value("data-id"));
    request.normalization.whitespace = WhitespaceMode::Normalize;
    let attribute_match =
        build_selector_match(&request, effective_base_url.as_deref(), &node, 1, 1, 1, 1)
            .expect("attribute match");
    assert_eq!(attribute_match.value.as_str(), Some("7"));

    request.extraction = request
        .extraction
        .clone()
        .with_value(attribute_value("href"));
    let missing_attribute =
        build_selector_match(&request, effective_base_url.as_deref(), &node, 1, 1, 1, 1)
            .expect_err("missing attr");
    assert_eq!(missing_attribute.code, "MISSING_ATTRIBUTE");

    request.extraction = request.extraction.clone().with_value(ValueSpec::Structured);
    request.output.include_html = false;
    request.output.include_text = false;
    let structured_match =
        build_selector_match(&request, effective_base_url.as_deref(), &node, 1, 1, 1, 2)
            .expect("structured");
    assert!(structured_match.html.is_none());
    assert!(structured_match.text.is_none());
    assert_eq!(structured_match.value["tagName"], "article");
    assert_eq!(structured_match.value["matchIndex"], 1);
    assert_eq!(structured_match.value["matchCount"], 1);
    assert_eq!(structured_match.value["candidateIndex"], 1);
    assert_eq!(structured_match.value["candidateCount"], 2);
    assert_eq!(structured_match.metadata.candidate_index(), 1);
    assert_eq!(structured_match.metadata.candidate_count(), 2);
}
#[test]
fn selector_match_builder_emits_optional_payloads_when_requested() {
    let request = selector_request("<article><p>Hello</p></article>");
    let document = parse_document_node("<article><p>Hello</p></article>");
    let node = select_first(&document, "article").expect("selector");
    let loaded = load_source(&request.source, &RuntimeOptions::default()).expect("loaded");
    let effective_base_url = resolve_document_base_url(&document, loaded.input_base_url.as_deref());

    let matched = build_selector_match(&request, effective_base_url.as_deref(), &node, 1, 1, 1, 1)
        .expect("match");

    assert!(
        matched
            .html
            .as_deref()
            .is_some_and(|html| html.contains("<article>"))
    );
    assert_eq!(matched.text.as_deref(), Some("Hello"));
}
#[test]
fn select_candidates_and_source_helpers_cover_remaining_branches() {
    let (selected, diagnostics) = select_candidates::<i32>(&[], &SelectionSpec::default());
    assert!(selected.is_empty());
    assert_eq!(diagnostics[0].code, "NO_MATCH");

    let (selected, diagnostics) = select_candidates(&[1, 2], &SelectionSpec::All);
    assert_eq!(
        selected
            .into_iter()
            .map(|item| item.candidate)
            .collect::<Vec<_>>(),
        vec![1, 2]
    );
    assert!(diagnostics.is_empty());

    let (selected, diagnostics) = select_candidates(
        &[1, 2],
        &SelectionSpec::nth(NonZeroUsize::new(2).expect("match index")),
    );
    assert_eq!(
        selected
            .into_iter()
            .map(|item| item.candidate)
            .collect::<Vec<_>>(),
        vec![2]
    );
    assert!(diagnostics.is_empty());

    let (selected, diagnostics) = select_candidates(
        &[1],
        &SelectionSpec::nth(NonZeroUsize::new(1).expect("match index")),
    );
    assert_eq!(
        selected
            .into_iter()
            .map(|item| item.candidate)
            .collect::<Vec<_>>(),
        vec![1]
    );
    assert!(diagnostics.is_empty());

    let source = memory_source("", "Hello");
    let loaded = load_source(&source, &RuntimeOptions::default()).expect("memory load");
    assert_eq!(loaded.value, "memory");
    let url_metadata = empty_source_metadata(&url_source("https://example.com/docs/page.html"));
    assert_eq!(url_metadata.value, "https://example.com/docs/page.html");
    assert_eq!(
        url_metadata.input_base_url.as_deref(),
        Some("https://example.com/docs/page.html")
    );
    assert_eq!(SourceRequest::stdin().kind(), SourceKind::Stdin);
    assert_eq!(url_source("https://example.com").kind(), SourceKind::Url);
    assert_eq!(file_source("page.html").kind(), SourceKind::File);
}
#[test]
fn select_candidates_covers_first_and_invalid_nth_cases() {
    let (selected, diagnostics) = select_candidates(&[1, 2], &SelectionSpec::default());
    assert_eq!(
        selected
            .into_iter()
            .map(|item| item.candidate)
            .collect::<Vec<_>>(),
        vec![1]
    );
    assert_eq!(diagnostics[0].code, "MULTIPLE_MATCHES");

    let (selected, diagnostics) = select_candidates(&[1], &SelectionSpec::First);
    assert_eq!(
        selected
            .into_iter()
            .map(|item| item.candidate)
            .collect::<Vec<_>>(),
        vec![1]
    );
    assert!(diagnostics.is_empty());

    let (selected, diagnostics) = select_candidates(&[1, 2], &SelectionSpec::single());
    assert!(selected.is_empty());
    assert_eq!(diagnostics[0].code, "AMBIGUOUS_MATCH");

    let (selected, diagnostics) = select_candidates(
        &[1, 2],
        &SelectionSpec::nth(NonZeroUsize::new(3).expect("match index")),
    );
    assert!(selected.is_empty());
    assert_eq!(diagnostics[0].code, "MATCH_INDEX_OUT_OF_RANGE");
}
