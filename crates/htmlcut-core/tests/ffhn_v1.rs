use std::collections::BTreeMap;
use std::num::NonZeroUsize;

use htmlcut_core::interop::ffhn_v1::{
    FFHN_ERROR_SCHEMA_NAME, FFHN_PLAN_SCHEMA_NAME, FFHN_RESULT_SCHEMA_NAME, FfhnDelimiterMode,
    FfhnError, FfhnErrorCode, FfhnInteropError, FfhnNormalization, FfhnOutput, FfhnOutputKind,
    FfhnPlan, FfhnPlanStrategy, FfhnRegexFlag, FfhnResult, FfhnResultExecution, FfhnResultSource,
    FfhnSelectedMatch, FfhnSelectedMatchMetadata, FfhnSelection, FfhnSelectionMode,
    FfhnSourceInput, execute_ffhn_plan, stable_json_v1, validate_ffhn_plan,
};
use htmlcut_core::{
    Diagnostic, DiagnosticLevel, Range, SelectorQuery, SliceBoundary, SourceInput, SourceKind,
};
use serde_json::json;
use url::Url;

fn selector_query(selector: &str) -> SelectorQuery {
    SelectorQuery::new(selector).expect("selector")
}

fn slice_boundary(boundary: &str) -> SliceBoundary {
    SliceBoundary::new(boundary).expect("slice boundary")
}

fn selector_match() -> FfhnSelectedMatch {
    FfhnSelectedMatch {
        candidate_index: NonZeroUsize::new(1).expect("candidate index"),
        value_kind: FfhnOutputKind::OuterHtml,
        value: "<article>Hello</article>".to_owned(),
        comparison_input_text: "Hello".to_owned(),
        inner_html: Some("Hello".to_owned()),
        outer_html: Some("<article>Hello</article>".to_owned()),
        metadata: FfhnSelectedMatchMetadata::CssSelector {
            candidate_count: 1,
            candidate_index: NonZeroUsize::new(1).expect("candidate index"),
            path: "html:nth-of-type(1) > body:nth-of-type(1) > article:nth-of-type(1)".to_owned(),
            tag_name: "article".to_owned(),
        },
    }
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
fn ffhn_source_input_builds_memory_source_request() {
    let base_url = Url::parse("https://example.com/start.html").expect("base url");
    let source = FfhnSourceInput::new("target-news", "<article>Hello</article>")
        .expect("source input")
        .with_input_base_url(base_url.clone())
        .into_source_request();

    assert_eq!(source.kind(), SourceKind::Memory);
    assert_eq!(source.base_url, Some(base_url));
    match source.input {
        SourceInput::Memory { label, text } => {
            assert_eq!(label, "target-news");
            assert_eq!(text, "<article>Hello</article>");
        }
        other => panic!("expected memory source, got {other:?}"),
    }
}

#[test]
fn ffhn_source_input_rejects_blank_labels() {
    let error = FfhnSourceInput::new("   ", "<article>Hello</article>").expect_err("blank label");
    assert!(matches!(error, FfhnInteropError::EmptySourceLabel));
}

#[test]
fn ffhn_plan_validates_literal_regex_flag_conflicts() {
    let plan = FfhnPlan::new(
        FfhnPlanStrategy::delimiter_pair(
            slice_boundary("<article>"),
            slice_boundary("</article>"),
            FfhnDelimiterMode::Literal,
            false,
            false,
            vec![FfhnRegexFlag::CaseInsensitive],
        ),
        FfhnSelection::single(),
        FfhnOutput::new(FfhnOutputKind::Text),
        FfhnNormalization::new(
            htmlcut_core::interop::ffhn_v1::FfhnWhitespaceMode::Normalize,
            false,
        ),
    );

    let error = plan.validate().expect_err("literal flags should fail");
    assert!(matches!(error, FfhnInteropError::LiteralDelimiterFlags));
}

#[test]
fn ffhn_plan_uses_frozen_schema_identity() {
    let plan = FfhnPlan::new(
        FfhnPlanStrategy::css_selector(selector_query("article")),
        FfhnSelection::first(),
        FfhnOutput::new(FfhnOutputKind::Text),
        FfhnNormalization::new(
            htmlcut_core::interop::ffhn_v1::FfhnWhitespaceMode::Normalize,
            true,
        ),
    );

    assert_eq!(plan.schema_name, FFHN_PLAN_SCHEMA_NAME);
    assert_eq!(plan.digest_sha256().expect("plan digest").len(), 64);
}

#[test]
fn ffhn_result_digest_ignores_existing_digest_field() {
    let mut result_one = FfhnResult::new(
        FfhnResultExecution::new(
            "plan-digest",
            htmlcut_core::interop::ffhn_v1::FfhnStrategyKind::CssSelector,
            FfhnSelectionMode::Single,
            1,
        ),
        FfhnResultSource {
            input_base_url: Some(Url::parse("https://example.com/start.html").expect("url")),
            effective_base_url: Some(Url::parse("https://example.com/base/").expect("url")),
            document_title: Some("Example".to_owned()),
        },
        selector_match(),
        vec![Diagnostic {
            level: DiagnosticLevel::Warning,
            code: "EFFECTIVE_BASE_URL_UNRESOLVED".to_owned(),
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
    assert_eq!(result_one.schema_name, FFHN_RESULT_SCHEMA_NAME);
}

#[test]
fn ffhn_result_validation_rejects_error_diagnostics() {
    let result = FfhnResult::new(
        FfhnResultExecution::new(
            "plan-digest",
            htmlcut_core::interop::ffhn_v1::FfhnStrategyKind::CssSelector,
            FfhnSelectionMode::Single,
            1,
        ),
        FfhnResultSource {
            input_base_url: None,
            effective_base_url: None,
            document_title: None,
        },
        selector_match(),
        vec![Diagnostic {
            level: DiagnosticLevel::Error,
            code: "NO_MATCH".to_owned(),
            message: "should not be present on success".to_owned(),
            details: None,
        }],
    );

    let error = result
        .validate()
        .expect_err("error diagnostics should fail");
    assert!(matches!(error, FfhnInteropError::ErrorDiagnosticsInSuccess));
}

#[test]
fn ffhn_error_digest_ignores_existing_digest_field() {
    let mut details = BTreeMap::new();
    details.insert("candidate_count".to_owned(), json!(0));

    let mut error_one = FfhnError::new(
        "plan-digest",
        FfhnErrorCode::NoMatch,
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
    assert_eq!(error_one.schema_name, FFHN_ERROR_SCHEMA_NAME);
}

#[test]
fn ffhn_result_round_trips_through_stable_json() {
    let result = FfhnResult::new(
        FfhnResultExecution::new(
            "plan-digest",
            htmlcut_core::interop::ffhn_v1::FfhnStrategyKind::DelimiterPair,
            FfhnSelectionMode::Nth,
            3,
        ),
        FfhnResultSource {
            input_base_url: None,
            effective_base_url: None,
            document_title: Some("Example".to_owned()),
        },
        FfhnSelectedMatch {
            candidate_index: NonZeroUsize::new(2).expect("candidate index"),
            value_kind: FfhnOutputKind::Text,
            value: "Hello".to_owned(),
            comparison_input_text: "Hello".to_owned(),
            inner_html: Some("Hello".to_owned()),
            outer_html: Some("<article>Hello</article>".to_owned()),
            metadata: FfhnSelectedMatchMetadata::DelimiterPair {
                candidate_count: 3,
                candidate_index: NonZeroUsize::new(2).expect("candidate index"),
                selected_range: Range { start: 10, end: 15 },
                inner_range: Range { start: 11, end: 14 },
                outer_range: Range { start: 9, end: 16 },
                include_start: true,
                include_end: false,
            },
        },
        Vec::new(),
    )
    .with_computed_digest()
    .expect("digest");

    let stable = result.stable_json().expect("stable json");
    let round_trip: FfhnResult = serde_json::from_str(&stable).expect("round trip result");

    assert_eq!(round_trip, result);
}

#[test]
fn validate_ffhn_plan_returns_typed_plan_invalid_error() {
    let plan = FfhnPlan::new(
        FfhnPlanStrategy::delimiter_pair(
            slice_boundary("<article>"),
            slice_boundary("</article>"),
            FfhnDelimiterMode::Literal,
            false,
            false,
            vec![FfhnRegexFlag::CaseInsensitive],
        ),
        FfhnSelection::single(),
        FfhnOutput::new(FfhnOutputKind::Text),
        FfhnNormalization::new(
            htmlcut_core::interop::ffhn_v1::FfhnWhitespaceMode::Normalize,
            false,
        ),
    );

    let error = validate_ffhn_plan(&plan).expect_err("invalid plan");
    assert_eq!(error.error_code, FfhnErrorCode::PlanInvalid);
    assert_eq!(
        error.strategy_kind,
        Some(htmlcut_core::interop::ffhn_v1::FfhnStrategyKind::DelimiterPair)
    );
    assert_eq!(error.plan_digest_sha256.len(), 64);
    assert_eq!(error.error_digest_sha256.len(), 64);
    assert!(
        error
            .details
            .get("interop_error")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|message| message.contains("delimiter_pair flags"))
    );
}

#[test]
fn execute_ffhn_plan_executes_css_selector_with_rewritten_outer_html() {
    let source = FfhnSourceInput::new(
        "target-story",
        "<html><head><title>Example</title></head><body><article><a href=\"guide.html\">Guide</a></article></body></html>",
    )
    .expect("source")
    .with_input_base_url(Url::parse("https://example.com/docs/start.html").expect("url"));
    let plan = FfhnPlan::new(
        FfhnPlanStrategy::css_selector(selector_query("article a")),
        FfhnSelection::single(),
        FfhnOutput::new(FfhnOutputKind::OuterHtml),
        FfhnNormalization::new(
            htmlcut_core::interop::ffhn_v1::FfhnWhitespaceMode::Normalize,
            true,
        ),
    );

    let result = execute_ffhn_plan(&source, &plan).expect("ffhn result");

    assert_eq!(result.schema_name, FFHN_RESULT_SCHEMA_NAME);
    assert_eq!(result.plan_digest_sha256.len(), 64);
    assert_eq!(result.result_digest_sha256.len(), 64);
    assert_eq!(
        result.source.input_base_url,
        Some(Url::parse("https://example.com/docs/start.html").expect("url"))
    );
    assert_eq!(
        result.source.effective_base_url,
        Some(Url::parse("https://example.com/docs/start.html").expect("url"))
    );
    assert_eq!(result.source.document_title.as_deref(), Some("Example"));
    assert_eq!(result.candidate_count, 1);
    assert_eq!(result.selected_match.value_kind, FfhnOutputKind::OuterHtml);
    assert_eq!(
        result.selected_match.value,
        "<a href=\"https://example.com/docs/guide.html\">Guide</a>"
    );
    assert_eq!(result.selected_match.comparison_input_text, "Guide");
    assert_eq!(result.selected_match.inner_html.as_deref(), Some("Guide"));
    assert_eq!(
        result.selected_match.outer_html.as_deref(),
        Some("<a href=\"https://example.com/docs/guide.html\">Guide</a>")
    );
    match result.selected_match.metadata {
        FfhnSelectedMatchMetadata::CssSelector {
            candidate_count,
            candidate_index,
            ref path,
            ref tag_name,
        } => {
            assert_eq!(candidate_count, 1);
            assert_eq!(candidate_index.get(), 1);
            assert!(path.contains("article:nth-of-type(1)"));
            assert_eq!(tag_name, "a");
        }
        other => panic!("expected selector metadata, got {other:?}"),
    }
}

#[test]
fn execute_ffhn_plan_executes_regex_delimiter_pair() {
    let source = FfhnSourceInput::new("target-article", "<ARTICLE data-id=\"7\">Hello</ARTICLE>")
        .expect("source");
    let plan = FfhnPlan::new(
        FfhnPlanStrategy::delimiter_pair(
            slice_boundary(r"<article[^>]*>"),
            slice_boundary(r"</article>"),
            FfhnDelimiterMode::Regex,
            true,
            true,
            vec![FfhnRegexFlag::CaseInsensitive],
        ),
        FfhnSelection::single(),
        FfhnOutput::new(FfhnOutputKind::InnerHtml),
        FfhnNormalization::new(
            htmlcut_core::interop::ffhn_v1::FfhnWhitespaceMode::Normalize,
            false,
        ),
    );

    let result = execute_ffhn_plan(&source, &plan).expect("ffhn result");

    assert_eq!(result.selected_match.value_kind, FfhnOutputKind::InnerHtml);
    assert_eq!(
        result.selected_match.value,
        "<ARTICLE data-id=\"7\">Hello</ARTICLE>"
    );
    assert_eq!(result.selected_match.comparison_input_text, "Hello");
    assert_eq!(result.selected_match.inner_html.as_deref(), Some("Hello"));
    assert_eq!(
        result.selected_match.outer_html.as_deref(),
        Some("<ARTICLE data-id=\"7\">Hello</ARTICLE>")
    );
    match result.selected_match.metadata {
        FfhnSelectedMatchMetadata::DelimiterPair {
            candidate_count,
            candidate_index,
            selected_range,
            inner_range,
            outer_range,
            include_start,
            include_end,
        } => {
            assert_eq!(candidate_count, 1);
            assert_eq!(candidate_index.get(), 1);
            assert_eq!(selected_range, Range { start: 0, end: 36 });
            assert_eq!(inner_range, Range { start: 21, end: 26 });
            assert_eq!(outer_range, Range { start: 0, end: 36 });
            assert!(include_start);
            assert!(include_end);
        }
        other => panic!("expected delimiter metadata, got {other:?}"),
    }
}

#[test]
fn execute_ffhn_plan_maps_ambiguous_single_to_ambiguous_match_error() {
    let source = FfhnSourceInput::new(
        "target-news",
        "<article>One</article><article>Two</article>",
    )
    .expect("source");
    let plan = FfhnPlan::new(
        FfhnPlanStrategy::css_selector(selector_query("article")),
        FfhnSelection::single(),
        FfhnOutput::new(FfhnOutputKind::Text),
        FfhnNormalization::new(
            htmlcut_core::interop::ffhn_v1::FfhnWhitespaceMode::Normalize,
            false,
        ),
    );

    let error = execute_ffhn_plan(&source, &plan).expect_err("ambiguous match");

    assert_eq!(error.error_code, FfhnErrorCode::AmbiguousMatch);
    assert_eq!(
        error.strategy_kind,
        Some(htmlcut_core::interop::ffhn_v1::FfhnStrategyKind::CssSelector)
    );
    assert_eq!(error.plan_digest_sha256.len(), 64);
    assert_eq!(error.error_digest_sha256.len(), 64);
    assert_eq!(error.diagnostics[0].code, "AMBIGUOUS_MATCH");
    assert!(error.message.contains("exactly one candidate"));
}
