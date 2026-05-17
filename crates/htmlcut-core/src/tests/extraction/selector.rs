use super::*;

#[test]
fn selector_match_builder_covers_value_modes_and_output_toggles() {
    let mut request = selector_request("<article data-id=\"7\"><p>Hello</p></article>");
    let document = parse_document_node("<article data-id=\"7\"><p>Hello</p></article>");
    let node = select_first(&document, "article").expect("selector");

    request.extraction = request.extraction.clone().with_value(ValueSpec::InnerHtml);
    let html_match = build_selector_match(&request, &node, &node, 1, 1, 1, 1).expect("html match");
    assert_eq!(html_match.value.as_str(), Some("<p>Hello</p>"));

    request.extraction = request.extraction.clone().with_value(ValueSpec::OuterHtml);
    let outer_match =
        build_selector_match(&request, &node, &node, 1, 1, 1, 1).expect("outer match");
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
    request.output.rendering.whitespace = WhitespaceMode::Normalize;
    let attribute_match =
        build_selector_match(&request, &node, &node, 1, 1, 1, 1).expect("attribute match");
    assert_eq!(attribute_match.value.as_str(), Some("7"));

    request.extraction = request
        .extraction
        .clone()
        .with_value(attribute_value("href"));
    let missing_attribute =
        build_selector_match(&request, &node, &node, 1, 1, 1, 1).expect_err("missing attr");
    assert_eq!(missing_attribute.code, "MISSING_ATTRIBUTE");

    request.extraction = request.extraction.clone().with_value(ValueSpec::Structured);
    request.output.include_html = false;
    request.output.include_text = false;
    let structured_match =
        build_selector_match(&request, &node, &node, 1, 1, 1, 2).expect("structured");
    assert!(structured_match.html.is_none());
    assert!(structured_match.text.is_none());
    assert_eq!(structured_match.value["tagName"], "article");
    assert_eq!(structured_match.value["matchIndex"], 1);
    assert_eq!(structured_match.value["matchCount"], 1);
    assert_eq!(structured_match.value["candidateIndex"], 1);
    assert_eq!(structured_match.value["candidateCount"], 2);
    assert_eq!(structured_match.value["textOutput"], "Hello");
    assert_eq!(structured_match.value["innerHtmlOutput"], "<p>Hello</p>");
    assert!(
        structured_match.value["outerHtmlOutput"]
            .as_str()
            .is_some_and(|html| html.contains("article"))
    );
    assert_eq!(structured_match.metadata.candidate_index(), 1);
    assert_eq!(structured_match.metadata.candidate_count(), 2);

    request.extraction = request
        .extraction
        .clone()
        .with_value(ValueSpec::SelectedHtml);
    let selected_html_error =
        build_selector_match(&request, &node, &node, 1, 1, 1, 1).expect_err("selected html error");
    assert_eq!(selected_html_error.code, "UNSUPPORTED_VALUE_TYPE");
}
#[test]
fn selector_match_builder_emits_optional_payloads_when_requested() {
    let request = selector_request("<article><p>Hello</p></article>");
    let document = parse_document_node("<article><p>Hello</p></article>");
    let node = select_first(&document, "article").expect("selector");

    let matched = build_selector_match(&request, &node, &node, 1, 1, 1, 1).expect("match");

    assert!(
        matched
            .html
            .as_deref()
            .is_some_and(|html| html.contains("<article>"))
    );
    assert_eq!(matched.text.as_deref(), Some("Hello"));
}

#[test]
fn selector_match_builder_renders_selected_element_semantics() {
    let image_request = selector_request("<img src=\"hero.png\" alt=\"Hero\">");
    let image_document = parse_document_node("<img src=\"hero.png\" alt=\"Hero\">");
    let image = select_first(&image_document, "img").expect("img");
    let image_match =
        build_selector_match(&image_request, &image, &image, 1, 1, 1, 1).expect("image");
    assert_eq!(image_match.value.as_str(), Some("Hero"));
    assert_eq!(image_match.text.as_deref(), Some("Hero"));

    let pre_request = selector_request("<pre>line 1\n  line 2</pre>");
    let pre_document = parse_document_node("<pre>line 1\n  line 2</pre>");
    let pre = select_first(&pre_document, "pre").expect("pre");
    let pre_match = build_selector_match(&pre_request, &pre, &pre, 1, 1, 1, 1).expect("pre");
    assert_eq!(pre_match.value.as_str(), Some("line 1\n  line 2"));
    assert_eq!(pre_match.text.as_deref(), Some("line 1\n  line 2"));
}

#[test]
fn selector_match_builder_preserves_selected_roots_that_only_look_like_utility_chrome() {
    let request =
        selector_request("<p class=\"status pricing report\">All Systems Operational</p>");
    let document =
        parse_document_node("<p class=\"status pricing report\">All Systems Operational</p>");
    let node = select_first(&document, "p").expect("status");

    let matched = build_selector_match(&request, &node, &node, 1, 1, 1, 1).expect("match");

    assert_eq!(matched.value.as_str(), Some("All Systems Operational"));
    assert_eq!(matched.text.as_deref(), Some("All Systems Operational"));

    let nav_request = selector_request("<nav><a href=\"/docs\">Docs</a></nav>");
    let nav_document = parse_document_node("<nav><a href=\"/docs\">Docs</a></nav>");
    let nav = select_first(&nav_document, "nav").expect("nav");
    let nav_match = build_selector_match(&nav_request, &nav, &nav, 1, 1, 1, 1).expect("nav");

    assert_eq!(nav_match.value.as_str(), Some("Docs [/docs]"));
    assert_eq!(nav_match.text.as_deref(), Some("Docs [/docs]"));
}

#[test]
fn selector_match_builder_uses_resolved_links_for_text_projection() {
    let request = ExtractionRequest::new(
        SourceRequest::memory(
            "inline",
            "<article><p><a href=\"guide.html\">Guide</a></p></article>",
        )
        .with_base_url(HttpUrl::parse("https://example.com/docs/page.html").expect("url")),
        ExtractionSpec::selector(SelectorQuery::new("article").expect("selector")),
    );
    let document =
        parse_document_node("<article><p><a href=\"guide.html\">Guide</a></p></article>");
    let original = select_first(&document, "article").expect("article");
    let mut rewritten_document = document.clone();
    rewrite_urls_in_document(
        &mut rewritten_document,
        "https://example.com/docs/page.html",
    );
    let text_projection = select_first(&rewritten_document, "article").expect("article");

    let matched =
        build_selector_match(&request, &original, &text_projection, 1, 1, 1, 1).expect("match");

    assert_eq!(
        matched.value.as_str(),
        Some("Guide [https://example.com/docs/guide.html]")
    );
    assert!(
        matched
            .html
            .as_deref()
            .is_some_and(|html| html.contains("href=\"guide.html\""))
    );
}

#[test]
fn selector_runtime_reports_misrouted_and_per_match_builder_errors() {
    let invalid_request = ExtractionRequest::new(
        memory_source("inline", "<article>Hello</article>"),
        ExtractionSpec::slice(slice_spec("<article>", "</article>")),
    );
    let loaded = load_source(&invalid_request.source, &RuntimeOptions::default()).expect("loaded");
    let invalid_run = run_selector_extraction(&invalid_request, &loaded);
    assert_eq!(invalid_run.candidate_count, 0);
    assert!(invalid_run.matches.is_empty());
    assert_eq!(invalid_run.diagnostics[0].code, "INVALID_SELECTOR");

    let mut missing_attribute_request = selector_request("<article>Hello</article>");
    missing_attribute_request.extraction = missing_attribute_request
        .extraction
        .clone()
        .with_value(attribute_value("href"));
    let loaded = load_source(
        &missing_attribute_request.source,
        &RuntimeOptions::default(),
    )
    .expect("loaded");
    let missing_attribute_run = run_selector_extraction(&missing_attribute_request, &loaded);
    assert_eq!(missing_attribute_run.candidate_count, 1);
    assert!(missing_attribute_run.matches.is_empty());
    assert_eq!(
        missing_attribute_run.diagnostics[0].code,
        "MISSING_ATTRIBUTE"
    );

    let mut selected_html_request = selector_request("<article>Hello</article>");
    selected_html_request.extraction = selected_html_request
        .extraction
        .clone()
        .with_value(ValueSpec::SelectedHtml);
    let loaded =
        load_source(&selected_html_request.source, &RuntimeOptions::default()).expect("loaded");
    let selected_html_run = run_selector_extraction(&selected_html_request, &loaded);
    assert_eq!(selected_html_run.candidate_count, 1);
    assert!(selected_html_run.matches.is_empty());
    assert_eq!(
        selected_html_run.diagnostics[0].code,
        "UNSUPPORTED_VALUE_TYPE"
    );
}

#[test]
fn selector_validation_preserves_parser_details() {
    let invalid_request = ExtractionRequest::new(
        memory_source("inline", "<article>Hello</article>"),
        ExtractionSpec::selector(SelectorQuery::new("[").expect("selector query")),
    );
    let loaded = load_source(&invalid_request.source, &RuntimeOptions::default()).expect("loaded");
    let invalid_run = run_selector_extraction(&invalid_request, &loaded);

    assert_eq!(invalid_run.candidate_count, 0);
    assert!(invalid_run.matches.is_empty());
    assert_eq!(invalid_run.diagnostics[0].code, "INVALID_SELECTOR");
    assert!(
        invalid_run.diagnostics[0]
            .message
            .contains("Invalid selector: [")
    );
    assert!(
        invalid_run.diagnostics[0]
            .details
            .as_ref()
            .and_then(|details| details.get("parseError"))
            .and_then(Value::as_str)
            .is_some()
    );
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
    #[cfg(feature = "http-client")]
    let url_metadata = empty_source_metadata(&url_source("https://example.com/docs/page.html"));
    #[cfg(feature = "http-client")]
    assert_eq!(url_metadata.value, "https://example.com/docs/page.html");
    #[cfg(feature = "http-client")]
    assert_eq!(
        url_metadata.input_base_url.as_deref(),
        Some("https://example.com/docs/page.html")
    );
    assert_eq!(SourceRequest::stdin().kind(), SourceKind::Stdin);
    #[cfg(feature = "http-client")]
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
