pub(super) use super::*;

#[test]
fn builder_validation_edges_surface_cli_errors_and_invalid_inputs() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture_file(tempdir.path(), "input.html", "<article>Hello</article>");
    let input = input_path.to_string_lossy().into_owned();

    let missing_slice_boundary = expect_cli_error(
        PreparedPreview::from_slice(InspectSliceArgs {
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
            from: None,
            to: Some("</article>".to_owned()),
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
                output: CliInspectOutputMode::Text,
                preview_chars: DEFAULT_PREVIEW_CHARS,
                include_source_text: false,
                output_file: None,
            },
        }),
        "missing --from",
    );
    assert_eq!(
        missing_slice_boundary.code,
        "CLI_REQUIRED_PARAMETER_MISSING"
    );
    assert_eq!(
        missing_slice_boundary.message,
        "--from is required unless --request-file is used."
    );

    assert!(
        PreparedExtraction::from_select(SelectArgs {
            definition: DefinitionArgs {
                request_file: None,
                emit_request_file: None
            },
            source: SourceArgs {
                input: Some(input.clone()),
                base_url: None,
                max_bytes: "banana".to_owned(),
                fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
                fetch_preflight: CliFetchPreflightMode::HeadFirst,
            },
            css: Some("article".to_owned()),
            selection: SelectionArgs {
                r#match: CliMatchMode::First,
                index: None,
            },
            output: default_extract_output(CliValueMode::Text),
        })
        .is_err()
    );
    assert!(
        PreparedExtraction::from_select(SelectArgs {
            definition: DefinitionArgs {
                request_file: None,
                emit_request_file: None
            },
            source: SourceArgs {
                input: Some(input.clone()),
                base_url: Some("ftp://example.com".to_owned()),
                max_bytes: DEFAULT_MAX_BYTES.to_string(),
                fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
                fetch_preflight: CliFetchPreflightMode::HeadFirst,
            },
            css: Some("article".to_owned()),
            selection: SelectionArgs {
                r#match: CliMatchMode::First,
                index: None,
            },
            output: default_extract_output(CliValueMode::Text),
        })
        .is_err()
    );
    assert!(
        PreparedExtraction::from_select(SelectArgs {
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
            css: Some("article".to_owned()),
            selection: SelectionArgs {
                r#match: CliMatchMode::First,
                index: None,
            },
            output: default_extract_output(CliValueMode::Attribute),
        })
        .is_err()
    );
    assert!(
        PreparedExtraction::from_slice(SliceArgs {
            definition: DefinitionArgs {
                request_file: None,
                emit_request_file: None
            },
            source: SourceArgs {
                input: Some(input.clone()),
                base_url: None,
                max_bytes: "banana".to_owned(),
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
            output: default_extract_output(CliValueMode::Text),
        })
        .is_err()
    );
    assert!(
        PreparedExtraction::from_slice(SliceArgs {
            definition: DefinitionArgs {
                request_file: None,
                emit_request_file: None
            },
            source: SourceArgs {
                input: Some(input.clone()),
                base_url: Some("ftp://example.com".to_owned()),
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
            output: default_extract_output(CliValueMode::Text),
        })
        .is_err()
    );
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
            output: default_extract_output(CliValueMode::Attribute),
        })
        .is_err()
    );
    assert!(
        PreparedSourceInspection::new(InspectSourceArgs {
            source: SourceArgs {
                input: Some(input.clone()),
                base_url: None,
                max_bytes: DEFAULT_MAX_BYTES.to_string(),
                fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
                fetch_preflight: CliFetchPreflightMode::HeadFirst,
            },
            sample_limit: DEFAULT_INSPECTION_SAMPLE_LIMIT,
            output: CliInspectOutputMode::Text,
            include_source_text: false,
            output_file: None,
            preview_chars: 0,
        })
        .is_err()
    );
    assert!(
        PreparedSourceInspection::new(InspectSourceArgs {
            source: SourceArgs {
                input: Some(input.clone()),
                base_url: None,
                max_bytes: "banana".to_owned(),
                fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
                fetch_preflight: CliFetchPreflightMode::HeadFirst,
            },
            sample_limit: DEFAULT_INSPECTION_SAMPLE_LIMIT,
            output: CliInspectOutputMode::Text,
            include_source_text: false,
            output_file: None,
            preview_chars: DEFAULT_PREVIEW_CHARS,
        })
        .is_err()
    );
    assert!(
        PreparedPreview::from_select(InspectSelectArgs {
            definition: DefinitionArgs {
                request_file: None,
                emit_request_file: None
            },
            source: SourceArgs {
                input: Some(input.clone()),
                base_url: None,
                max_bytes: "banana".to_owned(),
                fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
                fetch_preflight: CliFetchPreflightMode::HeadFirst,
            },
            css: Some("article".to_owned()),
            selection: SelectionArgs {
                r#match: CliMatchMode::First,
                index: None,
            },
            whitespace: CliWhitespaceMode::Normalize,
            rewrite_urls: false,
            output: InspectOutputArgs {
                output: CliInspectOutputMode::Text,
                preview_chars: DEFAULT_PREVIEW_CHARS,
                include_source_text: false,
                output_file: None,
            },
        })
        .is_err()
    );
    assert!(
        PreparedPreview::from_slice(InspectSliceArgs {
            definition: DefinitionArgs {
                request_file: None,
                emit_request_file: None
            },
            source: SourceArgs {
                input: Some(input),
                base_url: None,
                max_bytes: "banana".to_owned(),
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
            whitespace: CliWhitespaceMode::Normalize,
            rewrite_urls: false,
            output: InspectOutputArgs {
                output: CliInspectOutputMode::Text,
                preview_chars: DEFAULT_PREVIEW_CHARS,
                include_source_text: false,
                output_file: None,
            },
        })
        .is_err()
    );
}

fn default_extract_output(value: CliValueMode) -> ExtractOutputArgs {
    ExtractOutputArgs {
        value,
        attribute: None,
        whitespace: CliWhitespaceMode::Preserve,
        rewrite_urls: false,
        output: None,
        bundle: None,
        preview_chars: DEFAULT_PREVIEW_CHARS,
        include_source_text: false,
        output_file: None,
    }
}
