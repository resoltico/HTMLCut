use std::collections::BTreeMap;
use std::num::NonZeroUsize;

use super::{displayed_http_url, http_url};
use crate::DEFAULT_MAX_BYTES;
use crate::interop::v1::{
    ByteRange, ContractError, CssSelectorText, DelimiterBoundaryRetention, DelimiterBoundaryText,
    DelimiterMode, ERROR_SCHEMA_NAME, ErrorCode, HtmlInput, InteropDiagnostic,
    InteropDiagnosticCode, InteropDiagnosticLevel, InteropError, InteropResult, Output, OutputKind,
    PLAN_SCHEMA_NAME, Plan, PlanStrategy, RESULT_SCHEMA_NAME, RegexFlag, Rendering,
    ResultExecution, ResultSource, SelectedMatch, SelectedMatchMetadata, Selection, SelectionMode,
    StrategyKind, TextWhitespace, execute_plan, execute_validated_plan, prepare_plan,
    stable_json_v1,
};
use serde_json::json;

const TEST_PLAN_DIGEST_SHA256: &str =
    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn css_selector(selector: &str) -> CssSelectorText {
    CssSelectorText::new(selector).expect("selector")
}

fn delimiter_boundary(boundary: &str) -> DelimiterBoundaryText {
    DelimiterBoundaryText::new(boundary).expect("slice boundary")
}

fn selector_match() -> SelectedMatch {
    SelectedMatch {
        candidate_index: NonZeroUsize::new(1).expect("candidate index"),
        output_value: json!("<article>Hello</article>"),
        text_output: "Hello".to_owned(),
        selected_html_output: None,
        inner_html_output: "Hello".to_owned(),
        outer_html_output: "<article>Hello</article>".to_owned(),
        metadata: SelectedMatchMetadata::CssSelector {
            candidate_count: 1,
            candidate_index: NonZeroUsize::new(1).expect("candidate index"),
            path: "html:nth-of-type(1) > body:nth-of-type(1) > article:nth-of-type(1)".to_owned(),
            tag_name: "article".to_owned(),
            attributes: BTreeMap::new(),
        },
    }
}

fn selected_matches(selected_match: SelectedMatch) -> Vec<SelectedMatch> {
    vec![selected_match]
}

fn only_selected_match(result: &InteropResult) -> &SelectedMatch {
    result
        .selected_matches
        .first()
        .expect("interop result should carry one selected match")
}

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
