use super::*;

#[test]
fn json_render_failures_fall_back_to_human_errors_across_execution_paths() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture_file(
        tempdir.path(),
        "input.html",
        "<article><p>Hello</p></article>",
    );
    let input = input_path.to_string_lossy().into_owned();
    let missing = tempdir
        .path()
        .join("missing.html")
        .to_string_lossy()
        .into_owned();

    let inspect_failure = with_json_render_failure_for_tests(|| {
        run_inspect_source(
            InspectSourceArgs {
                source: SourceArgs {
                    input: Some(missing.clone()),
                    input_html: None,
                    base_url: None,
                    max_bytes: DEFAULT_MAX_BYTES.to_string(),
                    fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
                    fetch_connect_timeout_ms: htmlcut_core::DEFAULT_FETCH_CONNECT_TIMEOUT_MS,
                    tls_trust: CliTlsTrustMode::WebPki,
                    tls_ca_bundle: None,
                    fetch_preflight: CliFetchPreflightMode::HeadFirst,
                },
                output: CliInspectOutputMode::Json,
                include_source_text: false,
                output_file: None,
                sample_limit: DEFAULT_INSPECTION_SAMPLE_LIMIT,
                preview_chars: DEFAULT_PREVIEW_CHARS,
                file_write: default_output_file_write_args(),
            },
            0,
            false,
        )
    });
    assert_eq!(inspect_failure.exit_code, EXIT_CODE_INTERNAL);
    assert!(inspect_failure.stdout.is_none());
    assert!(
        inspect_failure
            .stderr
            .iter()
            .any(|line| line.contains("Could not render CLI JSON payload"))
    );

    let inspect_success = with_json_render_failure_for_tests(|| {
        run_inspect_source(
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
                output: CliInspectOutputMode::Json,
                include_source_text: false,
                output_file: None,
                sample_limit: DEFAULT_INSPECTION_SAMPLE_LIMIT,
                preview_chars: DEFAULT_PREVIEW_CHARS,
                file_write: default_output_file_write_args(),
            },
            0,
            false,
        )
    });
    assert_eq!(inspect_success.exit_code, EXIT_CODE_INTERNAL);
    assert!(inspect_success.stdout.is_none());

    let preview_failure = with_json_render_failure_for_tests(|| {
        execute_preview(
            PreparedPreview::from_select(InspectSelectArgs {
                definition: DefinitionArgs {
                    request_file: None,
                    emit_request_file: None,
                },
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
                css: Some("[".to_owned()),
                selection: SelectionArgs {
                    r#match: CliMatchMode::First,
                    index: None,
                },
                value: CliValueMode::Structured,
                attribute: None,
                whitespace: CliWhitespaceMode::Rendered,
                rewrite_urls: false,
                output: InspectOutputArgs {
                    output: CliInspectOutputMode::Json,
                    preview_chars: DEFAULT_PREVIEW_CHARS,
                    include_source_text: false,
                    output_file: None,
                },
                file_write: default_preview_file_write_args(),
            })
            .expect("preview builder"),
        )
    });
    assert_eq!(preview_failure.exit_code, EXIT_CODE_INTERNAL);
    assert!(preview_failure.stdout.is_none());

    let preview_success = with_json_render_failure_for_tests(|| {
        execute_preview(
            PreparedPreview::from_select(InspectSelectArgs {
                definition: DefinitionArgs {
                    request_file: None,
                    emit_request_file: None,
                },
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
                    output: CliInspectOutputMode::Json,
                    preview_chars: DEFAULT_PREVIEW_CHARS,
                    include_source_text: false,
                    output_file: None,
                },
                file_write: default_preview_file_write_args(),
            })
            .expect("preview builder"),
        )
    });
    assert_eq!(preview_success.exit_code, EXIT_CODE_INTERNAL);
    assert!(preview_success.stdout.is_none());

    let extraction_failure = with_json_render_failure_for_tests(|| {
        execute_extraction(
            PreparedExtraction::from_select(SelectArgs {
                definition: DefinitionArgs {
                    request_file: None,
                    emit_request_file: None,
                },
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
                css: Some("[".to_owned()),
                selection: SelectionArgs {
                    r#match: CliMatchMode::First,
                    index: None,
                },
                output: ExtractOutputArgs {
                    value: CliValueMode::Structured,
                    attribute: None,
                    whitespace: CliWhitespaceMode::Rendered,
                    rewrite_urls: false,
                    output: Some(CliOutputMode::Json),
                    bundle: None,
                    preview_chars: DEFAULT_PREVIEW_CHARS,
                    include_source_text: false,
                    output_file: None,
                },
                file_write: default_file_write_args(),
            })
            .expect("extract builder"),
        )
    });
    assert_eq!(extraction_failure.exit_code, EXIT_CODE_INTERNAL);
    assert!(extraction_failure.stdout.is_none());

    let extraction_success = with_json_render_failure_for_tests(|| {
        execute_extraction(
            PreparedExtraction::from_select(SelectArgs {
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
                output: ExtractOutputArgs {
                    value: CliValueMode::Structured,
                    attribute: None,
                    whitespace: CliWhitespaceMode::Rendered,
                    rewrite_urls: false,
                    output: Some(CliOutputMode::Json),
                    bundle: None,
                    preview_chars: DEFAULT_PREVIEW_CHARS,
                    include_source_text: false,
                    output_file: None,
                },
                file_write: default_file_write_args(),
            })
            .expect("extract builder"),
        )
    });
    assert_eq!(extraction_success.exit_code, EXIT_CODE_INTERNAL);
    assert!(extraction_success.stdout.is_none());
}
