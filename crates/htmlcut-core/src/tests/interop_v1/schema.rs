use super::*;

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

    let with_base =
        source.with_input_base_url(Url::parse("https://example.com/start.html").expect("url"));
    assert_eq!(
        with_base.input_base_url.as_ref().map(Url::as_str),
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

    let attribute = OutputAttributeName::new("href").expect("attribute");
    assert_eq!(attribute.as_str(), "href");
    assert_eq!(attribute.as_ref(), "href");
    assert_eq!(attribute.to_string(), "href");
    let attribute_from_string: OutputAttributeName = String::from("data-id")
        .try_into()
        .expect("attribute from string");
    assert_eq!(attribute_from_string.as_str(), "data-id");
    assert!(matches!(
        OutputAttributeName::new("   "),
        Err(ContractError::EmptyAttributeName)
    ));
    assert!(matches!(
        OutputAttributeName::new("href value"),
        Err(ContractError::AttributeNameContainsWhitespace)
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
