use super::*;
use crate::extract::SliceCandidate;
use crate::result::Range;

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
    request.output.include_html = false;
    request.output.include_text = false;
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
    assert!(outer_html_match.html.is_none());
    assert!(outer_html_match.text.is_none());
}

#[test]
fn slice_match_builder_covers_inner_html_without_text_projection() {
    let mut request = ExtractionRequest::new(
        memory_source_with_base(
            "inline",
            "<div><a href=\"/x\"> Hello </a></div>",
            "https://example.com/base/",
        ),
        ExtractionSpec::slice(slice_spec("<a", "</a>").with_boundary_inclusion(true, true))
            .with_value(ValueSpec::InnerHtml),
    );
    request.normalization = NormalizationOptions {
        whitespace: WhitespaceMode::Normalize,
        rewrite_urls: true,
    };
    request.output.include_html = false;
    request.output.include_text = false;
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

    let inner_html_match = build_slice_match(
        &request,
        effective_base_url.as_deref(),
        &candidate,
        1,
        1,
        1,
        1,
    )
    .expect("inner html");
    assert_eq!(
        inner_html_match.value.as_str(),
        Some("<a href=\"https://example.com/x\"> Hello </a>")
    );
    assert!(inner_html_match.html.is_none());
    assert!(inner_html_match.text.is_none());
}
#[test]
fn slice_candidate_extraction_and_regex_builder_cover_error_paths() {
    let slice = slice_spec("<p>", "</p>");
    let no_end = extract_slice_candidates("<p>Hello", &slice).expect_err("no end");
    assert_eq!(no_end.code, "NO_MATCH");

    let no_start = extract_slice_candidates("Hello", &slice).expect_err("no start");
    assert_eq!(no_start.code, "NO_MATCH");

    let empty_pattern = build_finder("", PatternMode::Literal, None).expect_err("empty pattern");
    assert_eq!(empty_pattern.code, "INVALID_SLICE_PATTERN");

    let regex = build_regex("a.*b", "imsUx").expect("regex");
    assert!(regex.is_match("A\nB"));

    let invalid_regex = build_regex("[", "i").expect_err("invalid regex");
    assert_eq!(invalid_regex.code, "INVALID_SLICE_PATTERN");

    let unsupported_flag = build_regex("a", "u").expect_err("unsupported flag");
    assert_eq!(unsupported_flag.code, "INVALID_SLICE_PATTERN");

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
fn slice_finders_cover_literal_regex_and_empty_reader_edges() {
    let literal = build_finder("<p>", PatternMode::Literal, None).expect("literal finder");
    assert_eq!(
        literal.find("<p>Hello</p>", 0).expect("literal hit").start,
        0
    );
    assert!(literal.find("<p>Hello</p>", 10).is_none());

    let regex = build_finder(r"h\w+", PatternMode::Regex, Some("i")).expect("regex finder");
    assert_eq!(regex.find("Hello", 0).expect("regex hit").start, 0);
    assert!(regex.find("Hello", 5).is_none());

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
fn slice_runtime_reports_misrouted_requests_and_missing_boundaries() {
    let selector_request = ExtractionRequest::new(
        memory_source("inline", "<article>Hello</article>"),
        ExtractionSpec::selector(selector_query("article")),
    );
    let loaded = load_source(&selector_request.source, &RuntimeOptions::default()).expect("loaded");
    let invalid_run = run_slice_extraction(&selector_request, &loaded);
    assert_eq!(invalid_run.candidate_count, 0);
    assert!(invalid_run.matches.is_empty());
    assert_eq!(invalid_run.diagnostics[0].code, "INVALID_SLICE_PATTERN");

    let candidate = SliceCandidate {
        inner_html: "Hello".to_owned(),
        outer_html: "<article>Hello</article>".to_owned(),
        selected_html: "Hello".to_owned(),
        selected_range: Range { start: 9, end: 14 },
        inner_range: Range { start: 9, end: 14 },
        outer_range: Range { start: 0, end: 24 },
        matched_start: "<article>".to_owned(),
        matched_end: "</article>".to_owned(),
    };
    let invalid_match =
        build_slice_match(&selector_request, None, &candidate, 1, 1, 1, 1).expect_err("invalid");
    assert_eq!(invalid_match.code, "INVALID_SLICE_PATTERN");
}

#[test]
fn slice_runtime_reports_unresolved_base_when_rewrite_is_requested() {
    let mut request = ExtractionRequest::new(
        memory_source("inline", "<a href=\"guide.html\">Guide</a>"),
        ExtractionSpec::slice(slice_spec("<a", "</a>").with_boundary_inclusion(true, true))
            .with_value(ValueSpec::OuterHtml),
    );
    request.normalization = NormalizationOptions {
        whitespace: WhitespaceMode::Normalize,
        rewrite_urls: true,
    };
    request.output.include_html = false;
    request.output.include_text = false;

    let loaded = load_source(&request.source, &RuntimeOptions::default()).expect("loaded");
    let run = run_slice_extraction(&request, &loaded);

    assert_eq!(run.candidate_count, 1);
    assert_eq!(run.matches.len(), 1);
    assert_eq!(run.diagnostics[0].code, "EFFECTIVE_BASE_URL_UNRESOLVED");
    assert_eq!(
        run.matches[0].value.as_str(),
        Some("<a href=\"guide.html\">Guide</a>")
    );
}

#[test]
fn slice_runtime_reports_invalid_regex_patterns_during_runtime_wrapper_compilation() {
    let request = ExtractionRequest::new(
        memory_source("inline", "<article>Hello</article>"),
        ExtractionSpec::slice(SliceSpec {
            pattern: SlicePatternSpec::Regex {
                from: slice_boundary("("),
                to: slice_boundary("</article>"),
                flags: String::new(),
            },
            include_start: false,
            include_end: false,
        }),
    );
    let loaded = load_source(&request.source, &RuntimeOptions::default()).expect("loaded");
    let run = run_slice_extraction(&request, &loaded);

    assert_eq!(run.candidate_count, 0);
    assert!(run.matches.is_empty());
    assert_eq!(run.diagnostics.len(), 1);
    assert_eq!(run.diagnostics[0].code, "INVALID_SLICE_PATTERN");
}

#[test]
fn slice_runtime_skips_unresolved_base_warning_when_base_resolves() {
    let mut request = ExtractionRequest::new(
        memory_source_with_base(
            "inline",
            "<a href=\"guide.html\">Guide</a>",
            "https://example.com/docs/",
        ),
        ExtractionSpec::slice(slice_spec("<a", "</a>").with_boundary_inclusion(true, true))
            .with_value(ValueSpec::OuterHtml),
    );
    request.normalization = NormalizationOptions {
        whitespace: WhitespaceMode::Normalize,
        rewrite_urls: true,
    };
    request.output.include_html = false;
    request.output.include_text = false;

    let loaded = load_source(&request.source, &RuntimeOptions::default()).expect("loaded");
    let run = run_slice_extraction(&request, &loaded);

    assert_eq!(run.candidate_count, 1);
    assert_eq!(run.matches.len(), 1);
    assert!(run.diagnostics.is_empty());
    assert_eq!(
        run.matches[0].value.as_str(),
        Some("<a href=\"https://example.com/docs/guide.html\">Guide</a>")
    );
}
