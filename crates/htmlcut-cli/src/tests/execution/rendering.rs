use super::*;

#[test]
fn bundle_document_title_prefers_core_and_then_falls_back() {
    let titled_report = build_extraction_report(
        "select",
        fixture_result(Value::String("Hello".to_owned()), ValueType::Text),
        None,
    );
    assert_eq!(bundle_document_title(&titled_report), "Fixture");

    let mut fallback_host = build_extraction_report(
        "select",
        fixture_result(Value::String("Hello".to_owned()), ValueType::Text),
        None,
    );
    fallback_host.document_title = None;
    fallback_host.source.effective_base_url =
        Some("https://example.net/docs/start.html".to_owned());
    assert_eq!(bundle_document_title(&fallback_host), "example.net");

    let mut fallback_path = build_extraction_report(
        "select",
        fixture_result(Value::String("Hello".to_owned()), ValueType::Text),
        None,
    );
    fallback_path.document_title = None;
    fallback_path.source.input_base_url = None;
    fallback_path.source.effective_base_url = None;
    fallback_path.source.value = "/tmp/sample name.html".to_owned();
    assert_eq!(bundle_document_title(&fallback_path), "sample name");

    let mut invalid_url = build_extraction_report(
        "select",
        fixture_result(Value::String("Hello".to_owned()), ValueType::Text),
        None,
    );
    invalid_url.document_title = None;
    invalid_url.source.effective_base_url = Some("not a url".to_owned());
    invalid_url.source.value = "/tmp/sample name.html".to_owned();
    assert_eq!(bundle_document_title(&invalid_url), "sample name");
}

#[test]
fn render_output_helpers_cover_text_html_json_and_none() {
    let text_report = build_extraction_report(
        "select",
        fixture_result(Value::String("Hello".to_owned()), ValueType::Text),
        None,
    );
    assert_eq!(
        render_extraction_output(&text_report, CliOutputMode::Text)
            .expect("text output")
            .expect("stdout payload"),
        "Hello"
    );

    let html_report = build_extraction_report(
        "select",
        fixture_result(
            Value::String("<p>Hello</p>".to_owned()),
            ValueType::InnerHtml,
        ),
        None,
    );
    assert!(
        render_extraction_output(&html_report, CliOutputMode::Html)
            .expect("html output")
            .expect("stdout payload")
            .contains("<p>Hello</p>")
    );
    assert!(
        render_extraction_output(&text_report, CliOutputMode::Json)
            .expect("json output")
            .expect("stdout payload")
            .contains("\"command\": \"select\"")
    );
    assert!(
        render_extraction_output(&text_report, CliOutputMode::None)
            .expect("none output")
            .is_none()
    );
}

#[test]
fn render_preview_and_source_inspection_text_are_human_readable() {
    let mut preview = build_extraction_report(
        "inspect-select",
        fixture_result(
            serde_json::json!({"tagName":"article"}),
            ValueType::Structured,
        ),
        None,
    );
    preview.operation_id = htmlcut_core::OperationId::SelectPreview;
    let preview_text = render_preview_text(&preview);
    assert!(preview_text.contains("Command: inspect-select"));
    assert!(preview_text.contains("Matches:"));
    assert!(preview_text.contains("tag: article"));
    assert!(preview_text.contains("text: Hello"));

    let mut slice_preview = build_extraction_report(
        "inspect-slice",
        fixture_result(
            serde_json::json!({"range":{"start":1,"end":18}}),
            ValueType::Structured,
        ),
        None,
    );
    slice_preview.operation_id = htmlcut_core::OperationId::SlicePreview;
    slice_preview.matches[0].path = None;
    slice_preview.matches[0].html = Some("<article>Hello</article>".to_owned());
    slice_preview.matches[0].text = Some("Hello".to_owned());
    slice_preview.matches[0].metadata =
        delimiter_metadata(1, 1, (1, 24), (10, 15), (1, 24), true, true);
    let slice_preview_text = render_preview_text(&slice_preview);
    assert!(slice_preview_text.contains("fragment: <article>Hello</article>"));
    assert!(slice_preview_text.contains("text: Hello"));
    assert!(slice_preview_text.contains("include start: true"));
    assert!(slice_preview_text.contains("matched start: <article>"));
    assert!(slice_preview_text.contains("matched end: </article>"));

    let mut inspection = fixture_inspection();
    inspection.source.load_steps = vec![
        SourceLoadStep {
            action: SourceLoadAction::HeadPreflight,
            outcome: SourceLoadOutcome::Fallback,
            status: Some(405),
            message: "HEAD returned 405, so HTMLCut fell back to GET.".to_owned(),
        },
        SourceLoadStep {
            action: SourceLoadAction::Get,
            outcome: SourceLoadOutcome::Succeeded,
            status: Some(200),
            message: "Fetched the remote source with GET.".to_owned(),
        },
    ];
    let inspection_text = render_source_inspection_text(&inspection, DEFAULT_PREVIEW_CHARS);
    assert!(inspection_text.contains("Top tags: a (2)"));
    assert!(inspection_text.contains("Link previews:"));
    assert!(inspection_text.contains("Document <base href>: ../content/"));
    assert!(inspection_text.contains("Load trace:"));
    assert!(inspection_text.contains("head preflight fallback (405)"));
    assert!(inspection_text.contains("get succeeded (200)"));

    let mut untitled = fixture_inspection();
    untitled.source.input_base_url = None;
    untitled.source.effective_base_url = None;
    let document = untitled.document.as_mut().expect("document");
    document.title = None;
    document.document_base_href = None;
    document.top_tags.clear();
    document.top_classes.clear();
    document.headings.clear();
    document.links.clear();
    let untitled_text = render_source_inspection_text(&untitled, DEFAULT_PREVIEW_CHARS);
    assert!(!untitled_text.contains("Input base URL:"));
    assert!(!untitled_text.contains("Effective base URL:"));
    assert!(!untitled_text.contains("Title:"));
    assert!(!untitled_text.contains("Document <base href>:"));
    assert!(!untitled_text.contains("Top tags:"));
    assert!(!untitled_text.contains("Top classes:"));
    assert!(!untitled_text.contains("Headings:"));
    assert!(!untitled_text.contains("Link previews:"));
}

#[test]
fn wrap_html_document_and_match_renderers_cover_remaining_paths() {
    let report = build_extraction_report(
        "select",
        fixture_result(
            Value::String("<!DOCTYPE html><html><body>Hello</body></html>".to_owned()),
            ValueType::OuterHtml,
        ),
        None,
    );
    assert!(
        wrap_html_document(&report)
            .expect("wrapped html document")
            .starts_with("<!DOCTYPE html>")
    );

    let json_match = ExtractionMatch {
        index: 1,
        path: None,
        value_type: ValueType::Structured,
        value: serde_json::json!({"hello":"world"}),
        html: None,
        text: None,
        preview: "preview".to_owned(),
        metadata: selector_metadata(1, 1, "article:nth-of-type(1)", "article", &[]),
    };
    assert!(
        render_match_as_text(&json_match)
            .expect("json text render")
            .contains("\"hello\"")
    );
    assert!(
        render_match_as_html(&json_match)
            .expect("json html render")
            .contains("<pre>")
    );

    let text_match = ExtractionMatch {
        index: 1,
        path: None,
        value_type: ValueType::Text,
        value: Value::String("Hello".to_owned()),
        html: Some("<article>Hello</article>".to_owned()),
        text: Some("Hello".to_owned()),
        preview: "Hello".to_owned(),
        metadata: selector_metadata(1, 1, "article:nth-of-type(1)", "article", &[]),
    };
    assert_eq!(
        render_match_as_html(&text_match).expect("text html render"),
        "<article>Hello</article>"
    );

    let wrapped = build_extraction_report(
        "select",
        fixture_result(
            Value::String("<p>Hello</p>".to_owned()),
            ValueType::InnerHtml,
        ),
        None,
    );
    assert!(
        wrap_html_document(&wrapped)
            .expect("wrapped html fragment")
            .contains("<section data-match-index=\"1\">")
    );
    assert!(!looks_like_document("<section>Hello</section>"));
}

#[test]
fn verbose_and_diagnostic_renderers_cover_branching_paths() {
    let mut result = fixture_result(Value::String("Hello".to_owned()), ValueType::Text);
    result.source.load_steps = vec![
        SourceLoadStep {
            action: SourceLoadAction::HeadPreflight,
            outcome: SourceLoadOutcome::Fallback,
            status: Some(405),
            message: "HEAD returned 405, so HTMLCut fell back to GET.".to_owned(),
        },
        SourceLoadStep {
            action: SourceLoadAction::Get,
            outcome: SourceLoadOutcome::Succeeded,
            status: Some(200),
            message: "Fetched the remote source with GET.".to_owned(),
        },
    ];
    let report = build_extraction_report(
        "select",
        result,
        Some(BundlePaths {
            dir: "/tmp/bundle".to_owned(),
            html: "/tmp/bundle/selection.html".to_owned(),
            text: "/tmp/bundle/selection.txt".to_owned(),
            report: "/tmp/bundle/report.json".to_owned(),
        }),
    );
    let verbose = build_verbose_lines(&report, 2);
    assert!(verbose[0].contains("selected 1 match"));
    assert!(verbose[1].contains("scanned 2 candidates"));
    assert!(verbose[2].contains("head preflight fallback (405)"));
    assert!(verbose[3].contains("get succeeded (200)"));
    assert!(build_verbose_lines(&report, 0).is_empty());
    assert_eq!(build_verbose_lines(&report, 1).len(), 1);
    let mut inspection = fixture_inspection();
    inspection.source.load_steps = report.source.load_steps.clone();
    let inspection_verbose = build_source_inspection_verbose_lines(&inspection, 2);
    assert!(inspection_verbose[0].contains("inspected 123 bytes"));
    assert!(inspection_verbose[1].contains("head preflight fallback (405)"));
    assert!(inspection_verbose[2].contains("get succeeded (200)"));
    assert_eq!(
        build_source_inspection_verbose_lines(&inspection, 1).len(),
        1
    );
    let warning_stderr = build_human_diagnostic_stderr_lines(&[Diagnostic {
        level: DiagnosticLevel::Warning,
        code: DiagnosticCode::EffectiveBaseUrlUnresolved,
        message: "warning".to_owned(),
        details: None,
    }]);
    assert_eq!(warning_stderr.len(), 1);
    assert!(warning_stderr[0].contains("htmlcut: warning EFFECTIVE_BASE_URL_UNRESOLVED"));
    assert_eq!(render_diagnostic_level(DiagnosticLevel::Warning), "warning");
    assert_eq!(render_source_kind(&SourceKind::Url), "url");
}

#[test]
fn skipped_load_traces_and_quiet_execution_cover_remaining_paths() {
    let mut preview = build_extraction_report(
        "inspect-select",
        fixture_result(
            serde_json::json!({"tagName":"article"}),
            ValueType::Structured,
        ),
        None,
    );
    preview.operation_id = htmlcut_core::OperationId::SelectPreview;
    preview.source.load_steps = vec![SourceLoadStep {
        action: SourceLoadAction::HeadPreflight,
        outcome: SourceLoadOutcome::Skipped,
        status: None,
        message: "Skipped the HEAD preflight because GET-only mode was configured.".to_owned(),
    }];
    let preview_text = render_preview_text(&preview);
    assert!(preview_text.contains("Load trace:"));
    assert!(preview_text.contains("head preflight skipped:"));

    let mut inspection = fixture_inspection();
    inspection.source.load_steps = preview.source.load_steps.clone();
    let inspection_verbose = build_source_inspection_verbose_lines(&inspection, 2);
    assert!(
        inspection_verbose[1]
            .contains("htmlcut: source load head preflight skipped: Skipped the HEAD preflight")
    );

    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture_file(
        tempdir.path(),
        "input.html",
        "<article><p>Hello</p></article>",
    );
    let input = input_path.to_string_lossy().into_owned();

    let inspect_quiet = run_inspect_source(
        InspectSourceArgs {
            source: SourceArgs {
                input: Some(input.clone()),
                base_url: None,
                max_bytes: DEFAULT_MAX_BYTES.to_string(),
                fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
                fetch_connect_timeout_ms: htmlcut_core::DEFAULT_FETCH_CONNECT_TIMEOUT_MS,
                fetch_preflight: CliFetchPreflightMode::HeadFirst,
            },
            output: CliInspectOutputMode::Text,
            include_source_text: false,
            output_file: None,
            sample_limit: DEFAULT_INSPECTION_SAMPLE_LIMIT,
            preview_chars: DEFAULT_PREVIEW_CHARS,
        },
        2,
        true,
    );
    assert_eq!(inspect_quiet.exit_code, 0);
    assert!(inspect_quiet.stderr.is_empty());
    assert!(
        inspect_quiet
            .stdout
            .as_deref()
            .is_some_and(|stdout| stdout.contains("Source: file"))
    );

    let preview_quiet = execute_preview(
        PreparedPreview::from_select_with_logging(
            InspectSelectArgs {
                definition: DefinitionArgs {
                    request_file: None,
                    emit_request_file: None,
                },
                source: SourceArgs {
                    input: Some(input),
                    base_url: None,
                    max_bytes: DEFAULT_MAX_BYTES.to_string(),
                    fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
                    fetch_connect_timeout_ms: htmlcut_core::DEFAULT_FETCH_CONNECT_TIMEOUT_MS,
                    fetch_preflight: CliFetchPreflightMode::HeadFirst,
                },
                css: Some("article".to_owned()),
                selection: SelectionArgs {
                    r#match: CliMatchMode::First,
                    index: None,
                },
                whitespace: CliWhitespaceMode::Preserve,
                rewrite_urls: false,
                output: InspectOutputArgs {
                    output: CliInspectOutputMode::Text,
                    preview_chars: DEFAULT_PREVIEW_CHARS,
                    include_source_text: false,
                    output_file: None,
                },
            },
            2,
            true,
        )
        .expect("preview builder"),
    );
    assert_eq!(preview_quiet.exit_code, 0);
    assert!(preview_quiet.stderr.is_empty());
    assert!(
        preview_quiet
            .stdout
            .as_deref()
            .is_some_and(|stdout| stdout.contains("Command: inspect-select"))
    );
}

#[test]
fn error_helpers_and_outcomes_cover_json_and_human_modes() {
    let error = usage_error(CliErrorCode::ParseError, "bad input");
    assert_eq!(exit_code_for_error(&error), EXIT_CODE_USAGE);
    let generated_diagnostics = json_error_diagnostics(&error);
    assert_eq!(generated_diagnostics.len(), 1);
    assert_eq!(
        generated_diagnostics[0].code,
        ErrorReportCode::Cli(CliErrorCode::ParseError)
    );

    let json = error_outcome("select".to_owned(), true, None, error);
    assert_eq!(json.exit_code, EXIT_CODE_USAGE);
    assert!(json.stdout.expect("json stdout").contains("\"ok\": false"));

    let human = error_outcome(
        "select".to_owned(),
        false,
        None,
        output_error(CliErrorCode::BundleTextWriteFailed, "could not write"),
    );
    assert!(human.stderr[0].contains("could not write"));

    let json_with_diagnostics = error_outcome(
        "select".to_owned(),
        true,
        None,
        usage_error_with_diagnostics(
            CliErrorCode::ParseError,
            "bad input",
            vec![Diagnostic {
                level: DiagnosticLevel::Error,
                code: DiagnosticCode::InvalidSelector,
                message: "bad input".to_owned(),
                details: None,
            }],
        ),
    );
    let existing_diagnostics = json_error_diagnostics(&usage_error_with_diagnostics(
        CliErrorCode::ParseError,
        "bad input",
        vec![Diagnostic {
            level: DiagnosticLevel::Error,
            code: DiagnosticCode::InvalidSelector,
            message: "bad input".to_owned(),
            details: None,
        }],
    ));
    assert_eq!(existing_diagnostics.len(), 1);
    assert!(
        json_with_diagnostics
            .stdout
            .expect("json stdout")
            .contains("\"diagnostics\"")
    );
    let direct_json = json_error_outcome(
        "select".to_owned(),
        None,
        usage_error(CliErrorCode::ParseError, "bad input"),
    );
    assert_eq!(direct_json.exit_code, EXIT_CODE_USAGE);
    let direct_json_report: ErrorCommandReport =
        serde_json::from_str(&direct_json.stdout.expect("json stdout")).expect("error report");
    assert_eq!(
        direct_json_report.schema_name,
        ERROR_COMMAND_REPORT_SCHEMA_NAME
    );
    assert_eq!(
        direct_json_report.schema_version,
        ERROR_COMMAND_REPORT_SCHEMA_VERSION
    );
    assert_eq!(
        direct_json_report.error.category,
        ErrorReportCategory::Usage
    );
    assert_eq!(
        direct_json_report.error.code,
        ErrorReportCode::Cli(CliErrorCode::ParseError)
    );
    assert!(direct_json_report.source_load_steps.is_empty());

    let source_trace_json = json_error_outcome(
        "select".to_owned(),
        None,
        with_source_load_steps(
            source_error(
                DiagnosticCode::SourceLoadFailed,
                "could not load",
                Vec::new(),
            ),
            &SourceMetadata {
                kind: SourceKind::Url,
                value: "https://example.com".to_owned(),
                input_base_url: Some("https://example.com".to_owned()),
                effective_base_url: Some("https://example.com".to_owned()),
                bytes_read: 0,
                load_steps: vec![SourceLoadStep {
                    action: SourceLoadAction::HeadPreflight,
                    outcome: SourceLoadOutcome::Fallback,
                    status: Some(405),
                    message: "HEAD returned 405, so HTMLCut fell back to GET.".to_owned(),
                }],
                text: None,
            },
        ),
    );
    let source_trace_report: ErrorCommandReport =
        serde_json::from_str(&source_trace_json.stdout.expect("json stdout"))
            .expect("source-trace error report");
    assert_eq!(source_trace_report.source_load_steps.len(), 1);
    assert_eq!(
        source_trace_report.source_load_steps[0].action,
        SourceLoadAction::HeadPreflight
    );
    let direct_human = human_error_outcome(output_error(
        CliErrorCode::BundleTextWriteFailed,
        "could not write",
    ));
    assert_eq!(direct_human.exit_code, EXIT_CODE_OUTPUT);
    assert!(direct_human.stderr[0].contains("could not write"));

    let json_render_failure = with_json_render_failure_for_tests(|| {
        json_error_outcome(
            "select".to_owned(),
            None,
            usage_error(CliErrorCode::ParseError, "bad input"),
        )
    });
    assert_eq!(json_render_failure.exit_code, EXIT_CODE_INTERNAL);
    assert!(json_render_failure.stdout.is_none());
    assert!(
        json_render_failure
            .stderr
            .iter()
            .any(|line| line.contains("Could not render CLI JSON payload"))
    );

    let core_error = primary_extraction_error(&[Diagnostic {
        level: DiagnosticLevel::Error,
        code: DiagnosticCode::NoMatch,
        message: "No matches".to_owned(),
        details: None,
    }]);
    assert_eq!(exit_code_for_error(&core_error), EXIT_CODE_EXTRACTION);
}

#[test]
fn typed_error_codes_render_and_compare_as_stable_strings() {
    assert_eq!(CliErrorCode::ALL.first(), Some(&CliErrorCode::ParseError));
    assert_eq!(
        CliErrorCode::ALL.last(),
        Some(&CliErrorCode::BundleTextWriteFailed)
    );
    assert_eq!(CliErrorCode::ParseError.as_str(), "CLI_PARSE_ERROR");
    assert_eq!(format!("{}", CliErrorCode::ParseError), "CLI_PARSE_ERROR");
    assert_eq!(CliErrorCode::ParseError, "CLI_PARSE_ERROR");
    assert_eq!("CLI_PARSE_ERROR", CliErrorCode::ParseError);

    let cli_code = ErrorReportCode::Cli(CliErrorCode::ParseError);
    assert_eq!(cli_code.as_str(), "CLI_PARSE_ERROR");
    assert_eq!(format!("{cli_code}"), "CLI_PARSE_ERROR");
    assert_eq!(cli_code, "CLI_PARSE_ERROR");
    assert_eq!("CLI_PARSE_ERROR", cli_code);

    let core_code = ErrorReportCode::Core(DiagnosticCode::NoMatch);
    assert_eq!(core_code.as_str(), "NO_MATCH");
    assert_eq!(format!("{core_code}"), "NO_MATCH");
    assert_eq!(core_code, "NO_MATCH");
    assert_eq!("NO_MATCH", core_code);
}
