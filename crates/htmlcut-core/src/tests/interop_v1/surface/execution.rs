use super::*;

#[test]
fn execute_plan_executes_css_selector_with_rewritten_outer_html() {
    let source = HtmlInput::new(
        "target-story",
        "<html><head><title>Example</title></head><body><article><a href=\"guide.html\">Guide</a></article></body></html>",
    )
    .expect("source")
    .with_input_base_url(http_url("https://example.com/docs/start.html"));
    let plan = Plan::new(
        PlanStrategy::css_selector(css_selector("article a")),
        Selection::single(),
        Output::outer_html(),
        Rendering::new(TextWhitespace::Normalize, true),
    );

    let result = execute_plan(&source, &plan).expect("interop result");

    assert_eq!(result.schema_name, RESULT_SCHEMA_NAME);
    assert_eq!(result.plan_digest_sha256.len(), 64);
    assert_eq!(result.result_digest_sha256.len(), 64);
    assert_eq!(
        result.source.input_base_url,
        Some(displayed_http_url("https://example.com/docs/start.html"))
    );
    assert_eq!(
        result.source.effective_base_url,
        Some(displayed_http_url("https://example.com/docs/start.html"))
    );
    assert_eq!(result.source.document_title.as_deref(), Some("Example"));
    assert_eq!(result.candidate_count, 1);
    let selected_match = only_selected_match(&result);
    assert_eq!(result.output.kind(), OutputKind::OuterHtml);
    assert_eq!(
        selected_match.output_value,
        json!("<a href=\"https://example.com/docs/guide.html\">Guide</a>")
    );
    assert_eq!(
        selected_match.text_output,
        "Guide [https://example.com/docs/guide.html]"
    );
    assert_eq!(selected_match.inner_html_output, "Guide");
    assert_eq!(
        selected_match.outer_html_output,
        "<a href=\"https://example.com/docs/guide.html\">Guide</a>"
    );
    match &selected_match.metadata {
        SelectedMatchMetadata::CssSelector {
            candidate_count,
            candidate_index,
            path,
            tag_name,
            attributes,
        } => {
            assert_eq!(*candidate_count, 1);
            assert_eq!(candidate_index.get(), 1);
            assert!(path.contains("article:nth-of-type(1)"));
            assert_eq!(tag_name, "a");
            assert_eq!(
                attributes.get("href"),
                Some(&"https://example.com/docs/guide.html".to_owned())
            );
        }
        other => panic!("expected selector metadata, got {other:?}"),
    }
}

#[test]
fn execute_plan_executes_regex_delimiter_pair() {
    let source =
        HtmlInput::new("target-article", "<ARTICLE data-id=\"7\">Hello</ARTICLE>").expect("source");
    let plan = Plan::new(
        PlanStrategy::delimiter_pair(
            delimiter_boundary(r"<article[^>]*>"),
            delimiter_boundary(r"</article>"),
            DelimiterMode::Regex,
            DelimiterBoundaryRetention::IncludeBoth,
            vec![RegexFlag::CaseInsensitive],
        ),
        Selection::single(),
        Output::selected_html(),
        Rendering::new(TextWhitespace::Normalize, false),
    );

    let result = execute_plan(&source, &plan).expect("interop result");

    let selected_match = only_selected_match(&result);
    assert_eq!(result.output.kind(), OutputKind::SelectedHtml);
    assert_eq!(
        selected_match.output_value,
        json!("<ARTICLE data-id=\"7\">Hello</ARTICLE>")
    );
    assert_eq!(selected_match.text_output, "Hello");
    assert_eq!(
        selected_match.selected_html_output.as_deref(),
        Some("<ARTICLE data-id=\"7\">Hello</ARTICLE>")
    );
    assert_eq!(selected_match.inner_html_output, "Hello");
    assert_eq!(
        selected_match.outer_html_output,
        "<ARTICLE data-id=\"7\">Hello</ARTICLE>"
    );
    match &selected_match.metadata {
        SelectedMatchMetadata::DelimiterPair {
            candidate_count,
            candidate_index,
            selected_range,
            inner_range,
            outer_range,
            include_start,
            include_end,
            matched_start,
            matched_end,
        } => {
            assert_eq!(*candidate_count, 1);
            assert_eq!(candidate_index.get(), 1);
            assert_eq!(*selected_range, ByteRange { start: 0, end: 36 });
            assert_eq!(*inner_range, ByteRange { start: 21, end: 26 });
            assert_eq!(*outer_range, ByteRange { start: 0, end: 36 });
            assert!(*include_start);
            assert!(*include_end);
            assert_eq!(matched_start, "<ARTICLE data-id=\"7\">");
            assert_eq!(matched_end, "</ARTICLE>");
        }
        other => panic!("expected delimiter metadata, got {other:?}"),
    }
}

#[test]
fn execute_plan_executes_all_selection_in_one_result_document() {
    let source = HtmlInput::new(
        "target-news",
        "<article>One</article><article>Two</article>",
    )
    .expect("source");
    let plan = Plan::new(
        PlanStrategy::css_selector(css_selector("article")),
        Selection::all(),
        Output::text(),
        Rendering::new(TextWhitespace::Normalize, false),
    );

    let result = execute_plan(&source, &plan).expect("interop result");

    assert_eq!(result.selection_mode, SelectionMode::All);
    assert_eq!(result.candidate_count, 2);
    assert_eq!(result.selected_matches.len(), 2);
    assert_eq!(result.selected_matches[0].output_value, json!("One"));
    assert_eq!(result.selected_matches[1].output_value, json!("Two"));
}

#[test]
fn execute_plan_enforces_the_default_html_size_limit_for_preloaded_input() {
    let oversized = format!("<article>{}</article>", "x".repeat(DEFAULT_MAX_BYTES + 1));
    let source = HtmlInput::new("target-oversized", oversized).expect("source");
    let plan = Plan::new(
        PlanStrategy::css_selector(css_selector("article")),
        Selection::single(),
        Output::text(),
        Rendering::new(TextWhitespace::Normalize, false),
    );

    let error = execute_plan(&source, &plan).expect_err("oversized source should fail");
    assert_eq!(error.error_code, ErrorCode::InternalError);
    assert_eq!(
        error.diagnostics[0].code,
        InteropDiagnosticCode::SourceLoadFailed
    );
    assert!(error.message.contains("exceeds"));
}

#[test]
fn execute_plan_maps_ambiguous_single_to_ambiguous_match_error() {
    let source = HtmlInput::new(
        "target-news",
        "<article>One</article><article>Two</article>",
    )
    .expect("source");
    let plan = Plan::new(
        PlanStrategy::css_selector(css_selector("article")),
        Selection::single(),
        Output::text(),
        Rendering::new(TextWhitespace::Normalize, false),
    );

    let error = execute_plan(&source, &plan).expect_err("ambiguous match");

    assert_eq!(error.error_code, ErrorCode::AmbiguousMatch);
    assert_eq!(error.strategy_kind, Some(StrategyKind::CssSelector));
    assert_eq!(error.plan_digest_sha256.len(), 64);
    assert_eq!(error.error_digest_sha256.len(), 64);
    assert_eq!(error.diagnostics[0].code, "AMBIGUOUS_MATCH");
    assert!(error.message.contains("exactly one candidate"));
}

#[test]
fn execute_plan_retains_no_match_candidate_count_in_the_interop_error_details() {
    let source = HtmlInput::new("target-news", "<article>One</article>").expect("source");
    let plan = Plan::new(
        PlanStrategy::css_selector(css_selector(".missing")),
        Selection::single(),
        Output::structured(),
        Rendering::new(TextWhitespace::Normalize, false),
    );

    let error = execute_plan(&source, &plan).expect_err("no matching candidate");

    assert_eq!(error.error_code, ErrorCode::NoMatch);
    assert_eq!(error.diagnostics[0].code, InteropDiagnosticCode::NoMatch);
    assert_eq!(error.details["core_diagnostic_code"], "NO_MATCH");
    assert_eq!(error.details["core_details"]["candidateCount"], 0);
}

#[test]
fn execute_plan_publishes_safe_selector_parse_details_with_utf16_locations() {
    let source = HtmlInput::new("target-news", "<article>One</article>").expect("source");
    let plan = Plan::new(
        PlanStrategy::css_selector(css_selector("article,\n😀span[[[bad")),
        Selection::single(),
        Output::text(),
        Rendering::new(TextWhitespace::Normalize, false),
    );

    let error = execute_plan(&source, &plan).expect_err("invalid selector");

    assert_eq!(error.error_code, ErrorCode::PlanInvalid);
    assert_eq!(error.message, "CSS selector is invalid.");
    assert_eq!(error.diagnostics.len(), 1);
    assert_eq!(error.diagnostics[0].message, "CSS selector is invalid.");
    assert_eq!(
        error.diagnostics[0].details,
        Some(json!({
            "selector_parse": {
                "line": 2,
                "column_utf16": 8,
                "parse_error_class": "invalid_attribute_selector",
            }
        }))
    );
    assert_eq!(
        error.details["core_details"]["selector_parse"],
        error.diagnostics[0]
            .details
            .as_ref()
            .expect("selector parse details")["selector_parse"]
    );
    assert_eq!(error.details["core_details"]["candidateCount"], 0);
    assert!(error.validate().is_ok());
}
