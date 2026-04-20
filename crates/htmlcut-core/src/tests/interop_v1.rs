use super::*;
use crate::interop::v1::{
    self, ContractError, DelimiterMode, ErrorCode, HtmlInput, InteropError, InteropResult,
    Normalization, Output, OutputKind, Plan, PlanStrategy, RegexFlag, ResultExecution,
    ResultSource, SelectedMatch, SelectedMatchMetadata, Selection, SelectionMode, StrategyKind,
    TextWhitespace,
};
use crate::result::{
    DelimiterPairMatchMetadata, ExtractionMatch, ExtractionStats, Range, SelectorMatchMetadata,
};
use std::collections::BTreeMap;

fn selector_plan() -> Plan {
    Plan::new(
        PlanStrategy::css_selector(selector_query("article")),
        Selection::single(),
        Output::new(OutputKind::Text),
        Normalization::new(TextWhitespace::Normalize, false),
    )
}

fn delimiter_plan() -> Plan {
    Plan::new(
        PlanStrategy::delimiter_pair(
            slice_boundary("<article>"),
            slice_boundary("</article>"),
            DelimiterMode::Regex,
            true,
            false,
            vec![
                RegexFlag::CaseInsensitive,
                RegexFlag::MultiLine,
                RegexFlag::DotMatchesNewLine,
                RegexFlag::SwapGreed,
                RegexFlag::IgnoreWhitespace,
            ],
        ),
        Selection::nth(NonZeroUsize::new(2).expect("index")),
        Output::new(OutputKind::OuterHtml),
        Normalization::new(TextWhitespace::Preserve, true),
    )
}

fn selector_selected_match() -> SelectedMatch {
    SelectedMatch {
        candidate_index: NonZeroUsize::new(1).expect("candidate index"),
        value_kind: OutputKind::Text,
        value: "Hello".to_owned(),
        comparison_input_text: "Hello".to_owned(),
        inner_html: Some("Hello".to_owned()),
        outer_html: Some("<article>Hello</article>".to_owned()),
        metadata: SelectedMatchMetadata::CssSelector {
            candidate_count: 1,
            candidate_index: NonZeroUsize::new(1).expect("candidate index"),
            path: "html:nth-of-type(1) > body:nth-of-type(1) > article:nth-of-type(1)".to_owned(),
            tag_name: "article".to_owned(),
        },
    }
}

fn selector_core_match(index: usize, candidate_index: usize) -> ExtractionMatch {
    ExtractionMatch {
        index,
        path: Some(format!("article:nth-of-type({index})")),
        value_type: ValueType::Structured,
        value: json!({
            "html": format!("<article data-index=\"{index}\">Hello</article>"),
            "text": "Hello",
            "outerHtml": format!("<article data-index=\"{index}\">Hello</article>")
        }),
        html: Some(format!("<article data-index=\"{index}\">Hello</article>")),
        text: Some("Hello".to_owned()),
        preview: "Hello".to_owned(),
        metadata: ExtractionMatchMetadata::Selector(SelectorMatchMetadata {
            candidate_count: index,
            candidate_index,
            path: format!("article:nth-of-type({index})"),
            tag_name: "article".to_owned(),
            attributes: BTreeMap::new(),
        }),
    }
}

fn successful_selector_extraction(
    matches: Vec<ExtractionMatch>,
    candidate_count: usize,
    effective_base_url: Option<&str>,
) -> ExtractionResult {
    ExtractionResult {
        operation_id: crate::OperationId::SelectExtract,
        schema_name: crate::CORE_RESULT_SCHEMA_NAME.to_owned(),
        schema_version: crate::CORE_RESULT_SCHEMA_VERSION,
        ok: true,
        source: SourceMetadata {
            kind: SourceKind::Memory,
            value: "inline".to_owned(),
            input_base_url: Some("https://example.com/start.html".to_owned()),
            effective_base_url: effective_base_url.map(str::to_owned),
            bytes_read: 22,
            load_steps: Vec::new(),
            text: None,
        },
        document_title: Some("Example".to_owned()),
        extraction: ExtractionSpec::selector(selector_query("article")),
        stats: ExtractionStats {
            duration_ms: 1,
            candidate_count,
            match_count: matches.len(),
        },
        matches,
        diagnostics: Vec::new(),
    }
}

#[test]
fn interop_public_helpers_cover_selection_modes_and_html_input_paths() {
    assert!(selector_plan().validate().is_ok());
    assert_eq!(Selection::single().mode(), SelectionMode::Single);
    assert_eq!(Selection::first().mode(), SelectionMode::First);
    assert_eq!(
        Selection::nth(NonZeroUsize::new(3).expect("index")).mode(),
        SelectionMode::Nth
    );

    let source = HtmlInput::new("inline", "<article>Hello</article>").expect("source");
    assert_eq!(source.to_source_request().kind(), SourceKind::Memory);
    assert_eq!(
        source.clone().into_source_request().kind(),
        SourceKind::Memory
    );

    let with_base =
        source.with_input_base_url(Url::parse("https://example.com/start.html").expect("url"));
    assert_eq!(
        with_base
            .clone()
            .into_source_request()
            .base_url
            .as_ref()
            .map(Url::as_str),
        Some("https://example.com/start.html")
    );
}

#[test]
fn interop_schema_identity_helpers_reject_name_version_and_profile_drift() {
    let name_error = v1::validate_schema_identity_for_tests(
        "wrong",
        v1::PLAN_SCHEMA_NAME,
        v1::PLAN_SCHEMA_VERSION,
        v1::PLAN_SCHEMA_VERSION,
        v1::INTEROP_V1_PROFILE,
        v1::INTEROP_V1_PROFILE,
    )
    .expect_err("schema name drift");
    assert!(matches!(
        name_error,
        ContractError::InvalidIdentity {
            field: "schema_name",
            ..
        }
    ));

    let version_error = v1::validate_schema_identity_for_tests(
        v1::PLAN_SCHEMA_NAME,
        v1::PLAN_SCHEMA_NAME,
        99,
        v1::PLAN_SCHEMA_VERSION,
        v1::INTEROP_V1_PROFILE,
        v1::INTEROP_V1_PROFILE,
    )
    .expect_err("schema version drift");
    assert!(matches!(
        version_error,
        ContractError::InvalidVersion {
            field: "schema_version",
            ..
        }
    ));

    let profile_error = v1::validate_schema_identity_for_tests(
        v1::PLAN_SCHEMA_NAME,
        v1::PLAN_SCHEMA_NAME,
        v1::PLAN_SCHEMA_VERSION,
        v1::PLAN_SCHEMA_VERSION,
        "wrong-profile",
        v1::INTEROP_V1_PROFILE,
    )
    .expect_err("interop profile drift");
    assert!(matches!(
        profile_error,
        ContractError::InvalidIdentity {
            field: "interop_profile",
            ..
        }
    ));

    let mut plan = selector_plan();
    plan.schema_name = "wrong".to_owned();
    let plan_error = plan
        .validate()
        .expect_err("public plan validate should reject drift");
    assert!(matches!(
        plan_error,
        ContractError::InvalidIdentity {
            field: "schema_name",
            ..
        }
    ));
}

#[test]
fn interop_result_validation_rejects_zero_counts_range_drift_and_metadata_mismatch() {
    let execution = ResultExecution::new(
        "plan-digest",
        StrategyKind::CssSelector,
        SelectionMode::Single,
        1,
    );
    let source = ResultSource {
        input_base_url: None,
        effective_base_url: None,
        document_title: None,
    };

    let mut zero = InteropResult::new(
        ResultExecution::new(
            "plan-digest",
            StrategyKind::CssSelector,
            SelectionMode::Single,
            0,
        ),
        source.clone(),
        selector_selected_match(),
        Vec::new(),
    );
    let zero_error = zero.validate().expect_err("zero candidates should fail");
    assert!(matches!(zero_error, ContractError::ZeroCandidateCount));

    let mut out_of_range = InteropResult::new(
        execution.clone(),
        source.clone(),
        selector_selected_match(),
        Vec::new(),
    );
    out_of_range.candidate_count = 1;
    out_of_range.selected_match.candidate_index = NonZeroUsize::new(2).expect("index");
    let range_error = out_of_range
        .validate()
        .expect_err("selected candidate out of range");
    assert!(matches!(
        range_error,
        ContractError::SelectedCandidateOutOfRange {
            selected: 2,
            candidate_count: 1
        }
    ));

    let valid_result = InteropResult::new(
        execution.clone(),
        source.clone(),
        selector_selected_match(),
        Vec::new(),
    );
    assert!(valid_result.validate().is_ok());

    let mut mismatched_kind =
        InteropResult::new(execution, source, selector_selected_match(), Vec::new());
    mismatched_kind.strategy_kind = StrategyKind::DelimiterPair;
    let mismatch_error = mismatched_kind
        .validate()
        .expect_err("metadata kind mismatch");
    assert!(matches!(
        mismatch_error,
        ContractError::MetadataKindMismatch {
            strategy_kind: StrategyKind::DelimiterPair,
            metadata_kind: StrategyKind::CssSelector
        }
    ));

    let mut mismatched_metadata = InteropResult::new(
        ResultExecution::new(
            "plan-digest",
            StrategyKind::CssSelector,
            SelectionMode::Single,
            2,
        ),
        ResultSource {
            input_base_url: None,
            effective_base_url: None,
            document_title: None,
        },
        selector_selected_match(),
        Vec::new(),
    );
    let SelectedMatchMetadata::CssSelector {
        ref mut candidate_count,
        ..
    } = mismatched_metadata.selected_match.metadata
    else {
        unreachable!("selector metadata");
    };
    *candidate_count = 1;
    let metadata_error = mismatched_metadata
        .validate()
        .expect_err("metadata cardinality mismatch");
    assert!(matches!(
        metadata_error,
        ContractError::SelectedCandidateOutOfRange {
            selected: 1,
            candidate_count: 2
        }
    ));

    let mut metadata_index_mismatch = InteropResult::new(
        ResultExecution::new(
            "plan-digest",
            StrategyKind::CssSelector,
            SelectionMode::Single,
            1,
        ),
        ResultSource {
            input_base_url: None,
            effective_base_url: None,
            document_title: None,
        },
        selector_selected_match(),
        Vec::new(),
    );
    let SelectedMatchMetadata::CssSelector {
        ref mut candidate_index,
        ..
    } = metadata_index_mismatch.selected_match.metadata
    else {
        unreachable!("selector metadata");
    };
    *candidate_index = NonZeroUsize::new(2).expect("candidate index");
    let index_mismatch_error = metadata_index_mismatch
        .validate()
        .expect_err("metadata candidate index mismatch");
    assert!(matches!(
        index_mismatch_error,
        ContractError::SelectedCandidateOutOfRange {
            selected: 1,
            candidate_count: 1
        }
    ));

    let mut result_identity_drift = InteropResult::new(
        ResultExecution::new(
            "plan-digest",
            StrategyKind::CssSelector,
            SelectionMode::Single,
            1,
        ),
        ResultSource {
            input_base_url: None,
            effective_base_url: None,
            document_title: None,
        },
        selector_selected_match(),
        Vec::new(),
    );
    result_identity_drift.schema_name = "wrong".to_owned();
    let identity_error = result_identity_drift
        .validate()
        .expect_err("public result validate should reject drift");
    assert!(matches!(
        identity_error,
        ContractError::InvalidIdentity {
            field: "schema_name",
            ..
        }
    ));

    zero.result_digest_sha256 = "ignored".to_owned();
}

#[test]
fn interop_stable_json_digest_helpers_cover_object_and_scalar_values() {
    let object_digest =
        v1::digest_stable_json_omitting_field_for_tests(&json!({"keep": 1, "drop": 2}), "drop")
            .expect("object digest");
    let expected_object_digest =
        v1::digest_stable_json_omitting_field_for_tests(&json!({"keep": 1}), "drop")
            .expect("object digest");
    assert_eq!(object_digest, expected_object_digest);

    let scalar_digest = v1::digest_stable_json_omitting_field_for_tests(&json!("value"), "drop")
        .expect("scalar digest");
    let expected_scalar_digest =
        v1::digest_stable_json_omitting_field_for_tests(&json!("value"), "other")
            .expect("scalar digest");
    assert_eq!(scalar_digest, expected_scalar_digest);
}

#[test]
fn interop_execution_helpers_cover_compile_projection_and_error_paths() {
    let selector_source =
        HtmlInput::new("inline", "<article>Hello</article>").expect("selector source");
    let selector_request = v1::compile_request_for_tests(&selector_source, &selector_plan());
    assert_eq!(
        selector_request.extraction.strategy(),
        ExtractionStrategy::Selector
    );
    assert_eq!(
        selector_request.normalization.whitespace,
        WhitespaceMode::Normalize
    );
    assert!(!selector_request.normalization.rewrite_urls);
    let first_request = v1::compile_request_for_tests(
        &selector_source,
        &Plan::new(
            PlanStrategy::css_selector(selector_query("article")),
            Selection::first(),
            Output::new(OutputKind::Text),
            Normalization::new(TextWhitespace::Normalize, false),
        ),
    );
    assert!(matches!(
        first_request.extraction.selection(),
        SelectionSpec::First
    ));

    let delimiter_source =
        HtmlInput::new("inline", "<article>Hello</article>").expect("delimiter source");
    let delimiter_request = v1::compile_request_for_tests(&delimiter_source, &delimiter_plan());
    assert_eq!(
        delimiter_request.extraction.strategy(),
        ExtractionStrategy::Slice
    );
    assert_eq!(
        delimiter_request.normalization.whitespace,
        WhitespaceMode::Preserve
    );
    assert!(delimiter_request.normalization.rewrite_urls);
    assert_eq!(
        v1::compile_regex_flags_for_tests(&[
            RegexFlag::CaseInsensitive,
            RegexFlag::MultiLine,
            RegexFlag::DotMatchesNewLine,
            RegexFlag::SwapGreed,
            RegexFlag::IgnoreWhitespace,
        ]),
        format!("{DEFAULT_REGEX_FLAGS}imsUx")
    );

    let selector_match = ExtractionMatch {
        index: 1,
        path: Some("article:nth-of-type(1)".to_owned()),
        value_type: ValueType::Structured,
        value: json!({
            "html": "<article>Hello</article>",
            "text": "Hello",
            "outerHtml": "<article>Hello</article>"
        }),
        html: Some("<article>Hello</article>".to_owned()),
        text: Some("Hello".to_owned()),
        preview: "Hello".to_owned(),
        metadata: ExtractionMatchMetadata::Selector(SelectorMatchMetadata {
            candidate_count: 1,
            candidate_index: 1,
            path: "article:nth-of-type(1)".to_owned(),
            tag_name: "article".to_owned(),
            attributes: BTreeMap::new(),
        }),
    };
    assert!(
        v1::project_structured_match_for_tests(&selector_match, StrategyKind::CssSelector, &[])
            .is_ok()
    );

    let delimiter_match = ExtractionMatch {
        index: 1,
        path: None,
        value_type: ValueType::Structured,
        value: json!({
            "html": "<article>Hello</article>",
            "text": "Hello",
            "innerHtml": "Hello",
            "outerHtml": "<article>Hello</article>"
        }),
        html: Some("<article>Hello</article>".to_owned()),
        text: Some("Hello".to_owned()),
        preview: "Hello".to_owned(),
        metadata: ExtractionMatchMetadata::DelimiterPair(DelimiterPairMatchMetadata {
            candidate_count: 1,
            candidate_index: 1,
            selected_range: Range { start: 0, end: 22 },
            inner_range: Range { start: 9, end: 14 },
            outer_range: Range { start: 0, end: 22 },
            include_start: true,
            include_end: false,
            matched_start: "<article>".to_owned(),
            matched_end: "</article>".to_owned(),
        }),
    };
    assert!(
        v1::project_structured_match_for_tests(&delimiter_match, StrategyKind::DelimiterPair, &[])
            .is_ok()
    );

    let adapted_text = v1::adapt_successful_extraction_for_tests(
        &selector_source
            .clone()
            .with_input_base_url(Url::parse("https://example.com/start.html").expect("url")),
        &selector_plan(),
        successful_selector_extraction(
            vec![selector_core_match(1, 1)],
            1,
            Some("https://example.com/base.html"),
        ),
    )
    .expect("adapted text result");
    assert_eq!(adapted_text.selected_match.value_kind, OutputKind::Text);
    assert_eq!(adapted_text.selected_match.value, "Hello");
    assert_eq!(
        adapted_text
            .source
            .effective_base_url
            .as_ref()
            .map(Url::as_str),
        Some("https://example.com/base.html")
    );

    let adapted_inner = v1::adapt_successful_extraction_for_tests(
        &selector_source,
        &Plan::new(
            PlanStrategy::css_selector(selector_query("article")),
            Selection::single(),
            Output::new(OutputKind::InnerHtml),
            Normalization::new(TextWhitespace::Normalize, false),
        ),
        successful_selector_extraction(vec![selector_core_match(1, 1)], 1, None),
    )
    .expect("adapted inner-html result");
    assert!(adapted_inner.selected_match.value.contains("<article"));

    let adapted_outer = v1::adapt_successful_extraction_for_tests(
        &selector_source,
        &Plan::new(
            PlanStrategy::css_selector(selector_query("article")),
            Selection::single(),
            Output::new(OutputKind::OuterHtml),
            Normalization::new(TextWhitespace::Normalize, false),
        ),
        successful_selector_extraction(vec![selector_core_match(1, 1)], 1, None),
    )
    .expect("adapted outer-html result");
    assert!(adapted_outer.selected_match.value.contains("<article"));

    let adapted_projection_error = v1::adapt_successful_extraction_for_tests(
        &selector_source,
        &selector_plan(),
        successful_selector_extraction(
            vec![ExtractionMatch {
                value: json!({"text": "Hello", "outerHtml": "<article>Hello</article>"}),
                ..selector_core_match(1, 1)
            }],
            1,
            None,
        ),
    )
    .expect_err("projection failure should surface as interop error");
    assert_eq!(
        adapted_projection_error.error_code,
        ErrorCode::InternalError
    );
    assert!(adapted_projection_error.message.contains("\"html\""));

    let adapted_url_error = v1::adapt_successful_extraction_for_tests(
        &selector_source,
        &selector_plan(),
        successful_selector_extraction(vec![selector_core_match(1, 1)], 1, Some("not a url")),
    )
    .expect_err("invalid effective base URL should surface as interop error");
    assert_eq!(adapted_url_error.error_code, ErrorCode::InternalError);
    assert!(adapted_url_error.message.contains("invalid URL"));

    let no_match_adapter_error = v1::adapt_successful_extraction_for_tests(
        &selector_source,
        &selector_plan(),
        successful_selector_extraction(Vec::new(), 0, None),
    )
    .expect_err("missing selected match");
    assert_eq!(no_match_adapter_error.error_code, ErrorCode::InternalError);
    assert!(
        no_match_adapter_error
            .message
            .contains("did not produce a selected match")
    );

    let multi_match_adapter_error = v1::adapt_successful_extraction_for_tests(
        &selector_source,
        &selector_plan(),
        successful_selector_extraction(
            vec![selector_core_match(1, 1), selector_core_match(2, 2)],
            2,
            None,
        ),
    )
    .expect_err("multiple selected matches");
    assert_eq!(
        multi_match_adapter_error.error_code,
        ErrorCode::InternalError
    );
    assert!(
        multi_match_adapter_error
            .message
            .contains("exactly one selected match")
    );

    let non_object_error = v1::project_structured_match_for_tests(
        &ExtractionMatch {
            value: json!("not-object"),
            ..selector_match.clone()
        },
        StrategyKind::CssSelector,
        &[],
    )
    .expect_err("non-object structured payload");
    assert_eq!(non_object_error.error_code, ErrorCode::InternalError);
    assert!(
        non_object_error
            .message
            .contains("structured core match payload")
    );

    let missing_field_error = v1::project_structured_match_for_tests(
        &ExtractionMatch {
            value: json!({"html": "<article>Hello</article>", "text": "Hello"}),
            ..selector_match.clone()
        },
        StrategyKind::CssSelector,
        &[],
    )
    .expect_err("missing outerHtml");
    assert_eq!(missing_field_error.error_code, ErrorCode::InternalError);
    assert!(missing_field_error.message.contains("\"outerHtml\""));

    let selector_zero_index_error = v1::project_structured_match_for_tests(
        &ExtractionMatch {
            metadata: ExtractionMatchMetadata::Selector(SelectorMatchMetadata {
                candidate_count: 1,
                candidate_index: 0,
                path: "article:nth-of-type(1)".to_owned(),
                tag_name: "article".to_owned(),
                attributes: BTreeMap::new(),
            }),
            ..selector_match.clone()
        },
        StrategyKind::CssSelector,
        &[],
    )
    .expect_err("zero selector candidate index");
    assert_eq!(
        selector_zero_index_error.error_code,
        ErrorCode::InternalError
    );
    assert!(
        selector_zero_index_error
            .message
            .contains("zero candidate index")
    );

    let selector_missing_text_error = v1::project_structured_match_for_tests(
        &ExtractionMatch {
            value: json!({
                "html": "<article>Hello</article>",
                "outerHtml": "<article>Hello</article>"
            }),
            ..selector_match.clone()
        },
        StrategyKind::CssSelector,
        &[],
    )
    .expect_err("missing text");
    assert_eq!(
        selector_missing_text_error.error_code,
        ErrorCode::InternalError
    );
    assert!(selector_missing_text_error.message.contains("\"text\""));

    let delimiter_missing_html_error = v1::project_structured_match_for_tests(
        &ExtractionMatch {
            value: json!({
                "text": "Hello",
                "innerHtml": "Hello",
                "outerHtml": "<article>Hello</article>"
            }),
            ..delimiter_match.clone()
        },
        StrategyKind::DelimiterPair,
        &[],
    )
    .expect_err("missing delimiter html");
    assert_eq!(
        delimiter_missing_html_error.error_code,
        ErrorCode::InternalError
    );
    assert!(delimiter_missing_html_error.message.contains("\"html\""));

    let delimiter_missing_text_error = v1::project_structured_match_for_tests(
        &ExtractionMatch {
            value: json!({
                "html": "<article>Hello</article>",
                "innerHtml": "Hello",
                "outerHtml": "<article>Hello</article>"
            }),
            ..delimiter_match.clone()
        },
        StrategyKind::DelimiterPair,
        &[],
    )
    .expect_err("missing delimiter text");
    assert_eq!(
        delimiter_missing_text_error.error_code,
        ErrorCode::InternalError
    );
    assert!(delimiter_missing_text_error.message.contains("\"text\""));

    let delimiter_missing_inner_html_error = v1::project_structured_match_for_tests(
        &ExtractionMatch {
            value: json!({
                "html": "<article>Hello</article>",
                "text": "Hello",
                "outerHtml": "<article>Hello</article>"
            }),
            ..delimiter_match.clone()
        },
        StrategyKind::DelimiterPair,
        &[],
    )
    .expect_err("missing delimiter innerHtml");
    assert_eq!(
        delimiter_missing_inner_html_error.error_code,
        ErrorCode::InternalError
    );
    assert!(
        delimiter_missing_inner_html_error
            .message
            .contains("\"innerHtml\"")
    );

    let delimiter_missing_outer_html_error = v1::project_structured_match_for_tests(
        &ExtractionMatch {
            value: json!({
                "html": "<article>Hello</article>",
                "text": "Hello",
                "innerHtml": "Hello"
            }),
            ..delimiter_match.clone()
        },
        StrategyKind::DelimiterPair,
        &[],
    )
    .expect_err("missing delimiter outerHtml");
    assert_eq!(
        delimiter_missing_outer_html_error.error_code,
        ErrorCode::InternalError
    );
    assert!(
        delimiter_missing_outer_html_error
            .message
            .contains("\"outerHtml\"")
    );

    let zero_index_error = v1::project_structured_match_for_tests(
        &ExtractionMatch {
            metadata: ExtractionMatchMetadata::DelimiterPair(DelimiterPairMatchMetadata {
                candidate_count: 1,
                candidate_index: 0,
                selected_range: Range { start: 0, end: 22 },
                inner_range: Range { start: 9, end: 14 },
                outer_range: Range { start: 0, end: 22 },
                include_start: true,
                include_end: false,
                matched_start: "<article>".to_owned(),
                matched_end: "</article>".to_owned(),
            }),
            ..delimiter_match.clone()
        },
        StrategyKind::DelimiterPair,
        &[],
    )
    .expect_err("zero candidate index");
    assert_eq!(zero_index_error.error_code, ErrorCode::InternalError);
    assert!(zero_index_error.message.contains("zero candidate index"));

    let invalid_url_error =
        v1::parse_optional_url_for_tests(Some("not a url"), "effective_base_url", &[])
            .expect_err("invalid url");
    assert_eq!(invalid_url_error.error_code, ErrorCode::InternalError);
    assert!(invalid_url_error.message.contains("invalid URL"));
    assert_eq!(
        v1::parse_optional_url_for_tests(None, "effective_base_url", &[]).expect("none url"),
        None
    );

    let no_primary_error = v1::core_execution_error_for_tests(&selector_plan(), &[]);
    assert_eq!(no_primary_error.error_code, ErrorCode::InternalError);
    assert!(
        no_primary_error
            .message
            .contains("without an error diagnostic")
    );

    let no_match_error = v1::core_execution_error_for_tests(
        &selector_plan(),
        &[Diagnostic {
            level: DiagnosticLevel::Error,
            code: DiagnosticCode::NoMatch.to_string(),
            message: "no match".to_owned(),
            details: None,
        }],
    );
    assert_eq!(no_match_error.error_code, ErrorCode::NoMatch);

    let ambiguous_error = v1::core_execution_error_for_tests(
        &selector_plan(),
        &[Diagnostic {
            level: DiagnosticLevel::Error,
            code: DiagnosticCode::AmbiguousMatch.to_string(),
            message: "ambiguous".to_owned(),
            details: Some(json!({"candidateCount": 2})),
        }],
    );
    assert_eq!(ambiguous_error.error_code, ErrorCode::AmbiguousMatch);
    assert!(ambiguous_error.details.contains_key("core_details"));

    let invalid_request_error = v1::core_execution_error_for_tests(
        &selector_plan(),
        &[Diagnostic {
            level: DiagnosticLevel::Error,
            code: DiagnosticCode::InvalidRequest.to_string(),
            message: "invalid".to_owned(),
            details: None,
        }],
    );
    assert_eq!(invalid_request_error.error_code, ErrorCode::PlanInvalid);

    let unknown_code_error = v1::core_execution_error_for_tests(
        &selector_plan(),
        &[Diagnostic {
            level: DiagnosticLevel::Error,
            code: "NOT_A_REAL_CODE".to_owned(),
            message: "weird".to_owned(),
            details: None,
        }],
    );
    assert_eq!(unknown_code_error.error_code, ErrorCode::InternalError);

    let adapter_error = v1::internal_adapter_error_for_tests(
        "adapter failure",
        BTreeMap::from([("field".to_owned(), Value::from("effective_base_url"))]),
        Vec::new(),
    );
    assert_eq!(adapter_error.error_code, ErrorCode::InternalError);
    assert_eq!(adapter_error.error_digest_sha256.len(), 64);

    let _typed: Box<InteropError> = Box::new(adapter_error);
}
