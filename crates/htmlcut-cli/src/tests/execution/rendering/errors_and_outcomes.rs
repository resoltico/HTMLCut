use super::*;

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
    assert!(inspection_verbose.iter().any(|line| {
        line.contains("htmlcut: source load head preflight skipped: Skipped the HEAD preflight")
    }));

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
                input_html: None,
                base_url: None,
                max_bytes: DEFAULT_MAX_BYTES.to_string(),
                fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
                fetch_connect_timeout_ms: htmlcut_core::DEFAULT_FETCH_CONNECT_TIMEOUT_MS,
                tls_trust: CliTlsTrustMode::WebPki,
                tls_ca_bundle: None,
                fetch_preflight: CliFetchPreflightMode::HeadFirst,
            },
            output: CliInspectOutputMode::Text,
            include_source_text: false,
            output_file: None,
            sample_limit: DEFAULT_INSPECTION_SAMPLE_LIMIT,
            preview_chars: DEFAULT_PREVIEW_CHARS,
            file_write: default_output_file_write_args(),
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
                    input_html: None,
                    base_url: None,
                    max_bytes: DEFAULT_MAX_BYTES.to_string(),
                    fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
                    fetch_connect_timeout_ms: htmlcut_core::DEFAULT_FETCH_CONNECT_TIMEOUT_MS,
                    tls_trust: CliTlsTrustMode::WebPki,
                    tls_ca_bundle: None,
                    fetch_preflight: CliFetchPreflightMode::HeadFirst,
                },
                css: Some("article".to_owned()),
                selection: SelectionArgs {
                    r#match: CliMatchMode::First,
                    index: None,
                },
                value: CliValueMode::Structured,
                attribute: None,
                whitespace: CliWhitespaceMode::Rendered,
                rewrite_urls: false,
                output: InspectOutputArgs {
                    output: CliInspectOutputMode::Text,
                    preview_chars: DEFAULT_PREVIEW_CHARS,
                    include_source_text: false,
                    output_file: None,
                },
                file_write: default_preview_file_write_args(),
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
        Some(&CliErrorCode::TextProjectionMissing)
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
