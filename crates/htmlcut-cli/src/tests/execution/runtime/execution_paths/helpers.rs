use super::*;

#[test]
fn helper_branches_cover_remaining_rendering_validation_and_error_paths() {
    assert_eq!(
        default_output_for_value(&ValueType::InnerHtml),
        CliOutputMode::Html
    );
    assert_eq!(
        default_output_for_value(&ValueType::OuterHtml),
        CliOutputMode::Html
    );
    assert_eq!(
        validate_base_url(None).expect("missing base url is okay"),
        None
    );
    assert!(validate_base_url(Some("::not-a-url::")).is_err());
    assert!(validate_base_url(Some("ftp://example.com")).is_err());
    assert!(
        resolve_selection_spec(&SelectionArgs {
            r#match: CliMatchMode::Nth,
            index: Some(0),
        })
        .is_err()
    );
    assert_eq!(
        resolve_value_spec(CliValueMode::InnerHtml, None)
            .expect("html value")
            .value_type(),
        ValueType::InnerHtml
    );
    assert_eq!(
        resolve_value_spec(CliValueMode::OuterHtml, None)
            .expect("outer html value")
            .value_type(),
        ValueType::OuterHtml
    );

    let mut preview = build_extraction_report(
        "inspect-slice",
        fixture_result(
            serde_json::json!({"tagName":"article"}),
            ValueType::Structured,
        ),
        None,
    );
    preview.matches.clear();
    preview.diagnostics.push(Diagnostic {
        level: DiagnosticLevel::Info,
        code: "NOTE".to_owned(),
        message: "preview note".to_owned(),
        details: None,
    });
    let preview_text = render_preview_text(&preview);
    assert!(preview_text.contains("Diagnostics:"));
    assert!(!preview_text.contains("Matches:"));

    let mut empty_inspection = fixture_inspection();
    empty_inspection.document = None;
    empty_inspection.diagnostics.push(Diagnostic {
        level: DiagnosticLevel::Warning,
        code: "WARN".to_owned(),
        message: "watch out".to_owned(),
        details: None,
    });
    let inspection_text = render_source_inspection_text(&empty_inspection, DEFAULT_PREVIEW_CHARS);
    assert!(inspection_text.contains("Effective base URL:"));
    assert!(inspection_text.contains("Diagnostics:"));
    assert!(!inspection_text.contains("Headings:"));

    let mut link_variants = fixture_inspection();
    link_variants.document.as_mut().expect("document").links = vec![
        LinkInspection {
            text: "Docs".to_owned(),
            href: Some("https://example.com/docs".to_owned()),
            resolved_href: Some("https://example.com/docs".to_owned()),
            path: "a:nth-of-type(1)".to_owned(),
        },
        LinkInspection {
            text: "Bare".to_owned(),
            href: None,
            resolved_href: None,
            path: "a:nth-of-type(2)".to_owned(),
        },
    ];
    let link_text = render_source_inspection_text(&link_variants, DEFAULT_PREVIEW_CHARS);
    assert!(link_text.contains("- Docs [https://example.com/docs] [a:nth-of-type(1)]"));
    assert!(link_text.contains("- Bare [a:nth-of-type(2)]"));

    let mut plural_report = build_extraction_report(
        "select",
        fixture_result(Value::String("Hello".to_owned()), ValueType::Text),
        None,
    );
    plural_report.stats.match_count = 2;
    let verbose = build_verbose_lines(&plural_report, 2);
    assert!(verbose[0].contains("selected 2 matches"));
    assert_eq!(render_diagnostic_level(DiagnosticLevel::Error), "error");
    assert_eq!(render_diagnostic_level(DiagnosticLevel::Info), "info");
    assert_eq!(render_source_kind(&SourceKind::File), "file");
    assert_eq!(render_source_kind(&SourceKind::Stdin), "stdin");
    assert_eq!(render_source_kind(&SourceKind::Memory), "memory");

    let mut wrapped = build_extraction_report(
        "select",
        fixture_result(
            Value::String("<p>Hello</p>".to_owned()),
            ValueType::InnerHtml,
        ),
        None,
    );
    wrapped.document_title = None;
    wrapped.source.effective_base_url = Some("https://example.net/docs/start.html".to_owned());
    assert!(wrap_html_document(&wrapped).contains("<title>example.net</title>"));

    let source = primary_extraction_error(&[Diagnostic {
        level: DiagnosticLevel::Error,
        code: "SOURCE_LOAD_FAILED".to_owned(),
        message: "boom".to_owned(),
        details: None,
    }]);
    assert_eq!(exit_code_for_error(&source), EXIT_CODE_SOURCE);

    let usage = primary_extraction_error(&[Diagnostic {
        level: DiagnosticLevel::Error,
        code: "INVALID_REQUEST".to_owned(),
        message: "bad".to_owned(),
        details: None,
    }]);
    assert_eq!(exit_code_for_error(&usage), EXIT_CODE_USAGE);

    let extraction = primary_extraction_error(&[Diagnostic {
        level: DiagnosticLevel::Error,
        code: "AMBIGUOUS_MATCH".to_owned(),
        message: "too many".to_owned(),
        details: None,
    }]);
    assert_eq!(exit_code_for_error(&extraction), EXIT_CODE_EXTRACTION);

    let internal = primary_extraction_error(&[Diagnostic {
        level: DiagnosticLevel::Error,
        code: "SURPRISE".to_owned(),
        message: "unexpected".to_owned(),
        details: None,
    }]);
    assert_eq!(exit_code_for_error(&internal), EXIT_CODE_INTERNAL);
    assert_eq!(
        exit_code_for_error(&primary_extraction_error(&[])),
        EXIT_CODE_INTERNAL
    );

    let inspection_source = primary_source_inspection_error(&[Diagnostic {
        level: DiagnosticLevel::Error,
        code: "SOURCE_LOAD_FAILED".to_owned(),
        message: "missing".to_owned(),
        details: None,
    }]);
    assert_eq!(exit_code_for_error(&inspection_source), EXIT_CODE_SOURCE);
    assert_eq!(
        exit_code_for_error(&primary_source_inspection_error(&[Diagnostic {
            level: DiagnosticLevel::Error,
            code: "OTHER".to_owned(),
            message: "other".to_owned(),
            details: None,
        }])),
        EXIT_CODE_INTERNAL
    );
    assert_eq!(
        exit_code_for_error(&primary_source_inspection_error(&[])),
        EXIT_CODE_INTERNAL
    );

    assert_eq!(render_error_category(CliErrorCategory::Usage), "usage");
    assert_eq!(render_error_category(CliErrorCategory::Source), "source");
    assert_eq!(
        render_error_category(CliErrorCategory::Extraction),
        "extraction"
    );
    assert_eq!(render_error_category(CliErrorCategory::Output), "output");
    assert_eq!(
        render_error_category(CliErrorCategory::Internal),
        "internal"
    );

    let human = error_outcome(
        "select".to_owned(),
        false,
        None,
        source_error("SRC", "could not load", Vec::new()),
    );
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit_code = write_outcome(human, &mut stdout, &mut stderr);
    assert_eq!(exit_code, EXIT_CODE_SOURCE);
    assert!(stdout.is_empty());
    assert!(
        String::from_utf8(stderr)
            .expect("stderr")
            .contains("could not load")
    );

    let contract_drift = operation_error_outcome_for_tests(
        htmlcut_core::OperationId::SelectExtract,
        true,
        None,
        usage_error("CLI_PARSE_ERROR", "bad invocation"),
        None,
    );
    assert_eq!(contract_drift.exit_code, EXIT_CODE_INTERNAL);
    assert!(
        contract_drift
            .stdout
            .as_deref()
            .expect("json stdout")
            .contains("\"code\": \"CLI_CONTRACT_MISSING\"")
    );
    assert!(
        contract_drift
            .stdout
            .as_deref()
            .expect("json stdout")
            .contains("\"command\": \"select.extract\"")
    );
}
