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
fn slice_match_builder_covers_value_modes() {
    let mut request = ExtractionRequest::new(
        memory_source_with_base(
            "inline",
            "<a href=\"/x\">Hello</a>",
            "https://example.com/base/",
        ),
        ExtractionSpec::slice(slice_spec("<a", "</a>").with_boundary_inclusion(true, true))
            .with_value(ValueSpec::InnerHtml),
    );
    request.normalization = NormalizationOptions {
        whitespace: WhitespaceMode::Normalize,
        rewrite_urls: true,
    };
    let loaded = load_source(&request.source, &RuntimeOptions::default()).expect("loaded");
    let effective_base_url = resolve_document_base_url(
        &parse_document_node(&loaded.text),
        loaded.input_base_url.as_deref(),
    );
    let candidate = extract_slice_candidates(
        &loaded.text,
        request.extraction.slice_spec().expect("slice spec"),
    )
    .expect("candidate")
    .remove(0);

    let html_match = build_slice_match(
        &request,
        effective_base_url.as_deref(),
        &candidate,
        1,
        1,
        1,
        1,
    )
    .expect("html");
    assert!(
        html_match
            .value
            .as_str()
            .is_some_and(|html| html.contains("https://example.com/x"))
    );

    let mut attribute_request = request.clone();
    attribute_request.extraction = attribute_request
        .extraction
        .clone()
        .with_value(attribute_value("href"));
    let attribute_match = build_slice_match(
        &attribute_request,
        effective_base_url.as_deref(),
        &candidate,
        1,
        1,
        1,
        1,
    )
    .expect("attribute");
    assert_eq!(
        attribute_match.value.as_str(),
        Some("https://example.com/x")
    );

    attribute_request.extraction = attribute_request
        .extraction
        .clone()
        .with_value(attribute_value("title"));
    let missing = build_slice_match(
        &attribute_request,
        effective_base_url.as_deref(),
        &candidate,
        1,
        1,
        1,
        1,
    )
    .expect_err("missing attr");
    assert_eq!(missing.code, "MISSING_ATTRIBUTE");
    assert!(
        missing
            .message
            .contains("Extracted fragment is missing attribute \"title\".")
    );

    let mut inner_capture_request = request.clone();
    inner_capture_request.extraction = ExtractionSpec::slice(SliceSpec {
        pattern: SlicePatternSpec::literal(slice_boundary("<a "), slice_boundary("</a>")),
        include_start: false,
        include_end: false,
    })
    .with_value(attribute_value("href"));
    let inner_candidate = extract_slice_candidates(
        &loaded.text,
        inner_capture_request
            .extraction
            .slice_spec()
            .expect("slice spec"),
    )
    .expect("candidate")
    .remove(0);
    let hinted_missing = build_slice_match(
        &inner_capture_request,
        effective_base_url.as_deref(),
        &inner_candidate,
        1,
        1,
        1,
        1,
    )
    .expect_err("inner capture should drop opening-tag attributes");
    assert_eq!(hinted_missing.code, "MISSING_ATTRIBUTE");
    assert!(hinted_missing.message.contains("use --include-start"));
    assert_eq!(
        hinted_missing
            .details
            .as_ref()
            .and_then(|details| details.get("hint"))
            .and_then(Value::as_str),
        Some("use --include-start")
    );

    let mut structured_request = request.clone();
    structured_request.extraction = structured_request
        .extraction
        .clone()
        .with_value(ValueSpec::Structured);
    let structured = build_slice_match(
        &structured_request,
        effective_base_url.as_deref(),
        &candidate,
        1,
        1,
        1,
        1,
    )
    .expect("structured");
    assert_eq!(structured.value["matchIndex"], 1);
    assert_eq!(structured.value["matchCount"], 1);
    assert_eq!(structured.value["candidateIndex"], 1);
    assert_eq!(structured.value["candidateCount"], 1);
    assert_eq!(
        structured.value["outerHtml"],
        "<a href=\"https://example.com/x\">Hello</a>"
    );
    assert_eq!(structured.value["includeStart"], true);
    assert_eq!(structured.value["includeEnd"], true);
    assert_eq!(structured.value["matchedStart"], "<a");
    assert_eq!(structured.value["matchedEnd"], "</a>");
    match &structured.metadata {
        ExtractionMatchMetadata::DelimiterPair(metadata) => {
            assert_eq!(metadata.matched_start, "<a");
            assert_eq!(metadata.matched_end, "</a>");
        }
        other => panic!("expected delimiter metadata, got {other:?}"),
    }
}

#[test]
fn slice_match_builder_covers_text_and_outer_html_modes() {
    let mut request = ExtractionRequest::new(
        memory_source_with_base(
            "inline",
            "<div><a href=\"/x\"> Hello </a></div>",
            "https://example.com/base/",
        ),
        ExtractionSpec::slice(slice_spec("<a", "</a>").with_boundary_inclusion(true, true))
            .with_value(ValueSpec::Text),
    );
    request.normalization = NormalizationOptions {
        whitespace: WhitespaceMode::Normalize,
        rewrite_urls: true,
    };
    let loaded = load_source(&request.source, &RuntimeOptions::default()).expect("loaded");
    let effective_base_url = resolve_document_base_url(
        &parse_document_node(&loaded.text),
        loaded.input_base_url.as_deref(),
    );
    let candidate = extract_slice_candidates(
        &loaded.text,
        request.extraction.slice_spec().expect("slice spec"),
    )
    .expect("candidate")
    .remove(0);

    let text_match = build_slice_match(
        &request,
        effective_base_url.as_deref(),
        &candidate,
        1,
        1,
        1,
        1,
    )
    .expect("text");
    assert_eq!(text_match.value.as_str(), Some("Hello"));

    request.extraction = request.extraction.clone().with_value(ValueSpec::OuterHtml);
    let outer_html_match = build_slice_match(
        &request,
        effective_base_url.as_deref(),
        &candidate,
        1,
        1,
        1,
        1,
    )
    .expect("outer html");
    assert!(
        outer_html_match
            .value
            .as_str()
            .is_some_and(|html| html.contains("https://example.com/x"))
    );
}

#[test]
fn slice_candidate_extraction_and_regex_builder_cover_error_paths() {
    let slice = slice_spec("<p>", "</p>");
    let no_end = extract_slice_candidates("<p>Hello", &slice).expect_err("no end");
    assert_eq!(no_end.code, "NO_MATCH");

    let no_start = extract_slice_candidates("Hello", &slice).expect_err("no start");
    assert_eq!(no_start.code, "NO_MATCH");

    let empty_pattern = build_finder("", PatternMode::Literal, None)
        .err()
        .expect("empty pattern");
    assert_eq!(empty_pattern.code, "INVALID_SLICE_PATTERN");

    let regex = build_regex("a.*b", "imsUx").expect("regex");
    assert!(regex.is_match("A\nB"));

    let invalid_regex = build_regex("[", "u").expect_err("invalid regex");
    assert_eq!(invalid_regex.code, "INVALID_SLICE_PATTERN");

    let zero_width = extract_slice_candidates(
        "abc",
        &regex_slice_spec(r"\b", r"\b").with_boundary_inclusion(true, true),
    )
    .expect("zero width candidates");
    assert_eq!(zero_width.len(), 2);
    assert_eq!(zero_width[0].selected_range.start, 0);
    assert_eq!(zero_width[1].selected_range.start, 3);
}

#[test]
fn extraction_specs_cover_optional_selector_and_slice_views() {
    assert!(SelectionSpec::single().index().is_none());
    assert!(SelectionSpec::First.index().is_none());
    assert!(SelectionSpec::All.index().is_none());
    assert_eq!(
        SelectionSpec::nth(NonZeroUsize::new(2).expect("index")).index(),
        Some(NonZeroUsize::new(2).expect("index"))
    );

    let selector = ExtractionSpec::selector(selector_query("article"));
    assert_eq!(
        selector
            .selector_query()
            .expect("selector query should exist")
            .as_ref(),
        "article"
    );
    assert!(selector.slice_spec().is_none());

    let slice = ExtractionSpec::slice(slice_spec("<article>", "</article>"));
    assert!(slice.selector_query().is_none());
    assert!(slice.slice_spec().is_some());

    assert!(ValueSpec::Text.attribute_name().is_none());
    assert!(ValueSpec::InnerHtml.attribute_name().is_none());
    assert!(ValueSpec::OuterHtml.attribute_name().is_none());
    assert!(ValueSpec::Structured.attribute_name().is_none());
}

#[test]
fn slice_finders_cover_literal_regex_and_empty_reader_edges() {
    let literal = build_finder("<p>", PatternMode::Literal, None).expect("literal finder");
    assert_eq!(literal("<p>Hello</p>", 0).expect("literal hit").start, 0);
    assert!(literal("<p>Hello</p>", 10).is_none());

    let regex = build_finder(r"h\w+", PatternMode::Regex, Some("iu")).expect("regex finder");
    assert_eq!(regex("Hello", 0).expect("regex hit").start, 0);
    assert!(regex("Hello", 5).is_none());

    let mut empty = Cursor::new(Vec::<u8>::new());
    assert_eq!(
        read_limited_to_string(&mut empty, 10, "Input").expect("empty input"),
        ""
    );
    assert!(!position_inside_markup_for_tests("plain text only", 0));
    assert!(!position_inside_markup_for_tests("plain text only", 5));
    assert!(!position_inside_markup_for_tests("plain text only", 99));
}

#[test]
fn extraction_runs_cover_selector_and_slice_candidate_selection_branches() {
    let mut selector_no_match_request = selector_request("<article>Hello</article>");
    selector_no_match_request.extraction = ExtractionSpec::selector(selector_query("aside"));
    let selector_no_match = extract(&selector_no_match_request, &RuntimeOptions::default());
    assert!(!selector_no_match.ok);
    assert_eq!(selector_no_match.diagnostics[0].code, "NO_MATCH");

    let selector_multiple = extract(
        &selector_request("<article>One</article><article>Two</article>"),
        &RuntimeOptions::default(),
    );
    assert!(selector_multiple.ok);
    assert!(
        selector_multiple
            .diagnostics
            .iter()
            .any(|item| item.code == "MULTIPLE_MATCHES")
    );

    let slice_no_match = extract(
        &slice_request("<div>Hello</div>", "<section>", "</section>"),
        &RuntimeOptions::default(),
    );
    assert!(!slice_no_match.ok);
    assert_eq!(slice_no_match.diagnostics[0].code, "NO_MATCH");

    let slice_multiple = extract(
        &slice_request(
            "<article>One</article><article>Two</article>",
            "<article>",
            "</article>",
        ),
        &RuntimeOptions::default(),
    );
    assert!(slice_multiple.ok);
    assert!(
        slice_multiple
            .diagnostics
            .iter()
            .any(|item| item.code == "MULTIPLE_MATCHES")
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
