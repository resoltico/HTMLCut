use super::*;
use crate::ContractValueError;
use crate::interop::v1::DomCanonicalization;
use serde_json::json;

fn interop_schema_accepts(schema_name: &str, schema_version: u32, document: &Value) -> bool {
    let schema = (crate::schema_descriptor(schema_name, schema_version)
        .expect("interop schema descriptor")
        .json_schema)()
    .expect("interop JSON schema");
    let schema_location =
        format!("https://schemas.htmlcut.invalid/{schema_name}/{schema_version}.json");
    let mut schemas = boon::Schemas::new();
    let mut compiler = boon::Compiler::new();
    compiler
        .add_resource(&schema_location, schema)
        .expect("published schema location");
    let schema_index = compiler
        .compile(&schema_location, &mut schemas)
        .expect("published schema must compile in an independent JSON Schema validator");

    schemas.validate(document, schema_index).is_ok()
}

fn canonicalization(ignored_attributes: &[&str]) -> DomCanonicalization {
    DomCanonicalization::new(
        ignored_attributes
            .iter()
            .map(|name| AttributeName::new(*name).expect("canonicalized attribute name")),
        true,
    )
}

fn result_source() -> ResultSource {
    ResultSource {
        input_base_url: None,
        effective_base_url: None,
        document_title: None,
    }
}

fn css_result(output: Output, selected: SelectedMatch) -> InteropResult {
    InteropResult::new(
        ResultExecution::new(
            TEST_PLAN_DIGEST_SHA256,
            StrategyKind::CssSelector,
            SelectionMode::Single,
            output,
            1,
        ),
        result_source(),
        vec![selected],
        Vec::new(),
    )
}

#[test]
fn interop_plan_schema_enforces_h5_canonicalization_shape_and_relationships() {
    let text_plan = selector_plan().with_dom_canonicalization(canonicalization(&["data-nonce"]));
    let text_document = serde_json::to_value(&text_plan).expect("text plan JSON");
    assert!(interop_schema_accepts(
        v1::PLAN_SCHEMA_NAME,
        v1::PLAN_SCHEMA_VERSION,
        &text_document,
    ));
    assert!(text_plan.validate().is_ok());

    let structured_plan = Plan::new(
        PlanStrategy::css_selector(css_selector("article")),
        Selection::single(),
        Output::structured(),
        Rendering::new(TextWhitespace::Normalize, false),
    )
    .with_dom_canonicalization(canonicalization(&["data-nonce"]));
    let structured_document = serde_json::to_value(&structured_plan).expect("structured plan JSON");
    assert!(interop_schema_accepts(
        v1::PLAN_SCHEMA_NAME,
        v1::PLAN_SCHEMA_VERSION,
        &structured_document,
    ));
    assert!(structured_plan.validate().is_ok());

    for output in [
        Output::inner_html(),
        Output::outer_html(),
        Output::attribute(output_attribute_name("href")),
    ] {
        let plan = Plan::new(
            PlanStrategy::css_selector(css_selector("article")),
            Selection::single(),
            output.clone(),
            Rendering::new(TextWhitespace::Normalize, false),
        )
        .with_dom_canonicalization(canonicalization(&["data-nonce"]));
        let document = serde_json::to_value(&plan).expect("raw output plan JSON");

        assert!(
            !interop_schema_accepts(v1::PLAN_SCHEMA_NAME, v1::PLAN_SCHEMA_VERSION, &document),
            "the plan schema must reject canonicalization for {} output",
            output.kind(),
        );
        assert!(matches!(
            plan.validate(),
            Err(ContractError::DomCanonicalizationRequiresComparisonTextOutput { output_kind })
                if output_kind == output.kind()
        ));
    }

    let non_css_plan = Plan::new(
        PlanStrategy::delimiter_pair(
            delimiter_boundary("<article>"),
            delimiter_boundary("</article>"),
            DelimiterMode::Literal,
            DelimiterBoundaryRetention::ExcludeBoth,
            Vec::new(),
        ),
        Selection::single(),
        Output::text(),
        Rendering::new(TextWhitespace::Normalize, false),
    )
    .with_dom_canonicalization(canonicalization(&["data-nonce"]));
    let non_css_document = serde_json::to_value(&non_css_plan).expect("non-CSS plan JSON");
    assert!(!interop_schema_accepts(
        v1::PLAN_SCHEMA_NAME,
        v1::PLAN_SCHEMA_VERSION,
        &non_css_document,
    ));
    assert!(matches!(
        non_css_plan.validate(),
        Err(ContractError::DomCanonicalizationRequiresCssSelector)
    ));

    for removed_control in ["sort_attributes", "strip_comments"] {
        let mut document = text_document.clone();
        document["dom_canonicalization"]
            .as_object_mut()
            .expect("canonicalization object")
            .insert(removed_control.to_owned(), json!(true));

        assert!(
            !interop_schema_accepts(v1::PLAN_SCHEMA_NAME, v1::PLAN_SCHEMA_VERSION, &document),
            "the plan schema must reject removed {removed_control} control",
        );
        assert!(serde_json::from_value::<Plan>(document).is_err());
    }
}

#[test]
fn interop_result_schema_enforces_h5_evidence_boundaries_and_marks_the_runtime_boundary() {
    let mut canonical_text_match = selector_selected_match();
    canonical_text_match.comparison_text_output = Some("Canonical text".to_owned());
    canonical_text_match.output_value = json!("Canonical text");
    let canonical_text_result = css_result(Output::text(), canonical_text_match)
        .with_computed_digest()
        .expect("valid canonical text result");
    let canonical_text_document =
        serde_json::to_value(&canonical_text_result).expect("canonical text result JSON");
    assert!(interop_schema_accepts(
        v1::RESULT_SCHEMA_NAME,
        v1::RESULT_SCHEMA_VERSION,
        &canonical_text_document,
    ));
    assert!(canonical_text_result.validate().is_ok());

    let mut raw_output_result = canonical_text_result.clone();
    raw_output_result.output = Output::attribute(output_attribute_name("href"));
    let raw_output_document =
        serde_json::to_value(&raw_output_result).expect("raw output result JSON");
    assert!(!interop_schema_accepts(
        v1::RESULT_SCHEMA_NAME,
        v1::RESULT_SCHEMA_VERSION,
        &raw_output_document,
    ));
    assert!(matches!(
        raw_output_result.validate(),
        Err(ContractError::UnexpectedComparisonTextOutputForOutput {
            output_kind: OutputKind::Attribute
        })
    ));

    let mut raw_value_result = css_result(Output::inner_html(), selector_selected_match());
    raw_value_result.selected_matches[0].output_value = json!({ "invented": "object" });
    let raw_value_document =
        serde_json::to_value(&raw_value_result).expect("raw value result JSON");
    assert!(!interop_schema_accepts(
        v1::RESULT_SCHEMA_NAME,
        v1::RESULT_SCHEMA_VERSION,
        &raw_value_document,
    ));
    assert!(matches!(
        raw_value_result.validate(),
        Err(ContractError::NonStringOutputValue {
            output_kind: OutputKind::InnerHtml
        })
    ));

    let mut structured_match = selector_selected_match();
    structured_match.comparison_text_output = Some("Canonical text".to_owned());
    structured_match.output_value = json!({ "textOutput": "raw text" });
    let structured_result = css_result(Output::structured(), structured_match)
        .with_computed_digest()
        .expect("valid structured result");
    let structured_document =
        serde_json::to_value(&structured_result).expect("structured result JSON");
    assert!(interop_schema_accepts(
        v1::RESULT_SCHEMA_NAME,
        v1::RESULT_SCHEMA_VERSION,
        &structured_document,
    ));
    assert!(structured_result.validate().is_ok());

    let mut structured_leak = structured_result.clone();
    structured_leak.selected_matches[0].output_value = json!({
        "textOutput": "raw text",
        "comparisonTextOutput": "invented clone text"
    });
    let structured_leak_document =
        serde_json::to_value(&structured_leak).expect("structured leak result JSON");
    assert!(!interop_schema_accepts(
        v1::RESULT_SCHEMA_NAME,
        v1::RESULT_SCHEMA_VERSION,
        &structured_leak_document,
    ));
    assert!(matches!(
        structured_leak.validate(),
        Err(ContractError::StructuredOutputContainsComparisonText)
    ));

    let mut text_value_mismatch = canonical_text_result.clone();
    text_value_mismatch.selected_matches[0].output_value = json!("Invented output");
    let text_value_mismatch_document =
        serde_json::to_value(&text_value_mismatch).expect("text mismatch result JSON");
    assert!(
        interop_schema_accepts(
            v1::RESULT_SCHEMA_NAME,
            v1::RESULT_SCHEMA_VERSION,
            &text_value_mismatch_document,
        ),
        "standard JSON Schema cannot compare output_value with comparison_text_output",
    );
    assert!(matches!(
        text_value_mismatch.validate(),
        Err(ContractError::TextOutputValueMismatch)
    ));
}

#[test]
fn interop_error_and_result_schemas_publish_message_length_limits() {
    let oversized = "x".repeat(1025);
    let error = InteropError::new(
        TEST_PLAN_DIGEST_SHA256,
        ErrorCode::InternalError,
        oversized.clone(),
        None,
        BTreeMap::new(),
        Vec::new(),
    );
    let error_document = serde_json::to_value(&error).expect("error JSON");
    assert!(!interop_schema_accepts(
        v1::ERROR_SCHEMA_NAME,
        v1::ERROR_SCHEMA_VERSION,
        &error_document,
    ));

    let result = InteropResult::new(
        ResultExecution::new(
            TEST_PLAN_DIGEST_SHA256,
            StrategyKind::CssSelector,
            SelectionMode::Single,
            Output::text(),
            1,
        ),
        ResultSource {
            input_base_url: None,
            effective_base_url: None,
            document_title: None,
        },
        selector_selected_matches(),
        vec![InteropDiagnostic {
            level: InteropDiagnosticLevel::Warning,
            code: InteropDiagnosticCode::EffectiveBaseUrlUnresolved,
            message: oversized,
            details: None,
        }],
    );
    let result_document = serde_json::to_value(&result).expect("result JSON");
    assert!(!interop_schema_accepts(
        v1::RESULT_SCHEMA_NAME,
        v1::RESULT_SCHEMA_VERSION,
        &result_document,
    ));
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
    assert_eq!(Selection::all().mode(), SelectionMode::All);

    let source = HtmlInput::new("inline", "<article>Hello</article>").expect("source");
    assert_eq!(source.label, "inline");
    assert_eq!(source.html, "<article>Hello</article>");

    let with_base = source.with_input_base_url(http_url("https://example.com/start.html"));
    assert_eq!(
        with_base.input_base_url.as_ref().map(HttpUrl::as_fetch_str),
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
fn interop_owned_value_objects_and_diagnostic_helpers_cover_public_surface() {
    let selector = CssSelectorText::new("article h1").expect("selector");
    assert_eq!(selector.as_str(), "article h1");
    assert_eq!(selector.as_ref(), "article h1");
    assert_eq!(selector.to_string(), "article h1");
    let selector_from_string: CssSelectorText = String::from("article h2")
        .try_into()
        .expect("selector from string");
    assert_eq!(selector_from_string.as_str(), "article h2");
    assert!(matches!(
        CssSelectorText::new("   "),
        Err(ContractError::EmptyCssSelector)
    ));

    let boundary = DelimiterBoundaryText::new("<article>").expect("boundary");
    assert_eq!(boundary.as_str(), "<article>");
    assert_eq!(boundary.as_ref(), "<article>");
    assert_eq!(boundary.to_string(), "<article>");
    let boundary_from_string: DelimiterBoundaryText = String::from("</article>")
        .try_into()
        .expect("boundary from string");
    assert_eq!(boundary_from_string.as_str(), "</article>");
    assert!(matches!(
        DelimiterBoundaryText::new("   "),
        Err(ContractError::EmptyDelimiterBoundary)
    ));

    assert_eq!(
        InteropDiagnosticCode::from(DiagnosticCode::UnsupportedValueType),
        InteropDiagnosticCode::UnsupportedValueType
    );

    let attribute = AttributeName::new("href").expect("attribute");
    assert_eq!(attribute.as_str(), "href");
    assert_eq!(attribute.as_ref(), "href");
    assert_eq!(attribute.to_string(), "href");
    let attribute_from_string: AttributeName = String::from("data-id")
        .try_into()
        .expect("attribute from string");
    assert_eq!(attribute_from_string.as_str(), "data-id");
    assert!(matches!(
        AttributeName::new("   "),
        Err(ContractValueError::Empty {
            field: "attribute name"
        })
    ));
    assert!(matches!(
        AttributeName::new("href value"),
        Err(ContractValueError::ContainsWhitespace {
            field: "attribute name"
        })
    ));

    let output_kinds = [
        (Output::text(), OutputKind::Text, "text"),
        (Output::inner_html(), OutputKind::InnerHtml, "inner_html"),
        (Output::outer_html(), OutputKind::OuterHtml, "outer_html"),
        (
            Output::selected_html(),
            OutputKind::SelectedHtml,
            "selected_html",
        ),
        (
            Output::attribute(output_attribute_name("href")),
            OutputKind::Attribute,
            "attribute",
        ),
        (Output::structured(), OutputKind::Structured, "structured"),
    ];
    for (output, kind, stable_text) in output_kinds {
        assert_eq!(output.kind(), kind);
        assert_eq!(kind.as_str(), stable_text);
        assert_eq!(kind.to_string(), stable_text);
    }

    assert_eq!(
        InteropDiagnosticCode::SourceLoadFailed,
        "SOURCE_LOAD_FAILED"
    );
    assert_eq!(
        "SOURCE_LOAD_FAILED",
        InteropDiagnosticCode::SourceLoadFailed
    );
    assert_eq!(
        InteropDiagnosticCode::SourceLoadFailed.to_string(),
        "SOURCE_LOAD_FAILED"
    );

    let diagnostic_code_mappings = [
        (
            crate::DiagnosticCode::SourceLoadFailed,
            InteropDiagnosticCode::SourceLoadFailed,
        ),
        (
            crate::DiagnosticCode::UnsupportedSpecVersion,
            InteropDiagnosticCode::UnsupportedSpecVersion,
        ),
        (
            crate::DiagnosticCode::InvalidSelector,
            InteropDiagnosticCode::InvalidSelector,
        ),
        (
            crate::DiagnosticCode::InvalidSlicePattern,
            InteropDiagnosticCode::InvalidSlicePattern,
        ),
        (
            crate::DiagnosticCode::NoMatch,
            InteropDiagnosticCode::NoMatch,
        ),
        (
            crate::DiagnosticCode::AmbiguousMatch,
            InteropDiagnosticCode::AmbiguousMatch,
        ),
        (
            crate::DiagnosticCode::MatchIndexOutOfRange,
            InteropDiagnosticCode::MatchIndexOutOfRange,
        ),
        (
            crate::DiagnosticCode::MissingAttribute,
            InteropDiagnosticCode::MissingAttribute,
        ),
        (
            crate::DiagnosticCode::MultipleMatches,
            InteropDiagnosticCode::MultipleMatches,
        ),
        (
            crate::DiagnosticCode::EffectiveBaseUrlUnresolved,
            InteropDiagnosticCode::EffectiveBaseUrlUnresolved,
        ),
        (
            crate::DiagnosticCode::SliceSplitsMarkup,
            InteropDiagnosticCode::SliceSplitsMarkup,
        ),
    ];
    for (core_code, expected_code) in diagnostic_code_mappings {
        assert_eq!(InteropDiagnosticCode::from(core_code), expected_code);
    }

    assert_eq!(
        InteropDiagnosticLevel::from(crate::DiagnosticLevel::Error),
        InteropDiagnosticLevel::Error
    );
    assert_eq!(
        InteropDiagnosticLevel::from(crate::DiagnosticLevel::Warning),
        InteropDiagnosticLevel::Warning
    );
    assert_eq!(
        InteropDiagnosticLevel::from(crate::DiagnosticLevel::Info),
        InteropDiagnosticLevel::Info
    );

    assert_eq!(
        ByteRange::from(&Range { start: 4, end: 9 }),
        ByteRange { start: 4, end: 9 }
    );

    let invalid_strategy_output = Plan::new(
        PlanStrategy::css_selector(css_selector("article")),
        Selection::single(),
        Output::selected_html(),
        Rendering::new(TextWhitespace::Normalize, false),
    );
    let invalid_strategy_error = invalid_strategy_output
        .validate()
        .expect_err("selected_html should be rejected for css selectors");
    assert!(matches!(
        invalid_strategy_error,
        ContractError::UnsupportedOutputForStrategy {
            strategy_kind: StrategyKind::CssSelector,
            output_kind: OutputKind::SelectedHtml
        }
    ));
}
