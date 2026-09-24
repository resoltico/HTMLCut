use super::*;
use std::num::NonZeroU32;

#[test]
fn interop_result_round_trips_through_stable_json() {
    let result = InteropResult::new(
        ResultExecution::new(
            TEST_PLAN_DIGEST_SHA256,
            StrategyKind::DelimiterPair,
            SelectionMode::Nth,
            Output::text(),
            3,
        ),
        ResultSource {
            prepared_source_digest_sha256:
                "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_owned(),
            input_base_url: None,
            effective_base_url: None,
            document_title: Some("Example".to_owned()),
        },
        selected_matches(SelectedMatch {
            candidate_index: NonZeroU32::new(2).expect("candidate index"),
            output_value: json!("Hello"),
            text_output: "Hello".to_owned(),
            comparison_text_output: None,
            plain_text_output: None,
            comparison_plain_text_output: None,
            selected_html_output: Some("Hello".to_owned()),
            inner_html_output: "Hello".to_owned(),
            outer_html_output: "<article>Hello</article>".to_owned(),
            metadata: SelectedMatchMetadata::DelimiterPair {
                candidate_count: 3,
                candidate_index: NonZeroU32::new(2).expect("candidate index"),
                selected_range: ByteRange { start: 10, end: 15 },
                inner_range: ByteRange { start: 11, end: 14 },
                outer_range: ByteRange { start: 9, end: 16 },
                include_start: true,
                include_end: false,
                matched_start: "<article>".to_owned(),
                matched_end: "</article>".to_owned(),
            },
        }),
        Vec::new(),
    )
    .with_computed_digest()
    .expect("digest");

    let stable = result.stable_json().expect("stable json");
    let round_trip: InteropResult = serde_json::from_str(&stable).expect("round trip result");

    assert_eq!(round_trip, result);
}

#[test]
fn compile_plan_returns_typed_plan_invalid_error() {
    let plan = Plan::new(
        PlanStrategy::delimiter_pair(
            delimiter_boundary("<article>"),
            delimiter_boundary("</article>"),
            DelimiterMode::Literal,
            DelimiterBoundaryRetention::ExcludeBoth,
            vec![RegexFlag::CaseInsensitive],
        ),
        Selection::single(),
        Output::text(),
        Rendering::new(TextWhitespace::Normalize, false),
    );

    let error = compile_plan(&plan).expect_err("invalid plan");
    assert_eq!(error.error_code, ErrorCode::PlanInvalid);
    assert_eq!(error.strategy_kind, Some(StrategyKind::DelimiterPair));
    assert_eq!(error.plan_digest_sha256.len(), 64);
    assert_eq!(error.error_digest_sha256.len(), 64);
    assert_eq!(error.detail, InteropErrorDetail::ContractViolation);
}

#[test]
fn compilation_rejects_invalid_css_grammar_before_source_preparation() {
    let plan = Plan::new(
        PlanStrategy::css_selector(css_selector("[")),
        Selection::single(),
        Output::text(),
        Rendering::new(TextWhitespace::Normalize, false),
    );

    assert!(compile_plan(&plan).is_err());
}

#[test]
fn compilation_rejects_invalid_regex_grammar_before_source_preparation() {
    let plan = Plan::new(
        PlanStrategy::delimiter_pair(
            delimiter_boundary("("),
            delimiter_boundary("</article>"),
            DelimiterMode::Regex,
            DelimiterBoundaryRetention::ExcludeBoth,
            Vec::new(),
        ),
        Selection::single(),
        Output::text(),
        Rendering::new(TextWhitespace::Normalize, false),
    );

    assert!(compile_plan(&plan).is_err());
}

#[test]
fn document_preparation_binds_exact_input_and_refuses_oversize_input_before_parse() {
    let source = HtmlInput::new("prepared", "<article>One</article>").expect("source");
    let prepared = prepare_document(source, PreparationLimits::default()).expect("prepared");
    assert_eq!(prepared.input_base_url(), None);
    assert_eq!(prepared.effective_base_url(), None);
    assert_eq!(prepared.input_digest_sha256().len(), 64);

    let limits = PreparationLimits::new(
        NonZeroU32::new(5).expect("non-zero"),
        NonZeroU32::new(1).expect("non-zero"),
        NonZeroU32::new(1).expect("non-zero"),
    )
    .expect("limits");
    let error = prepare_document(
        HtmlInput::new("prepared", "<article>One</article>").expect("source"),
        limits,
    )
    .expect_err("input must be rejected before parse");
    assert_eq!(error.error_code, PreparationErrorCode::InputTooLarge);
    assert_eq!(error.schema_name, PREPARATION_ERROR_SCHEMA_NAME);
}

#[test]
fn document_preparation_rejects_invalid_limits_and_parsed_complexity() {
    assert!(
        PreparationLimits::new(
            NonZeroU32::new(50 * 1024 * 1024 + 1).expect("non-zero"),
            NonZeroU32::new(1).expect("non-zero"),
            NonZeroU32::new(1).expect("non-zero"),
        )
        .is_err()
    );
    assert!(
        PreparationLimits::new(
            NonZeroU32::new(1).expect("non-zero"),
            NonZeroU32::new(1_000_001).expect("non-zero"),
            NonZeroU32::new(1).expect("non-zero"),
        )
        .is_err()
    );
    assert!(
        PreparationLimits::new(
            NonZeroU32::new(1).expect("non-zero"),
            NonZeroU32::new(1).expect("non-zero"),
            NonZeroU32::new(4_097).expect("non-zero"),
        )
        .is_err()
    );

    let invalid_limits = PreparationLimits {
        max_input_bytes: NonZeroU32::new(50 * 1024 * 1024 + 1).expect("non-zero"),
        max_elements: NonZeroU32::new(1).expect("non-zero"),
        max_dom_depth: NonZeroU32::new(1).expect("non-zero"),
    };
    assert_eq!(
        prepare_document(
            HtmlInput::new("invalid-limits", "<main>One</main>").expect("source"),
            invalid_limits,
        )
        .expect_err("invalid limits")
        .error_code,
        PreparationErrorCode::InternalInvariantViolation
    );

    let element_limited = PreparationLimits::new(
        NonZeroU32::new(1_024).expect("non-zero"),
        NonZeroU32::new(1).expect("non-zero"),
        NonZeroU32::new(32).expect("non-zero"),
    )
    .expect("limits");
    assert_eq!(
        prepare_document(
            HtmlInput::new("element-limit", "<main><p>One</p></main>").expect("source"),
            element_limited,
        )
        .expect_err("element limit")
        .error_code,
        PreparationErrorCode::DocumentTooComplex
    );

    let depth_limited = PreparationLimits::new(
        NonZeroU32::new(1_024).expect("non-zero"),
        NonZeroU32::new(32).expect("non-zero"),
        NonZeroU32::new(1).expect("non-zero"),
    )
    .expect("limits");
    assert_eq!(
        prepare_document(
            HtmlInput::new("depth-limit", "<main><p>One</p></main>").expect("source"),
            depth_limited,
        )
        .expect_err("depth limit")
        .error_code,
        PreparationErrorCode::DocumentTooDeep
    );

    let prepared = prepare_document(
        HtmlInput::new("debug-document", "<main>One</main>").expect("source"),
        PreparationLimits::default(),
    )
    .expect("prepared");
    assert!(format!("{prepared:?}").contains("PreparedDocument"));
}

#[test]
fn prepared_plan_executes_without_revalidating_the_plan_surface() {
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

    let prepared = compile_plan(&plan).expect("prepared plan");
    assert_eq!(prepared.plan(), &plan);
    assert_eq!(
        prepared.plan_digest_sha256(),
        plan.digest_sha256().expect("plan digest")
    );
    assert!(format!("{prepared:?}").contains("CompiledPlan"));

    let document =
        prepare_document(source, PreparationLimits::default()).expect("prepared document");
    let result = execute(&document, &prepared).expect("validated execution");
    assert_eq!(result.plan_digest_sha256, prepared.plan_digest_sha256());
    assert_eq!(
        result.source.prepared_source_digest_sha256,
        document.input_digest_sha256(),
        "every execution result must prove the prepared snapshot it used"
    );
    assert_eq!(
        only_selected_match(&result).output_value,
        json!("<a href=\"https://example.com/docs/guide.html\">Guide</a>")
    );
}

#[test]
fn one_prepared_document_executes_multiple_compiled_plans() {
    reset_preparation_parse_count_for_tests();
    let document = prepare_document(
        HtmlInput::new(
            "multi-plan",
            "<html><body><h1>Title</h1><article data-price=\"10\">Story</article></body></html>",
        )
        .expect("source"),
        PreparationLimits::default(),
    )
    .expect("prepared document");
    let heading = compile_plan(&Plan::new(
        PlanStrategy::css_selector(css_selector("h1")),
        Selection::single(),
        Output::plain_text(),
        Rendering::new(TextWhitespace::Normalize, false),
    ))
    .expect("heading plan");
    let attribute = compile_plan(&Plan::new(
        PlanStrategy::css_selector(css_selector("article")),
        Selection::single(),
        Output::attribute(output_attribute_name("data-price")),
        Rendering::new(TextWhitespace::Normalize, false),
    ))
    .expect("attribute plan");

    for iteration in 0..50 {
        let compiled = if iteration % 2 == 0 {
            &heading
        } else {
            &attribute
        };
        let expected = if iteration % 2 == 0 {
            json!("Title")
        } else {
            json!("10")
        };
        assert_eq!(
            only_selected_match(&execute(&document, compiled).expect("prepared execution"))
                .output_value,
            expected
        );
    }
    assert_eq!(
        preparation_parse_count_for_tests(),
        1,
        "fifty mixed executions must reuse the one prepared DOM"
    );
}

#[test]
fn one_compiled_plan_executes_multiple_prepared_documents_without_recompilation() {
    reset_preparation_parse_count_for_tests();
    let plan = compile_plan(&Plan::new(
        PlanStrategy::css_selector(css_selector("article")),
        Selection::single(),
        Output::plain_text(),
        Rendering::new(TextWhitespace::Normalize, false),
    ))
    .expect("compiled plan");
    let first = prepare_document(
        HtmlInput::new("first", "<article>First</article>").expect("source"),
        PreparationLimits::default(),
    )
    .expect("first prepared document");
    let second = prepare_document(
        HtmlInput::new("second", "<article>Second</article>").expect("source"),
        PreparationLimits::default(),
    )
    .expect("second prepared document");

    assert_eq!(
        only_selected_match(&execute(&first, &plan).expect("first execution")).output_value,
        json!("First")
    );
    assert_eq!(
        only_selected_match(&execute(&second, &plan).expect("second execution")).output_value,
        json!("Second")
    );
    assert_eq!(
        preparation_parse_count_for_tests(),
        2,
        "one compiled plan must execute both prepared snapshots without reparsing either"
    );
}
