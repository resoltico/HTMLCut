use super::*;

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
            input_base_url: None,
            effective_base_url: None,
            document_title: Some("Example".to_owned()),
        },
        selected_matches(SelectedMatch {
            candidate_index: NonZeroUsize::new(2).expect("candidate index"),
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
                candidate_index: NonZeroUsize::new(2).expect("candidate index"),
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
fn prepare_plan_returns_typed_plan_invalid_error() {
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

    let error = prepare_plan(&plan).expect_err("invalid plan");
    assert_eq!(error.error_code, ErrorCode::PlanInvalid);
    assert_eq!(error.strategy_kind, Some(StrategyKind::DelimiterPair));
    assert_eq!(error.plan_digest_sha256.len(), 64);
    assert_eq!(error.error_digest_sha256.len(), 64);
    assert!(
        error
            .details
            .get("contract_error")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|message| message.contains("delimiter_pair flags"))
    );
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

    let prepared = prepare_plan(&plan).expect("prepared plan");
    assert_eq!(prepared.plan(), &plan);
    assert_eq!(
        prepared.plan_digest_sha256(),
        plan.digest_sha256().expect("plan digest")
    );

    let result = execute_validated_plan(&source, &prepared).expect("validated execution");
    assert_eq!(result.plan_digest_sha256, prepared.plan_digest_sha256());
    assert_eq!(
        only_selected_match(&result).output_value,
        json!("<a href=\"https://example.com/docs/guide.html\">Guide</a>")
    );
}
