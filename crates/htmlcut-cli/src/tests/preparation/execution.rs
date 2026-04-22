use super::*;

#[test]
fn execution_paths_cover_direct_success_and_failure_variants() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture_file(
        tempdir.path(),
        "input.html",
        "<article><p>Hello</p></article>",
    );
    let input = input_path.to_string_lossy().into_owned();

    assert_eq!(
        validate_base_url(Some("https://example.com/docs"))
            .expect("valid base url")
            .as_ref()
            .map(|url| url.as_str()),
        Some("https://example.com/docs")
    );
    assert_eq!(
        validate_base_url(Some("http://example.com/docs"))
            .expect("valid http base url")
            .as_ref()
            .map(|url| url.as_str()),
        Some("http://example.com/docs")
    );
    assert_eq!(parse_byte_size("512").expect("plain bytes"), 512);
    assert!(parse_byte_size(&"9".repeat(400)).is_err());
    let fractional_byte = parse_byte_size("0.5b").expect_err("fractional bytes should fail");
    assert_eq!(fractional_byte.code, "CLI_BYTE_SIZE_INVALID");
    assert!(fractional_byte.message.contains("Invalid byte size"));
    let too_large = parse_byte_size("18446744073709551615kb").expect_err("too large byte size");
    assert_eq!(too_large.code, "CLI_BYTE_SIZE_INVALID");
    assert!(too_large.message.contains("Byte size is too large"));
    assert!(!raw_args_prefers_json(&[
        "htmlcut".to_owned(),
        "select".to_owned(),
        "page.html".to_owned(),
        "--output=html".to_owned(),
    ]));
    assert!(!raw_args_prefers_json(&[
        "htmlcut".to_owned(),
        "select".to_owned(),
        "page.html".to_owned(),
        "--value".to_owned(),
        "text".to_owned(),
    ]));
    assert!(!raw_args_prefers_json(&[
        "htmlcut".to_owned(),
        "select".to_owned(),
        "page.html".to_owned(),
        "--output".to_owned(),
    ]));

    let missing = tempdir
        .path()
        .join("missing.html")
        .to_string_lossy()
        .into_owned();
    let inspect_text = run_inspect_source(
        InspectSourceArgs {
            source: SourceArgs {
                input: Some(missing.clone()),
                base_url: None,
                max_bytes: DEFAULT_MAX_BYTES.to_string(),
                fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
                fetch_preflight: CliFetchPreflightMode::HeadFirst,
            },
            output: CliInspectOutputMode::Text,
            include_source_text: false,
            output_file: None,
            sample_limit: DEFAULT_INSPECTION_SAMPLE_LIMIT,
            preview_chars: DEFAULT_PREVIEW_CHARS,
        },
        0,
        false,
    );
    assert_eq!(inspect_text.exit_code, EXIT_CODE_SOURCE);
    assert!(inspect_text.stdout.is_none());
    assert!(inspect_text.stderr[0].contains("Could not access file"));

    let inspect_json = run_inspect_source(
        InspectSourceArgs {
            source: SourceArgs {
                input: Some(missing),
                base_url: None,
                max_bytes: DEFAULT_MAX_BYTES.to_string(),
                fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
                fetch_preflight: CliFetchPreflightMode::HeadFirst,
            },
            output: CliInspectOutputMode::Json,
            include_source_text: false,
            output_file: None,
            sample_limit: DEFAULT_INSPECTION_SAMPLE_LIMIT,
            preview_chars: DEFAULT_PREVIEW_CHARS,
        },
        0,
        false,
    );
    assert_eq!(inspect_json.exit_code, EXIT_CODE_SOURCE);
    assert!(
        inspect_json
            .stdout
            .as_deref()
            .is_some_and(|stdout| stdout.contains("\"command\": \"inspect-source\""))
    );

    let preview_text = execute_preview(
        PreparedPreview::from_select(InspectSelectArgs {
            definition: DefinitionArgs {
                request_file: None,
                emit_request_file: None,
            },
            source: SourceArgs {
                input: Some(input.clone()),
                base_url: None,
                max_bytes: DEFAULT_MAX_BYTES.to_string(),
                fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
                fetch_preflight: CliFetchPreflightMode::HeadFirst,
            },
            css: Some("[".to_owned()),
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
        })
        .expect("preview builder"),
    );
    assert_eq!(preview_text.exit_code, EXIT_CODE_USAGE);
    assert!(preview_text.stdout.is_none());
    assert!(preview_text.stderr[0].contains("Invalid selector"));

    let preview_json = execute_preview(
        PreparedPreview::from_select(InspectSelectArgs {
            definition: DefinitionArgs {
                request_file: None,
                emit_request_file: None,
            },
            source: SourceArgs {
                input: Some(input.clone()),
                base_url: None,
                max_bytes: DEFAULT_MAX_BYTES.to_string(),
                fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
                fetch_preflight: CliFetchPreflightMode::HeadFirst,
            },
            css: Some("[".to_owned()),
            selection: SelectionArgs {
                r#match: CliMatchMode::First,
                index: None,
            },
            whitespace: CliWhitespaceMode::Preserve,
            rewrite_urls: false,
            output: InspectOutputArgs {
                output: CliInspectOutputMode::Json,
                preview_chars: DEFAULT_PREVIEW_CHARS,
                include_source_text: false,
                output_file: None,
            },
        })
        .expect("preview builder"),
    );
    assert_eq!(preview_json.exit_code, EXIT_CODE_USAGE);
    assert!(
        preview_json
            .stdout
            .as_deref()
            .is_some_and(|stdout| stdout.contains("\"ok\": false"))
    );

    let preview_success_json = execute_preview(
        PreparedPreview::from_select(InspectSelectArgs {
            definition: DefinitionArgs {
                request_file: None,
                emit_request_file: None,
            },
            source: SourceArgs {
                input: Some(input.clone()),
                base_url: None,
                max_bytes: DEFAULT_MAX_BYTES.to_string(),
                fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
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
                output: CliInspectOutputMode::Json,
                preview_chars: DEFAULT_PREVIEW_CHARS,
                include_source_text: false,
                output_file: None,
            },
        })
        .expect("preview builder"),
    );
    assert_eq!(preview_success_json.exit_code, 0);
    assert!(
        preview_success_json
            .stdout
            .as_deref()
            .is_some_and(|stdout| stdout.contains("\"command\": \"inspect-select\""))
    );

    let extract_text = execute_extraction(
        PreparedExtraction::from_select(SelectArgs {
            definition: DefinitionArgs {
                request_file: None,
                emit_request_file: None,
            },
            source: SourceArgs {
                input: Some(input.clone()),
                base_url: None,
                max_bytes: DEFAULT_MAX_BYTES.to_string(),
                fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
                fetch_preflight: CliFetchPreflightMode::HeadFirst,
            },
            css: Some("[".to_owned()),
            selection: SelectionArgs {
                r#match: CliMatchMode::First,
                index: None,
            },
            output: ExtractOutputArgs {
                value: CliValueMode::Text,
                attribute: None,
                whitespace: CliWhitespaceMode::Preserve,
                rewrite_urls: false,
                output: Some(CliOutputMode::Text),
                bundle: None,
                preview_chars: DEFAULT_PREVIEW_CHARS,
                include_source_text: false,
                output_file: None,
            },
        })
        .expect("extract builder"),
    );
    assert_eq!(extract_text.exit_code, EXIT_CODE_USAGE);
    assert!(extract_text.stdout.is_none());
    assert!(extract_text.stderr[0].contains("Invalid selector"));

    let bundle_dir = tempdir.path().join("bundle out");
    let extract_success = execute_extraction(
        PreparedExtraction::from_select_with_logging(
            SelectArgs {
                definition: DefinitionArgs {
                    request_file: None,
                    emit_request_file: None,
                },
                source: SourceArgs {
                    input: Some(input.clone()),
                    base_url: None,
                    max_bytes: DEFAULT_MAX_BYTES.to_string(),
                    fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
                    fetch_preflight: CliFetchPreflightMode::HeadFirst,
                },
                css: Some("article".to_owned()),
                selection: SelectionArgs {
                    r#match: CliMatchMode::First,
                    index: None,
                },
                output: ExtractOutputArgs {
                    value: CliValueMode::Text,
                    attribute: None,
                    whitespace: CliWhitespaceMode::Preserve,
                    rewrite_urls: false,
                    output: Some(CliOutputMode::Text),
                    bundle: Some(bundle_dir.clone()),
                    preview_chars: DEFAULT_PREVIEW_CHARS,
                    include_source_text: false,
                    output_file: None,
                },
            },
            1,
            false,
        )
        .expect("extract builder"),
    );
    assert_eq!(extract_success.exit_code, 0);
    assert!(
        extract_success
            .stderr
            .iter()
            .any(|line| line.contains("wrote bundle to"))
    );
    assert!(bundle_dir.join("report.json").exists());

    let extract_success_no_bundle = execute_extraction(
        PreparedExtraction::from_select(SelectArgs {
            definition: DefinitionArgs {
                request_file: None,
                emit_request_file: None,
            },
            source: SourceArgs {
                input: Some(input.clone()),
                base_url: None,
                max_bytes: DEFAULT_MAX_BYTES.to_string(),
                fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
                fetch_preflight: CliFetchPreflightMode::HeadFirst,
            },
            css: Some("article".to_owned()),
            selection: SelectionArgs {
                r#match: CliMatchMode::First,
                index: None,
            },
            output: ExtractOutputArgs {
                value: CliValueMode::Text,
                attribute: None,
                whitespace: CliWhitespaceMode::Preserve,
                rewrite_urls: false,
                output: Some(CliOutputMode::Text),
                bundle: None,
                preview_chars: DEFAULT_PREVIEW_CHARS,
                include_source_text: false,
                output_file: None,
            },
        })
        .expect("extract builder"),
    );
    assert_eq!(extract_success_no_bundle.exit_code, 0);
    assert!(extract_success_no_bundle.stderr.is_empty());

    let extract_success_verbose_no_bundle = execute_extraction(
        PreparedExtraction::from_select_with_logging(
            SelectArgs {
                definition: DefinitionArgs {
                    request_file: None,
                    emit_request_file: None,
                },
                source: SourceArgs {
                    input: Some(input.clone()),
                    base_url: None,
                    max_bytes: DEFAULT_MAX_BYTES.to_string(),
                    fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
                    fetch_preflight: CliFetchPreflightMode::HeadFirst,
                },
                css: Some("article".to_owned()),
                selection: SelectionArgs {
                    r#match: CliMatchMode::First,
                    index: None,
                },
                output: ExtractOutputArgs {
                    value: CliValueMode::Text,
                    attribute: None,
                    whitespace: CliWhitespaceMode::Preserve,
                    rewrite_urls: false,
                    output: Some(CliOutputMode::Text),
                    bundle: None,
                    preview_chars: DEFAULT_PREVIEW_CHARS,
                    include_source_text: false,
                    output_file: None,
                },
            },
            1,
            false,
        )
        .expect("extract builder"),
    );
    assert_eq!(extract_success_verbose_no_bundle.exit_code, 0);
    assert_eq!(extract_success_verbose_no_bundle.stderr.len(), 1);

    assert!(
        PreparedExtraction::from_slice(SliceArgs {
            definition: DefinitionArgs {
                request_file: None,
                emit_request_file: None
            },
            source: SourceArgs {
                input: Some(input.clone()),
                base_url: None,
                max_bytes: DEFAULT_MAX_BYTES.to_string(),
                fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
                fetch_preflight: CliFetchPreflightMode::HeadFirst,
            },
            from: Some("<article>".to_owned()),
            to: Some("</article>".to_owned()),
            pattern: CliPatternMode::Literal,
            regex_flags: None,
            include_start: false,
            include_end: false,
            selection: SelectionArgs {
                r#match: CliMatchMode::First,
                index: None,
            },
            output: ExtractOutputArgs {
                value: CliValueMode::Text,
                attribute: None,
                whitespace: CliWhitespaceMode::Preserve,
                rewrite_urls: false,
                output: Some(CliOutputMode::None),
                bundle: None,
                preview_chars: DEFAULT_PREVIEW_CHARS,
                include_source_text: false,
                output_file: None,
            },
        })
        .is_err()
    );

    let one_structured_match_report = ExtractionCommandReport {
        tool: TOOL_NAME.to_owned(),
        engine: ENGINE_NAME.to_owned(),
        version: HTMLCUT_VERSION.to_owned(),
        schema_name: EXTRACTION_COMMAND_REPORT_SCHEMA_NAME.to_owned(),
        schema_version: EXTRACTION_COMMAND_REPORT_SCHEMA_VERSION,
        command: "select".to_owned(),
        operation_id: htmlcut_core::OperationId::SelectExtract,
        ok: true,
        source: SourceMetadata {
            kind: SourceKind::File,
            value: "/tmp/input.html".to_owned(),
            input_base_url: None,
            effective_base_url: None,
            bytes_read: 10,
            load_steps: Vec::new(),
            text: None,
        },
        extraction: ExtractionSpec::selector(SelectorQuery::new("article").expect("selector"))
            .with_selection(SelectionSpec::default())
            .with_value(ValueSpec::Structured),
        stats: ExtractionStats {
            duration_ms: 1,
            candidate_count: 1,
            match_count: 1,
        },
        document_title: None,
        matches: vec![ExtractionMatch {
            index: 1,
            path: None,
            value_type: ValueType::Structured,
            value: serde_json::json!({"hello":"world"}),
            html: None,
            text: None,
            preview: "preview".to_owned(),
            metadata: selector_metadata(1, 1, "article:nth-of-type(1)", "article", &[]),
        }],
        diagnostics: Vec::new(),
        bundle: None,
    };
    assert!(wrap_html_document(&one_structured_match_report).contains("<pre>"));

    let mut multi_match_report = build_extraction_report(
        "select",
        fixture_result(
            Value::String("<p>Hello</p>".to_owned()),
            ValueType::InnerHtml,
        ),
        None,
    );
    multi_match_report.matches.push(ExtractionMatch {
        index: 2,
        path: Some("article:nth-of-type(2)".to_owned()),
        value_type: ValueType::OuterHtml,
        value: Value::String("<article>World</article>".to_owned()),
        html: Some("<article>World</article>".to_owned()),
        text: Some("World".to_owned()),
        preview: "World".to_owned(),
        metadata: selector_metadata(2, 2, "article:nth-of-type(2)", "article", &[]),
    });
    assert!(wrap_html_document(&multi_match_report).contains("data-match-index=\"2\""));

    let outer_html_match = ExtractionMatch {
        index: 1,
        path: None,
        value_type: ValueType::OuterHtml,
        value: Value::String("<article>Hello</article>".to_owned()),
        html: Some("<article>Hello</article>".to_owned()),
        text: Some("Hello".to_owned()),
        preview: "Hello".to_owned(),
        metadata: selector_metadata(1, 1, "article:nth-of-type(1)", "article", &[]),
    };
    assert_eq!(
        render_match_as_html(&outer_html_match),
        "<article>Hello</article>"
    );
}
