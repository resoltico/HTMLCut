use super::*;

#[test]
fn prepared_request_file_builders_load_selector_and_slice_definitions() {
    let fixture = request_file_fixture();

    let prepared_slice = PreparedExtraction::from_slice_with_logging(
        SliceArgs {
            definition: DefinitionArgs {
                request_file: Some(fixture.slice_definition_path.clone()),
                emit_request_file: None,
            },
            source: SourceArgs {
                input: None,
                input_html: None,
                base_url: None,
                max_bytes: DEFAULT_MAX_BYTES.to_string(),
                fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
                fetch_connect_timeout_ms: htmlcut_core::DEFAULT_FETCH_CONNECT_TIMEOUT_MS,
                tls_trust: CliTlsTrustMode::WebPki,
                tls_ca_bundle: None,
                fetch_preflight: CliFetchPreflightMode::HeadFirst,
            },
            from: None,
            to: None,
            pattern: CliPatternMode::Literal,
            regex_flags: None,
            boundary_retention: crate::args::CliBoundaryRetentionMode::ExcludeBoth,
            selection: SelectionArgs {
                r#match: CliMatchMode::First,
                index: None,
            },
            output: SliceExtractOutputArgs {
                value: CliSliceValueMode::Text,
                attribute: None,
                whitespace: CliWhitespaceMode::Rendered,
                rewrite_urls: false,
                output: None,
                bundle: None,
                output_file: Some(fixture.request_file_output_path.clone()),
                preview_chars: DEFAULT_PREVIEW_CHARS,
                include_source_text: false,
            },
            file_write: default_file_write_args(),
        },
        0,
        false,
    )
    .expect("slice request file");
    assert_eq!(
        prepared_slice.request.extraction.strategy(),
        ExtractionStrategy::Slice
    );
    assert_eq!(
        prepared_slice.output_file.as_deref(),
        Some(fixture.request_file_output_path.as_path())
    );
    assert!(prepared_slice.request_definition_output.is_none());

    let preview_select = PreparedPreview::from_select(InspectSelectArgs {
        definition: DefinitionArgs {
            request_file: Some(fixture.selector_definition_path.clone()),
            emit_request_file: None,
        },
        source: SourceArgs {
            input: None,
            input_html: None,
            base_url: None,
            max_bytes: DEFAULT_MAX_BYTES.to_string(),
            fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
            fetch_connect_timeout_ms: htmlcut_core::DEFAULT_FETCH_CONNECT_TIMEOUT_MS,
            tls_trust: CliTlsTrustMode::WebPki,
            tls_ca_bundle: None,
            fetch_preflight: CliFetchPreflightMode::HeadFirst,
        },
        css: None,
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
    .expect("inspect select request file");
    assert_eq!(preview_select.request.extraction.value(), &ValueSpec::Text);
    assert!(preview_select.request_definition_output.is_none());

    let preview_slice = PreparedPreview::from_slice(InspectSliceArgs {
        definition: DefinitionArgs {
            request_file: Some(fixture.slice_definition_path.clone()),
            emit_request_file: None,
        },
        source: SourceArgs {
            input: None,
            input_html: None,
            base_url: None,
            max_bytes: DEFAULT_MAX_BYTES.to_string(),
            fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
            fetch_connect_timeout_ms: htmlcut_core::DEFAULT_FETCH_CONNECT_TIMEOUT_MS,
            tls_trust: CliTlsTrustMode::WebPki,
            tls_ca_bundle: None,
            fetch_preflight: CliFetchPreflightMode::HeadFirst,
        },
        from: None,
        to: None,
        pattern: CliPatternMode::Literal,
        regex_flags: None,
        boundary_retention: crate::args::CliBoundaryRetentionMode::ExcludeBoth,
        selection: SelectionArgs {
            r#match: CliMatchMode::First,
            index: None,
        },
        value: CliSliceValueMode::Structured,
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
    .expect("inspect slice request file");
    assert_eq!(preview_slice.request.extraction.value(), &ValueSpec::Text);
    assert!(preview_slice.request_definition_output.is_none());
}

#[test]
fn prepared_request_file_builders_report_cli_conflicts() {
    let fixture = request_file_fixture();

    let slice_conflict = expect_cli_error(
        PreparedExtraction::from_slice_with_logging(
            SliceArgs {
                definition: DefinitionArgs {
                    request_file: Some(fixture.slice_definition_path.clone()),
                    emit_request_file: None,
                },
                source: SourceArgs {
                    input: Some(fixture.input.clone()),
                    input_html: None,
                    base_url: Some("https://example.com/base/".to_owned()),
                    max_bytes: DEFAULT_MAX_BYTES.to_string(),
                    fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
                    fetch_connect_timeout_ms: htmlcut_core::DEFAULT_FETCH_CONNECT_TIMEOUT_MS,
                    tls_trust: CliTlsTrustMode::WebPki,
                    tls_ca_bundle: None,
                    fetch_preflight: CliFetchPreflightMode::HeadFirst,
                },
                from: Some("<article>".to_owned()),
                to: Some("</article>".to_owned()),
                pattern: CliPatternMode::Regex,
                regex_flags: Some("i".to_owned()),
                boundary_retention: crate::args::CliBoundaryRetentionMode::IncludeBoth,
                selection: SelectionArgs {
                    r#match: CliMatchMode::Nth,
                    index: Some(2),
                },
                output: SliceExtractOutputArgs {
                    value: CliSliceValueMode::Structured,
                    attribute: None,
                    whitespace: CliWhitespaceMode::Normalize,
                    rewrite_urls: true,
                    output: Some(CliOutputMode::Json),
                    bundle: Some(fixture.tempdir.path().join("bundle")),
                    output_file: Some(fixture.tempdir.path().join("stdout.json")),
                    preview_chars: DEFAULT_PREVIEW_CHARS + 1,
                    include_source_text: true,
                },
                file_write: default_file_write_args(),
            },
            0,
            false,
        ),
        "slice request file conflict",
    );
    assert_eq!(slice_conflict.code, "CLI_REQUEST_FILE_CONFLICT");
    assert!(slice_conflict.message.contains("--regex-flags"));
    assert!(
        slice_conflict
            .message
            .contains("--emit-request-file <PATH>")
    );
    assert!(!slice_conflict.message.contains("--output-file"));

    let inspect_select_conflict = expect_cli_error(
        PreparedPreview::from_select(InspectSelectArgs {
            definition: DefinitionArgs {
                request_file: Some(fixture.selector_definition_path),
                emit_request_file: None,
            },
            source: SourceArgs {
                input: Some(fixture.input.clone()),
                input_html: None,
                base_url: Some("https://example.com/base/".to_owned()),
                max_bytes: DEFAULT_MAX_BYTES.to_string(),
                fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
                fetch_connect_timeout_ms: htmlcut_core::DEFAULT_FETCH_CONNECT_TIMEOUT_MS,
                tls_trust: CliTlsTrustMode::WebPki,
                tls_ca_bundle: None,
                fetch_preflight: CliFetchPreflightMode::HeadFirst,
            },
            css: Some("article".to_owned()),
            selection: SelectionArgs {
                r#match: CliMatchMode::Nth,
                index: Some(2),
            },
            value: CliValueMode::Structured,
            attribute: None,
            whitespace: CliWhitespaceMode::Normalize,
            rewrite_urls: true,
            output: InspectOutputArgs {
                output: CliInspectOutputMode::Text,
                preview_chars: DEFAULT_PREVIEW_CHARS + 1,
                include_source_text: true,
                output_file: None,
            },
            file_write: default_preview_file_write_args(),
        }),
        "inspect select request file conflict",
    );
    assert_eq!(inspect_select_conflict.code, "CLI_REQUEST_FILE_CONFLICT");
    assert!(inspect_select_conflict.message.contains("--whitespace"));
    assert!(inspect_select_conflict.message.contains("--preview-chars"));

    let inspect_slice_conflict = expect_cli_error(
        PreparedPreview::from_slice(InspectSliceArgs {
            definition: DefinitionArgs {
                request_file: Some(fixture.slice_definition_path),
                emit_request_file: None,
            },
            source: SourceArgs {
                input: Some(fixture.input),
                input_html: None,
                base_url: Some("https://example.com/base/".to_owned()),
                max_bytes: DEFAULT_MAX_BYTES.to_string(),
                fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
                fetch_connect_timeout_ms: htmlcut_core::DEFAULT_FETCH_CONNECT_TIMEOUT_MS,
                tls_trust: CliTlsTrustMode::WebPki,
                tls_ca_bundle: None,
                fetch_preflight: CliFetchPreflightMode::HeadFirst,
            },
            from: Some("<article>".to_owned()),
            to: Some("</article>".to_owned()),
            pattern: CliPatternMode::Regex,
            regex_flags: Some("i".to_owned()),
            boundary_retention: crate::args::CliBoundaryRetentionMode::IncludeBoth,
            selection: SelectionArgs {
                r#match: CliMatchMode::Nth,
                index: Some(2),
            },
            value: CliSliceValueMode::Structured,
            attribute: None,
            whitespace: CliWhitespaceMode::Normalize,
            rewrite_urls: true,
            output: InspectOutputArgs {
                output: CliInspectOutputMode::Text,
                preview_chars: DEFAULT_PREVIEW_CHARS + 1,
                include_source_text: true,
                output_file: None,
            },
            file_write: default_preview_file_write_args(),
        }),
        "inspect slice request file conflict",
    );
    assert_eq!(inspect_slice_conflict.code, "CLI_REQUEST_FILE_CONFLICT");
    assert!(
        inspect_slice_conflict
            .message
            .contains("--boundary-retention")
    );
    assert!(
        inspect_slice_conflict
            .message
            .contains("--include-source-text")
    );
}
