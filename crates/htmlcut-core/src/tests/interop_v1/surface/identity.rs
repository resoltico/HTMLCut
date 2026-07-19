use super::*;

#[test]
fn stable_json_v1_sorts_object_keys_recursively() {
    let value = json!({
        "z": 1,
        "a": {
            "d": 4,
            "b": 2,
            "a": 1
        },
        "arr": [
            {
                "y": 2,
                "x": 1
            }
        ]
    });

    let stable = stable_json_v1(&value).expect("stable json");

    assert_eq!(
        stable,
        r#"{"a":{"a":1,"b":2,"d":4},"arr":[{"x":1,"y":2}],"z":1}"#
    );
}

#[test]
fn html_input_builds_memory_source_request() {
    let base_url = http_url("https://example.com/start.html");
    let source = HtmlInput::new("target-news", "<article>Hello</article>")
        .expect("source input")
        .with_input_base_url(base_url.clone());

    assert_eq!(source.label, "target-news");
    assert_eq!(source.html, "<article>Hello</article>");
    assert_eq!(source.input_base_url, Some(base_url));
}

#[test]
fn html_input_rejects_blank_labels() {
    let error = HtmlInput::new("   ", "<article>Hello</article>").expect_err("blank label");
    assert!(matches!(error, ContractError::EmptySourceLabel));
}

#[test]
fn html_input_extraction_identity_binds_complete_input_plan_and_semantics_version() {
    let source = HtmlInput::new("target-news", "<article>Hello</article>")
        .expect("source input")
        .with_input_base_url(http_url("https://example.com/start.html"));
    let plan = Plan::new(
        PlanStrategy::css_selector(css_selector("article")),
        Selection::single(),
        Output::text(),
        Rendering::new(TextWhitespace::Normalize, false),
    );

    let identity = source
        .extraction_identity_sha256(&plan)
        .expect("extraction identity");

    assert_eq!(HTMLCUT_EXTRACTION_SEMANTICS_VERSION, 4);
    assert_eq!(
        identity,
        "5289fb8f86b322f12c72e584dab7cbc48e2af00d7c98638e34047831d91fd797"
    );
    assert_eq!(
        source
            .extraction_identity_sha256(&plan)
            .expect("repeated extraction identity"),
        identity
    );

    let mut changed_label = source.clone();
    changed_label.label = "target-news-revision".to_owned();
    assert_ne!(
        changed_label
            .extraction_identity_sha256(&plan)
            .expect("changed label identity"),
        identity
    );

    let mut changed_html = source.clone();
    changed_html.html = "<article>Hello, world</article>".to_owned();
    assert_ne!(
        changed_html
            .extraction_identity_sha256(&plan)
            .expect("changed HTML identity"),
        identity
    );

    let mut changed_base_url = source.clone();
    changed_base_url.input_base_url = Some(http_url("https://example.com/elsewhere.html"));
    assert_ne!(
        changed_base_url
            .extraction_identity_sha256(&plan)
            .expect("changed base URL identity"),
        identity
    );

    let changed_plan = Plan::new(
        PlanStrategy::css_selector(css_selector("article")),
        Selection::first(),
        Output::text(),
        Rendering::new(TextWhitespace::Normalize, false),
    );
    assert_ne!(
        source
            .extraction_identity_sha256(&changed_plan)
            .expect("changed plan identity"),
        identity
    );
}

#[test]
fn html_input_extraction_identity_binds_invalid_plans_for_diagnostics() {
    let source = HtmlInput::new("target-news", "<article>Hello</article>")
        .expect("source input")
        .with_input_base_url(http_url("https://example.com/start.html"));
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

    let identity = source
        .extraction_identity_sha256(&plan)
        .expect("invalid-plan extraction identity");

    assert_eq!(
        identity,
        "dff8152418c9423294432bf279969ac6dfae9fdfdf60a2923bf808fc911a8bef"
    );
    assert_eq!(
        prepare_plan(&plan)
            .expect_err("invalid plan should produce a diagnostic")
            .error_code,
        ErrorCode::PlanInvalid
    );
    assert_eq!(
        source
            .extraction_identity_sha256(&plan)
            .expect("repeated invalid-plan extraction identity"),
        identity
    );

    let mut changed_source = source.clone();
    changed_source.html = "<article>Changed</article>".to_owned();
    assert_ne!(
        changed_source
            .extraction_identity_sha256(&plan)
            .expect("changed source invalid-plan extraction identity"),
        identity
    );

    let mut changed_plan = plan.clone();
    changed_plan.schema_version = 0;
    assert_ne!(
        source
            .extraction_identity_sha256(&changed_plan)
            .expect("changed invalid-plan extraction identity"),
        identity
    );
}

#[test]
fn plan_validates_literal_regex_flag_conflicts() {
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

    let error = plan.validate().expect_err("literal flags should fail");
    assert!(matches!(error, ContractError::LiteralDelimiterFlags));
}

#[test]
fn plan_uses_frozen_schema_identity() {
    let plan = Plan::new(
        PlanStrategy::css_selector(css_selector("article")),
        Selection::first(),
        Output::text(),
        Rendering::new(TextWhitespace::Normalize, true),
    );

    assert_eq!(plan.schema_name, PLAN_SCHEMA_NAME);
    assert_eq!(plan.digest_sha256().expect("plan digest").len(), 64);
}

#[test]
fn interop_result_digest_ignores_existing_digest_field() {
    let mut result_one = InteropResult::new(
        ResultExecution::new(
            TEST_PLAN_DIGEST_SHA256,
            StrategyKind::CssSelector,
            SelectionMode::Single,
            Output::outer_html(),
            1,
        ),
        ResultSource {
            input_base_url: Some(displayed_http_url("https://example.com/start.html")),
            effective_base_url: Some(displayed_http_url("https://example.com/base/")),
            document_title: Some("Example".to_owned()),
        },
        selected_matches(selector_match()),
        vec![InteropDiagnostic {
            level: InteropDiagnosticLevel::Warning,
            code: InteropDiagnosticCode::EffectiveBaseUrlUnresolved,
            message: "ignored for digest stability".to_owned(),
            details: None,
        }],
    );
    result_one.result_digest_sha256 = "first".to_owned();

    let mut result_two = result_one.clone();
    result_two.result_digest_sha256 = "second".to_owned();

    assert_eq!(
        result_one.digest_sha256().expect("result digest"),
        result_two.digest_sha256().expect("result digest")
    );
    assert_eq!(result_one.schema_name, RESULT_SCHEMA_NAME);
}

#[test]
fn interop_result_validation_rejects_error_diagnostics() {
    let result = InteropResult::new(
        ResultExecution::new(
            TEST_PLAN_DIGEST_SHA256,
            StrategyKind::CssSelector,
            SelectionMode::Single,
            Output::outer_html(),
            1,
        ),
        ResultSource {
            input_base_url: None,
            effective_base_url: None,
            document_title: None,
        },
        selected_matches(selector_match()),
        vec![InteropDiagnostic {
            level: InteropDiagnosticLevel::Error,
            code: InteropDiagnosticCode::NoMatch,
            message: "should not be present on success".to_owned(),
            details: None,
        }],
    );

    let error = result
        .validate()
        .expect_err("error diagnostics should fail");
    assert!(matches!(error, ContractError::ErrorDiagnosticsInSuccess));
}

#[test]
fn interop_error_digest_ignores_existing_digest_field() {
    let mut details = BTreeMap::new();
    details.insert("candidate_count".to_owned(), json!(0));

    let mut error_one = InteropError::new(
        TEST_PLAN_DIGEST_SHA256,
        ErrorCode::NoMatch,
        "No matching candidate.",
        None,
        details.clone(),
        Vec::new(),
    );
    error_one.error_digest_sha256 = "first".to_owned();

    let mut error_two = error_one.clone();
    error_two.error_digest_sha256 = "second".to_owned();

    assert_eq!(
        error_one.digest_sha256().expect("error digest"),
        error_two.digest_sha256().expect("error digest")
    );
    assert_eq!(error_one.schema_name, ERROR_SCHEMA_NAME);
}
