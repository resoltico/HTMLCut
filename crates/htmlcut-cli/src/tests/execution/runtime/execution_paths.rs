use super::*;

#[test]
fn run_covers_inspection_text_failure_and_preview_modes() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture_file(
        tempdir.path(),
        "input.html",
        "<html><body><article><h1>Hello</h1><a href=\"/guide\">Guide</a></article></body></html>",
    );
    let input = input_path.to_string_lossy().into_owned();
    let missing = tempdir
        .path()
        .join("missing.html")
        .to_string_lossy()
        .into_owned();

    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "inspect".to_owned(),
        "source".to_owned(),
        input.clone(),
        "--output".to_owned(),
        "text".to_owned(),
    ]);
    assert_eq!(exit_code, 0);
    assert!(stdout.contains("Root tag: html"));
    assert!(stderr.is_empty());

    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "inspect".to_owned(),
        "source".to_owned(),
        input.clone(),
        "--base-url".to_owned(),
        "ftp://example.com".to_owned(),
    ]);
    assert_eq!(exit_code, EXIT_CODE_USAGE);
    assert!(stdout.contains("\"code\": \"CLI_BASE_URL_SCHEME_INVALID\""));
    assert!(stderr.is_empty());

    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "inspect".to_owned(),
        "source".to_owned(),
        missing.clone(),
        "--output".to_owned(),
        "json".to_owned(),
    ]);
    assert_eq!(exit_code, EXIT_CODE_SOURCE);
    assert!(stdout.contains("\"command\": \"inspect-source\""));
    assert!(stderr.is_empty());

    let (exit_code, _, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "inspect".to_owned(),
        "source".to_owned(),
        missing,
        "--output".to_owned(),
        "text".to_owned(),
    ]);
    assert_eq!(exit_code, EXIT_CODE_SOURCE);
    assert!(stderr.contains("Could not access file"));

    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "inspect".to_owned(),
        "select".to_owned(),
        input.clone(),
        "--css".to_owned(),
        "article".to_owned(),
        "--match".to_owned(),
        "nth".to_owned(),
    ]);
    assert_eq!(exit_code, EXIT_CODE_USAGE);
    assert!(stdout.contains("\"code\": \"CLI_MATCH_INDEX_REQUIRED\""));
    assert!(stderr.is_empty());

    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "inspect".to_owned(),
        "slice".to_owned(),
        input.clone(),
        "--from".to_owned(),
        "<article>".to_owned(),
        "--to".to_owned(),
        "</article>".to_owned(),
        "--regex-flags".to_owned(),
        "u".to_owned(),
    ]);
    assert_eq!(exit_code, EXIT_CODE_USAGE);
    assert!(stdout.contains("\"code\": \"CLI_REGEX_FLAGS_CONFLICT\""));
    assert!(stderr.is_empty());

    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "inspect".to_owned(),
        "select".to_owned(),
        input,
        "--css".to_owned(),
        "article".to_owned(),
        "--output".to_owned(),
        "text".to_owned(),
    ]);
    assert_eq!(exit_code, 0);
    assert!(stdout.contains("Command: inspect-select"));
    assert!(stderr.is_empty());
}

#[test]
fn run_covers_extraction_error_json_and_bundle_failure_modes() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture_file(
        tempdir.path(),
        "input.html",
        "<article><p>Hello</p></article>",
    );
    let input = input_path.to_string_lossy().into_owned();
    let bundle_path = tempdir.path().join("not-a-dir");
    fs::write(&bundle_path, "file").expect("bundle sentinel");

    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "select".to_owned(),
        input.clone(),
        "--css".to_owned(),
        "[".to_owned(),
        "--output".to_owned(),
        "json".to_owned(),
    ]);
    assert_eq!(exit_code, EXIT_CODE_USAGE);
    assert!(stdout.contains("\"code\": \"INVALID_SELECTOR\""));
    assert!(stdout.contains("Invalid selector"));
    assert!(stderr.is_empty());

    let (exit_code, _, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "slice".to_owned(),
        input.clone(),
        "--from".to_owned(),
        "<article>".to_owned(),
        "--to".to_owned(),
        "</article>".to_owned(),
        "--regex-flags".to_owned(),
        "u".to_owned(),
    ]);
    assert_eq!(exit_code, EXIT_CODE_USAGE);
    assert!(stderr.contains("--regex-flags can only be used with --pattern regex."));

    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "select".to_owned(),
        input.clone(),
        "--css".to_owned(),
        "article".to_owned(),
        "--output".to_owned(),
        "json".to_owned(),
        "--bundle".to_owned(),
        bundle_path.to_string_lossy().into_owned(),
    ]);
    assert_eq!(exit_code, EXIT_CODE_OUTPUT);
    assert!(stdout.contains("\"category\": \"output\""));
    assert!(stderr.is_empty());

    let (exit_code, stdout, _) = run_vec(vec![
        "htmlcut".to_owned(),
        "inspect".to_owned(),
        "select".to_owned(),
        input.clone(),
        "--css".to_owned(),
        "[".to_owned(),
    ]);
    assert_eq!(exit_code, EXIT_CODE_USAGE);
    assert!(stdout.contains("\"command\": \"inspect-select\""));

    let (exit_code, _, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "inspect".to_owned(),
        "slice".to_owned(),
        input,
        "--from".to_owned(),
        "[".to_owned(),
        "--to".to_owned(),
        "]".to_owned(),
        "--pattern".to_owned(),
        "regex".to_owned(),
        "--output".to_owned(),
        "text".to_owned(),
    ]);
    assert_eq!(exit_code, EXIT_CODE_USAGE);
    assert!(stderr.contains("Invalid regular expression"));
}

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
}

#[test]
fn write_bundle_reports_each_output_failure() {
    let report = build_extraction_report(
        "select",
        fixture_result(Value::String("Hello".to_owned()), ValueType::Text),
        None,
    );

    let create_dir_temp = tempdir().expect("tempdir");
    let create_dir_path = create_dir_temp.path().join("bundle");
    fs::write(&create_dir_path, "file").expect("write file");
    assert_eq!(
        write_bundle(
            &report,
            &BundlePaths {
                dir: create_dir_path.to_string_lossy().into_owned(),
                html: create_dir_path
                    .join("selection.html")
                    .to_string_lossy()
                    .into_owned(),
                text: create_dir_path
                    .join("selection.txt")
                    .to_string_lossy()
                    .into_owned(),
                report: create_dir_path
                    .join("report.json")
                    .to_string_lossy()
                    .into_owned(),
            },
        )
        .expect_err("directory creation should fail")
        .code,
        "CLI_BUNDLE_DIRECTORY_CREATE_FAILED"
    );

    let html_temp = tempdir().expect("tempdir");
    fs::create_dir(html_temp.path().join("selection.html")).expect("html dir");
    assert_eq!(
        write_bundle(
            &report,
            &BundlePaths {
                dir: html_temp.path().to_string_lossy().into_owned(),
                html: html_temp
                    .path()
                    .join("selection.html")
                    .to_string_lossy()
                    .into_owned(),
                text: html_temp
                    .path()
                    .join("selection.txt")
                    .to_string_lossy()
                    .into_owned(),
                report: html_temp
                    .path()
                    .join("report.json")
                    .to_string_lossy()
                    .into_owned(),
            },
        )
        .expect_err("html write should fail")
        .code,
        "CLI_BUNDLE_HTML_WRITE_FAILED"
    );

    let text_temp = tempdir().expect("tempdir");
    fs::create_dir(text_temp.path().join("selection.txt")).expect("text dir");
    assert_eq!(
        write_bundle(
            &report,
            &BundlePaths {
                dir: text_temp.path().to_string_lossy().into_owned(),
                html: text_temp
                    .path()
                    .join("selection.html")
                    .to_string_lossy()
                    .into_owned(),
                text: text_temp
                    .path()
                    .join("selection.txt")
                    .to_string_lossy()
                    .into_owned(),
                report: text_temp
                    .path()
                    .join("report.json")
                    .to_string_lossy()
                    .into_owned(),
            },
        )
        .expect_err("text write should fail")
        .code,
        "CLI_BUNDLE_TEXT_WRITE_FAILED"
    );

    let report_temp = tempdir().expect("tempdir");
    fs::create_dir(report_temp.path().join("report.json")).expect("report dir");
    assert_eq!(
        write_bundle(
            &report,
            &BundlePaths {
                dir: report_temp.path().to_string_lossy().into_owned(),
                html: report_temp
                    .path()
                    .join("selection.html")
                    .to_string_lossy()
                    .into_owned(),
                text: report_temp
                    .path()
                    .join("selection.txt")
                    .to_string_lossy()
                    .into_owned(),
                report: report_temp
                    .path()
                    .join("report.json")
                    .to_string_lossy()
                    .into_owned(),
            },
        )
        .expect_err("report write should fail")
        .code,
        "CLI_BUNDLE_REPORT_WRITE_FAILED"
    );
}
