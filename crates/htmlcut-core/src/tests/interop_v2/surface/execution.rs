use super::*;

#[test]
fn execute_compiled_plan_executes_css_selector_with_rewritten_outer_html() {
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

    let result = execute_one_shot_for_tests(&source, &plan).expect("interop result");

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
fn execute_compiled_plan_exposes_plain_dom_text_without_heading_decoration() {
    let source = HtmlInput::new(
        "target-heading",
        "<html><body><h1>Big <em>Title</em></h1></body></html>",
    )
    .expect("source");
    let plan = Plan::new(
        PlanStrategy::css_selector(css_selector("h1")),
        Selection::single(),
        Output::plain_text(),
        Rendering::new(TextWhitespace::Normalize, false),
    );

    let result = execute_one_shot_for_tests(&source, &plan).expect("plain-text interop result");
    let selected_match = only_selected_match(&result);

    assert_eq!(result.output.kind(), OutputKind::PlainText);
    assert_eq!(selected_match.output_value, json!("Big Title"));
    assert_eq!(
        selected_match.plain_text_output.as_deref(),
        Some("Big Title")
    );
    assert_eq!(selected_match.text_output, "# Big Title");
}

#[test]
fn execute_compiled_plan_executes_regex_delimiter_pair() {
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

    let result = execute_one_shot_for_tests(&source, &plan).expect("interop result");

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
fn execute_compiled_plan_executes_all_selection_in_one_result_document() {
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

    let result = execute_one_shot_for_tests(&source, &plan).expect("interop result");

    assert_eq!(result.selection_mode, SelectionMode::All);
    assert_eq!(result.candidate_count, 2);
    assert_eq!(result.selected_matches.len(), 2);
    assert_eq!(result.selected_matches[0].output_value, json!("One"));
    assert_eq!(result.selected_matches[1].output_value, json!("Two"));
}

#[test]
fn document_preparation_refuses_oversize_input_before_execution() {
    let source =
        HtmlInput::new("target-oversized", "<article>oversized</article>").expect("source");
    let limits = PreparationLimits::new(
        NonZeroU32::new(5).expect("non-zero"),
        PreparationLimits::default().max_elements,
        PreparationLimits::default().max_dom_depth,
    )
    .expect("limits");
    let error = prepare_document(source, limits).expect_err("oversized source should fail");
    assert_eq!(error.error_code, PreparationErrorCode::InputTooLarge);
    assert!(error.message.contains("exceeds"));
}

#[test]
fn prepared_execution_reports_selector_work_exhaustion_without_no_match() {
    let source = HtmlInput::new(
        "target-budget",
        "<main><article>One</article><article>Two</article></main>",
    )
    .expect("source");
    let budget = ExecutionBudget {
        max_selector_work_units: NonZeroU32::new(1).expect("non-zero"),
        ..ExecutionBudget::default()
    };
    let plan = Plan::new(
        PlanStrategy::css_selector(css_selector("article")),
        Selection::all(),
        Output::text(),
        Rendering::new(TextWhitespace::Normalize, false),
    )
    .with_execution_budget(budget);
    let compiled = compile_plan(&plan).expect("compiled plan");
    let document = prepare_document(source, PreparationLimits::default()).expect("prepared");

    let error = execute(&document, &compiled).expect_err("selector work must be bounded");
    assert_eq!(error.error_code, ErrorCode::ExecutionLimitExceeded);
    assert_eq!(
        error.diagnostics[0].code,
        InteropDiagnosticCode::SelectorWorkLimitExceeded
    );
    assert_ne!(error.error_code, ErrorCode::NoMatch);
}

#[test]
fn prepared_selector_execution_preserves_single_first_and_nth_retention_paths() {
    let document = prepare_document(
        HtmlInput::new(
            "selector-retention",
            "<main><article>One</article><article>Two</article><article>Three</article></main>",
        )
        .expect("source"),
        PreparationLimits::default(),
    )
    .expect("prepared");
    let plan_for = |selection| {
        Plan::new(
            PlanStrategy::css_selector(css_selector("article")),
            selection,
            Output::plain_text(),
            Rendering::new(TextWhitespace::Normalize, false),
        )
    };

    assert_eq!(
        execute(
            &document,
            &compile_plan(&plan_for(Selection::single())).expect("single plan"),
        )
        .expect_err("single must reject multiple candidates")
        .error_code,
        ErrorCode::AmbiguousMatch
    );
    assert_eq!(
        only_selected_match(
            &execute(
                &document,
                &compile_plan(&plan_for(Selection::first())).expect("first plan"),
            )
            .expect("first result"),
        )
        .output_value,
        json!("One")
    );
    assert_eq!(
        only_selected_match(
            &execute(
                &document,
                &compile_plan(&plan_for(Selection::nth(
                    NonZeroU32::new(2).expect("non-zero")
                )))
                .expect("nth plan"),
            )
            .expect("nth result"),
        )
        .output_value,
        json!("Two")
    );
}

#[test]
fn prepared_execution_refuses_candidate_and_selected_match_overflow() {
    let source = HtmlInput::new(
        "target-cardinality",
        "<main><article>One</article><article>Two</article></main>",
    )
    .expect("source");
    let document = prepare_document(source, PreparationLimits::default()).expect("prepared");

    let candidate_budget = ExecutionBudget {
        max_candidates: NonZeroU32::new(1).expect("non-zero"),
        ..ExecutionBudget::default()
    };
    let candidate_plan = Plan::new(
        PlanStrategy::css_selector(css_selector("article")),
        Selection::all(),
        Output::text(),
        Rendering::new(TextWhitespace::Normalize, false),
    )
    .with_execution_budget(candidate_budget);
    let candidate_error = execute(
        &document,
        &compile_plan(&candidate_plan).expect("compiled candidate plan"),
    )
    .expect_err("candidate limit must be enforced");
    assert_eq!(
        candidate_error.error_code,
        ErrorCode::ExecutionLimitExceeded
    );
    assert_eq!(
        candidate_error.diagnostics[0].code,
        InteropDiagnosticCode::CandidateLimitExceeded
    );

    let selected_budget = ExecutionBudget {
        max_selected_matches: NonZeroU32::new(1).expect("non-zero"),
        ..ExecutionBudget::default()
    };
    let selected_plan = Plan::new(
        PlanStrategy::css_selector(css_selector("article")),
        Selection::all(),
        Output::text(),
        Rendering::new(TextWhitespace::Normalize, false),
    )
    .with_execution_budget(selected_budget);
    let selected_error = execute(
        &document,
        &compile_plan(&selected_plan).expect("compiled selected plan"),
    )
    .expect_err("selected-match limit must be enforced");
    assert_eq!(selected_error.error_code, ErrorCode::ExecutionLimitExceeded);
    assert_eq!(
        selected_error.diagnostics[0].code,
        InteropDiagnosticCode::SelectedMatchLimitExceeded
    );
}

#[test]
fn prepared_execution_refuses_per_match_and_total_output_overflow() {
    let source = HtmlInput::new(
        "target-output",
        "<main><article>One</article><article>Two</article></main>",
    )
    .expect("source");
    let document = prepare_document(source, PreparationLimits::default()).expect("prepared");

    let per_match_budget = ExecutionBudget {
        max_match_output_bytes: NonZeroU32::new(1).expect("non-zero"),
        ..ExecutionBudget::default()
    };
    let per_match_plan = Plan::new(
        PlanStrategy::css_selector(css_selector("article:first-of-type")),
        Selection::single(),
        Output::text(),
        Rendering::new(TextWhitespace::Normalize, false),
    )
    .with_execution_budget(per_match_budget);
    let per_match_error = execute(
        &document,
        &compile_plan(&per_match_plan).expect("compiled per-match plan"),
    )
    .expect_err("per-match output limit must be enforced");
    assert_eq!(
        per_match_error.error_code,
        ErrorCode::ExecutionLimitExceeded
    );
    assert_eq!(
        per_match_error.diagnostics[0].code,
        InteropDiagnosticCode::OutputLimitExceeded
    );
    assert_eq!(
        per_match_error.diagnostics[0]
            .details
            .as_ref()
            .expect("details")["limitName"],
        "maxMatchOutputBytes"
    );

    let total_budget = ExecutionBudget {
        max_match_output_bytes: NonZeroU32::new(1_024).expect("non-zero"),
        max_total_output_bytes: NonZeroU32::new(10).expect("non-zero"),
        ..ExecutionBudget::default()
    };
    let total_plan = Plan::new(
        PlanStrategy::css_selector(css_selector("article")),
        Selection::all(),
        Output::text(),
        Rendering::new(TextWhitespace::Normalize, false),
    )
    .with_execution_budget(total_budget);
    let total_error = execute(
        &document,
        &compile_plan(&total_plan).expect("compiled total plan"),
    )
    .expect_err("total output limit must be enforced");
    assert_eq!(total_error.error_code, ErrorCode::ExecutionLimitExceeded);
    assert_eq!(
        total_error.diagnostics[0]
            .details
            .as_ref()
            .expect("details")["limitName"],
        "maxTotalOutputBytes"
    );
}

#[test]
fn prepared_delimiter_execution_honors_candidate_and_output_limits() {
    let source = HtmlInput::new(
        "target-delimiter-budget",
        "<article>One</article><article>Two</article>",
    )
    .expect("source");
    let document = prepare_document(source, PreparationLimits::default()).expect("prepared");
    let delimiter_strategy = || {
        PlanStrategy::delimiter_pair(
            delimiter_boundary("<article>"),
            delimiter_boundary("</article>"),
            DelimiterMode::Literal,
            DelimiterBoundaryRetention::IncludeBoth,
            Vec::new(),
        )
    };

    let candidate_budget = ExecutionBudget {
        max_candidates: NonZeroU32::new(1).expect("non-zero"),
        ..ExecutionBudget::default()
    };
    let candidate_plan = Plan::new(
        delimiter_strategy(),
        Selection::all(),
        Output::selected_html(),
        Rendering::new(TextWhitespace::Normalize, false),
    )
    .with_execution_budget(candidate_budget);
    let candidate_error = execute(
        &document,
        &compile_plan(&candidate_plan).expect("compiled candidate plan"),
    )
    .expect_err("delimiter candidate limit must be enforced");
    assert_eq!(
        candidate_error.error_code,
        ErrorCode::ExecutionLimitExceeded
    );
    assert_eq!(
        candidate_error.diagnostics[0].code,
        InteropDiagnosticCode::CandidateLimitExceeded
    );

    let output_budget = ExecutionBudget {
        max_match_output_bytes: NonZeroU32::new(1).expect("non-zero"),
        ..ExecutionBudget::default()
    };
    let output_plan = Plan::new(
        delimiter_strategy(),
        Selection::first(),
        Output::selected_html(),
        Rendering::new(TextWhitespace::Normalize, false),
    )
    .with_execution_budget(output_budget);
    let output_error = execute(
        &document,
        &compile_plan(&output_plan).expect("compiled output plan"),
    )
    .expect_err("delimiter output limit must be enforced");
    assert_eq!(output_error.error_code, ErrorCode::ExecutionLimitExceeded);
    assert_eq!(
        output_error.diagnostics[0].code,
        InteropDiagnosticCode::OutputLimitExceeded
    );
}

#[test]
fn execute_compiled_plan_maps_ambiguous_single_to_ambiguous_match_error() {
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

    let error = execute_one_shot_for_tests(&source, &plan).expect_err("ambiguous match");

    assert_eq!(error.error_code, ErrorCode::AmbiguousMatch);
    assert_eq!(error.strategy_kind, Some(StrategyKind::CssSelector));
    assert_eq!(error.plan_digest_sha256.len(), 64);
    assert_eq!(error.error_digest_sha256.len(), 64);
    assert_eq!(error.diagnostics[0].code, "AMBIGUOUS_MATCH");
    assert!(error.message.contains("exactly one candidate"));
}

#[test]
fn execute_compiled_plan_retains_no_match_candidate_count_in_closed_interop_error_evidence() {
    let source = HtmlInput::new("target-news", "<article>One</article>").expect("source");
    let plan = Plan::new(
        PlanStrategy::css_selector(css_selector(".missing")),
        Selection::single(),
        Output::structured(),
        Rendering::new(TextWhitespace::Normalize, false),
    );

    let error = execute_one_shot_for_tests(&source, &plan).expect_err("no matching candidate");

    assert_eq!(error.error_code, ErrorCode::NoMatch);
    assert_eq!(error.diagnostics[0].code, InteropDiagnosticCode::NoMatch);
    assert_eq!(
        error.detail,
        InteropErrorDetail::CoreExecution {
            core_diagnostic_code: InteropDiagnosticCode::NoMatch,
            candidate_count: 0,
        }
    );
}

#[test]
fn execute_compiled_plan_publishes_safe_selector_parse_details_with_utf16_locations() {
    let source = HtmlInput::new("target-news", "<article>One</article>").expect("source");
    let plan = Plan::new(
        PlanStrategy::css_selector(css_selector("article,\n😀span[[[bad")),
        Selection::single(),
        Output::text(),
        Rendering::new(TextWhitespace::Normalize, false),
    );

    let error = execute_one_shot_for_tests(&source, &plan).expect_err("invalid selector");

    assert_eq!(error.error_code, ErrorCode::PlanInvalid);
    assert_eq!(error.message, "CSS selector is invalid.");
    assert_eq!(error.diagnostics.len(), 1);
    assert_eq!(error.diagnostics[0].message, "CSS selector is invalid.");
    let selector_parse = &error.diagnostics[0]
        .details
        .as_ref()
        .expect("selector parse diagnostics")["selector_parse"];
    assert_eq!(selector_parse["line"], 2);
    assert_eq!(selector_parse["column_utf16"], 8);
    assert_eq!(
        selector_parse["parse_error_class"],
        "invalid_attribute_selector"
    );
    assert_eq!(
        error.detail,
        InteropErrorDetail::InvalidSelector {
            candidate_count: 0,
            selector_parse: crate::selector_parse::validate_selector_parse_details(
                error.diagnostics[0]
                    .details
                    .as_ref()
                    .expect("selector parse diagnostics"),
            )
            .expect("valid selector parse diagnostics"),
        }
    );
    assert!(error.validate().is_ok());
}

mod limits;
