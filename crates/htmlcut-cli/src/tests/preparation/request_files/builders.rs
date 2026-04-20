use super::*;

#[test]
fn request_file_builders_and_output_file_edges_cover_remaining_branches() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture_file(tempdir.path(), "input.html", "<article>Hello</article>");
    let input = input_path.to_string_lossy().into_owned();

    let selector_definition = ExtractionDefinition::new(ExtractionRequest::new(
        SourceRequest::file(&input_path),
        ExtractionSpec::selector(SelectorQuery::new("article").expect("selector"))
            .with_selection(SelectionSpec::single())
            .with_value(ValueSpec::Text),
    ));
    let selector_definition_path = write_definition_file(
        tempdir.path(),
        "selector-request.json",
        &selector_definition,
    );

    let slice_definition = ExtractionDefinition::new(ExtractionRequest::new(
        SourceRequest::file(&input_path),
        ExtractionSpec::slice(
            htmlcut_core::SliceSpec::new(
                htmlcut_core::SliceBoundary::new("<article>").expect("slice boundary"),
                htmlcut_core::SliceBoundary::new("</article>").expect("slice boundary"),
            )
            .with_boundary_inclusion(true, true),
        )
        .with_selection(SelectionSpec::single())
        .with_value(ValueSpec::Text),
    ));
    let slice_definition_path =
        write_definition_file(tempdir.path(), "slice-request.json", &slice_definition);
    let request_file_output_path = tempdir.path().join("request-file-output.json");

    let get_only_runtime = build_runtime(&SourceArgs {
        input: Some(input.clone()),
        base_url: None,
        max_bytes: DEFAULT_MAX_BYTES.to_string(),
        fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
        fetch_preflight: CliFetchPreflightMode::GetOnly,
    })
    .expect("runtime");
    assert_eq!(
        get_only_runtime.fetch_preflight,
        FetchPreflightMode::GetOnly
    );

    assert_eq!(
        build_source_request(&SourceArgs {
            input: None,
            base_url: None,
            max_bytes: DEFAULT_MAX_BYTES.to_string(),
            fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
            fetch_preflight: CliFetchPreflightMode::HeadFirst,
        })
        .expect_err("missing input")
        .code,
        "CLI_REQUIRED_PARAMETER_MISSING"
    );

    assert_eq!(
        expect_cli_error(
            load_extraction_definition_for_tests(
                &tempdir.path().join("missing-request.json"),
                ExtractionStrategy::Selector,
                "select",
            ),
            "missing request file",
        )
        .code,
        "CLI_REQUEST_FILE_READ_FAILED"
    );

    let invalid_json_path = write_fixture_file(tempdir.path(), "invalid-request.json", "{not json");
    let invalid_json_error = expect_cli_error(
        load_extraction_definition_for_tests(
            &invalid_json_path,
            ExtractionStrategy::Selector,
            "select",
        ),
        "invalid request file json",
    );
    assert_eq!(invalid_json_error.code, "CLI_REQUEST_FILE_INVALID");
    assert!(
        invalid_json_error
            .message
            .contains("htmlcut schema --name htmlcut.extraction_definition --output json")
    );
    assert!(
        invalid_json_error
            .message
            .contains("htmlcut catalog --operation select.extract --output json")
    );

    let invalid_shape_path = write_fixture_file(
        tempdir.path(),
        "invalid-shape.json",
        r#"{
  "schema_name": "htmlcut.extraction_definition",
  "schema_version": 1,
  "request": {
    "source": { "input": { "type": "stdin" } },
    "extraction": {
      "kind": "selector",
      "selector": { "css": "article" }
    }
  }
}"#,
    );
    let invalid_shape_error = expect_cli_error(
        load_extraction_definition_for_tests(
            &invalid_shape_path,
            ExtractionStrategy::Selector,
            "select",
        ),
        "invalid request file shape",
    );
    assert_eq!(invalid_shape_error.code, "CLI_REQUEST_FILE_INVALID");
    assert!(invalid_shape_error.message.contains("JSON path $"));
    assert!(invalid_shape_error.message.contains("selector"));
    assert!(
        invalid_shape_error
            .message
            .contains("request.extraction.selector` as a plain JSON string")
    );

    let mut unsupported_schema =
        serde_json::to_value(&selector_definition).expect("definition json");
    unsupported_schema["schema_name"] = Value::String("synthetic.request".to_owned());
    unsupported_schema["schema_version"] = Value::from(99);
    let unsupported_schema_path = tempdir.path().join("unsupported-schema.json");
    fs::write(
        &unsupported_schema_path,
        serde_json::to_string_pretty(&unsupported_schema).expect("serialize unsupported schema"),
    )
    .expect("write unsupported schema");
    assert_eq!(
        expect_cli_error(
            load_extraction_definition_for_tests(
                &unsupported_schema_path,
                ExtractionStrategy::Selector,
                "select",
            ),
            "unsupported schema",
        )
        .code,
        "CLI_REQUEST_FILE_SCHEMA_UNSUPPORTED"
    );

    let mut unsupported_version =
        serde_json::to_value(&selector_definition).expect("definition json");
    unsupported_version["schema_version"] = Value::from(99);
    let unsupported_version_path = tempdir.path().join("unsupported-version.json");
    fs::write(
        &unsupported_version_path,
        serde_json::to_string_pretty(&unsupported_version).expect("serialize unsupported version"),
    )
    .expect("write unsupported version");
    assert_eq!(
        expect_cli_error(
            load_extraction_definition_for_tests(
                &unsupported_version_path,
                ExtractionStrategy::Selector,
                "select",
            ),
            "unsupported version",
        )
        .code,
        "CLI_REQUEST_FILE_SCHEMA_UNSUPPORTED"
    );

    assert_eq!(
        expect_cli_error(
            load_extraction_definition_for_tests(
                &selector_definition_path,
                ExtractionStrategy::Slice,
                "slice",
            ),
            "strategy mismatch",
        )
        .code,
        "CLI_REQUEST_FILE_STRATEGY_MISMATCH"
    );
    assert_eq!(
        expect_cli_error(
            load_extraction_definition_for_tests(
                &slice_definition_path,
                ExtractionStrategy::Selector,
                "select",
            ),
            "slice strategy mismatch",
        )
        .code,
        "CLI_REQUEST_FILE_STRATEGY_MISMATCH"
    );

    let prepared_slice = PreparedExtraction::from_slice_with_logging(
        SliceArgs {
            definition: DefinitionArgs {
                request_file: Some(slice_definition_path.clone()),
                emit_request_file: None,
            },
            source: SourceArgs {
                input: None,
                base_url: None,
                max_bytes: DEFAULT_MAX_BYTES.to_string(),
                fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
                fetch_preflight: CliFetchPreflightMode::HeadFirst,
            },
            from: None,
            to: None,
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
                output: None,
                bundle: None,
                output_file: Some(request_file_output_path.clone()),
                preview_chars: DEFAULT_PREVIEW_CHARS,
                include_source_text: false,
            },
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
        Some(request_file_output_path.as_path())
    );
    assert!(prepared_slice.request_definition_output.is_none());

    let preview_select = PreparedPreview::from_select(InspectSelectArgs {
        definition: DefinitionArgs {
            request_file: Some(selector_definition_path.clone()),
            emit_request_file: None,
        },
        source: SourceArgs {
            input: None,
            base_url: None,
            max_bytes: DEFAULT_MAX_BYTES.to_string(),
            fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
            fetch_preflight: CliFetchPreflightMode::HeadFirst,
        },
        css: None,
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
    .expect("inspect select request file");
    assert_eq!(
        preview_select.request.extraction.value(),
        &ValueSpec::Structured
    );
    assert!(preview_select.request_definition_output.is_none());

    let preview_slice = PreparedPreview::from_slice(InspectSliceArgs {
        definition: DefinitionArgs {
            request_file: Some(slice_definition_path.clone()),
            emit_request_file: None,
        },
        source: SourceArgs {
            input: None,
            base_url: None,
            max_bytes: DEFAULT_MAX_BYTES.to_string(),
            fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
            fetch_preflight: CliFetchPreflightMode::HeadFirst,
        },
        from: None,
        to: None,
        pattern: CliPatternMode::Literal,
        regex_flags: None,
        include_start: false,
        include_end: false,
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
    .expect("inspect slice request file");
    assert_eq!(
        preview_slice.request.extraction.value(),
        &ValueSpec::Structured
    );
    assert!(preview_slice.request_definition_output.is_none());

    let slice_conflict = expect_cli_error(
        PreparedExtraction::from_slice_with_logging(
            SliceArgs {
                definition: DefinitionArgs {
                    request_file: Some(slice_definition_path.clone()),
                    emit_request_file: None,
                },
                source: SourceArgs {
                    input: Some(input.clone()),
                    base_url: Some("https://example.com/base/".to_owned()),
                    max_bytes: DEFAULT_MAX_BYTES.to_string(),
                    fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
                    fetch_preflight: CliFetchPreflightMode::HeadFirst,
                },
                from: Some("<article>".to_owned()),
                to: Some("</article>".to_owned()),
                pattern: CliPatternMode::Regex,
                regex_flags: Some("u".to_owned()),
                include_start: true,
                include_end: true,
                selection: SelectionArgs {
                    r#match: CliMatchMode::Nth,
                    index: Some(2),
                },
                output: ExtractOutputArgs {
                    value: CliValueMode::Structured,
                    attribute: None,
                    whitespace: CliWhitespaceMode::Normalize,
                    rewrite_urls: true,
                    output: Some(CliOutputMode::Json),
                    bundle: Some(tempdir.path().join("bundle")),
                    output_file: Some(tempdir.path().join("stdout.json")),
                    preview_chars: DEFAULT_PREVIEW_CHARS + 1,
                    include_source_text: true,
                },
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
                request_file: Some(selector_definition_path.clone()),
                emit_request_file: None,
            },
            source: SourceArgs {
                input: Some(input.clone()),
                base_url: Some("https://example.com/base/".to_owned()),
                max_bytes: DEFAULT_MAX_BYTES.to_string(),
                fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
                fetch_preflight: CliFetchPreflightMode::HeadFirst,
            },
            css: Some("article".to_owned()),
            selection: SelectionArgs {
                r#match: CliMatchMode::Nth,
                index: Some(2),
            },
            whitespace: CliWhitespaceMode::Normalize,
            rewrite_urls: true,
            output: InspectOutputArgs {
                output: CliInspectOutputMode::Text,
                preview_chars: DEFAULT_PREVIEW_CHARS + 1,
                include_source_text: true,
                output_file: None,
            },
        }),
        "inspect select request file conflict",
    );
    assert_eq!(inspect_select_conflict.code, "CLI_REQUEST_FILE_CONFLICT");
    assert!(inspect_select_conflict.message.contains("--whitespace"));
    assert!(inspect_select_conflict.message.contains("--preview-chars"));

    let inspect_slice_conflict = expect_cli_error(
        PreparedPreview::from_slice(InspectSliceArgs {
            definition: DefinitionArgs {
                request_file: Some(slice_definition_path),
                emit_request_file: None,
            },
            source: SourceArgs {
                input: Some(input),
                base_url: Some("https://example.com/base/".to_owned()),
                max_bytes: DEFAULT_MAX_BYTES.to_string(),
                fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
                fetch_preflight: CliFetchPreflightMode::HeadFirst,
            },
            from: Some("<article>".to_owned()),
            to: Some("</article>".to_owned()),
            pattern: CliPatternMode::Regex,
            regex_flags: Some("u".to_owned()),
            include_start: true,
            include_end: true,
            selection: SelectionArgs {
                r#match: CliMatchMode::Nth,
                index: Some(2),
            },
            whitespace: CliWhitespaceMode::Normalize,
            rewrite_urls: true,
            output: InspectOutputArgs {
                output: CliInspectOutputMode::Text,
                preview_chars: DEFAULT_PREVIEW_CHARS + 1,
                include_source_text: true,
                output_file: None,
            },
        }),
        "inspect slice request file conflict",
    );
    assert_eq!(inspect_slice_conflict.code, "CLI_REQUEST_FILE_CONFLICT");
    assert!(inspect_slice_conflict.message.contains("--include-start"));
    assert!(
        inspect_slice_conflict
            .message
            .contains("--include-source-text")
    );

    assert_eq!(
        resolve_extract_output_mode_with_output_file(
            Some(CliOutputMode::None),
            &ValueType::Text,
            Some(tempdir.path()),
            Some(&tempdir.path().join("selection.txt")),
        )
        .expect_err("output file requires stdout payload")
        .code,
        "CLI_OUTPUT_FILE_REQUIRES_STDOUT_PAYLOAD"
    );

    let nested_output = tempdir.path().join("nested/output/selection.txt");
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit_code = write_outcome(
        ExecutionOutcome {
            stdout: Some("Hello".to_owned()),
            output_file: Some(nested_output.clone()),
            post_write_stderr: Vec::new(),
            stderr: Vec::new(),
            exit_code: 0,
        },
        &mut stdout,
        &mut stderr,
    );
    assert_eq!(exit_code, 0);
    assert!(stdout.is_empty());
    assert!(stderr.is_empty());
    assert_eq!(
        fs::read_to_string(&nested_output).expect("nested output file"),
        "Hello\n"
    );
    let ordered_output = tempdir.path().join("ordered/output/report.txt");
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit_code = write_outcome(
        ExecutionOutcome {
            stdout: Some("Hello".to_owned()),
            output_file: Some(ordered_output.clone()),
            post_write_stderr: vec![
                "htmlcut: wrote output file to ordered/output/report.txt".to_owned(),
            ],
            stderr: vec![
                "htmlcut: request normalized".to_owned(),
                "htmlcut: preview complete".to_owned(),
            ],
            exit_code: 0,
        },
        &mut stdout,
        &mut stderr,
    );
    assert_eq!(exit_code, 0);
    assert_eq!(
        String::from_utf8(stderr).expect("stderr"),
        "htmlcut: request normalized\nhtmlcut: preview complete\nhtmlcut: wrote output file to ordered/output/report.txt\n"
    );
    assert_eq!(
        fs::read_to_string(&ordered_output).expect("ordered output file"),
        "Hello\n"
    );

    let direct_nested_output = tempdir.path().join("direct/output/report.txt");
    write_stdout_payload_for_tests(&direct_nested_output, "Hello")
        .expect("write stdout payload with nested parent");
    assert_eq!(
        fs::read_to_string(&direct_nested_output).expect("direct nested output file"),
        "Hello\n"
    );
    let relative_output =
        PathBuf::from(format!(".htmlcut-write-payload-{}.txt", std::process::id()));
    write_stdout_payload_for_tests(&relative_output, "Hello")
        .expect("write stdout payload without parent directory");
    assert_eq!(
        fs::read_to_string(&relative_output).expect("relative output file"),
        "Hello\n"
    );
    fs::remove_file(&relative_output).expect("remove relative output file");
    assert!(
        write_stdout_payload_for_tests(Path::new("/"), "Hello")
            .expect_err("root write should fail")
            .kind()
            != std::io::ErrorKind::NotFound
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit_code = write_outcome(
        ExecutionOutcome {
            stdout: Some("Hello".to_owned()),
            output_file: Some(tempdir.path().to_path_buf()),
            post_write_stderr: Vec::new(),
            stderr: Vec::new(),
            exit_code: 0,
        },
        &mut stdout,
        &mut stderr,
    );
    assert_eq!(exit_code, EXIT_CODE_OUTPUT);
    assert!(stdout.is_empty());
    assert!(
        String::from_utf8(stderr)
            .expect("stderr")
            .contains("Could not write")
    );
}
