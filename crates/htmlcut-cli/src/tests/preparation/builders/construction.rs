use super::*;

#[test]
fn prepared_builders_and_helper_edges_cover_remaining_branches() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture_file(tempdir.path(), "input.html", "<article>Hello</article>");
    let input = input_path.to_string_lossy().into_owned();

    let select = PreparedExtraction::from_select(SelectArgs {
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
            output: None,
            bundle: None,
            preview_chars: DEFAULT_PREVIEW_CHARS,
            include_source_text: false,
            output_file: None,
        },
    })
    .expect("select builder");
    assert_eq!(select.command, "select");

    let slice = PreparedExtraction::from_slice(SliceArgs {
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
            output: Some(CliOutputMode::Json),
            bundle: None,
            preview_chars: DEFAULT_PREVIEW_CHARS,
            include_source_text: false,
            output_file: None,
        },
    })
    .expect("slice builder");
    assert_eq!(slice.command, "slice");

    let preview = PreparedPreview::from_select(InspectSelectArgs {
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
        whitespace: CliWhitespaceMode::Normalize,
        rewrite_urls: false,
        output: InspectOutputArgs {
            output: CliInspectOutputMode::Text,
            preview_chars: DEFAULT_PREVIEW_CHARS,
            include_source_text: false,
            output_file: None,
        },
    })
    .expect("preview builder");
    assert_eq!(
        preview.request.normalization.whitespace,
        WhitespaceMode::Normalize
    );
    let slice_preview = PreparedPreview::from_slice_with_logging(
        InspectSliceArgs {
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
            whitespace: CliWhitespaceMode::Preserve,
            rewrite_urls: false,
            output: InspectOutputArgs {
                output: CliInspectOutputMode::Json,
                preview_chars: DEFAULT_PREVIEW_CHARS,
                include_source_text: false,
                output_file: None,
            },
        },
        2,
        true,
    )
    .expect("slice preview builder");
    assert_eq!(slice_preview.command, "inspect-slice");
    assert_eq!(slice_preview.verbose, 2);
    assert!(slice_preview.quiet);

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
            output: ExtractOutputArgs {
                value: CliValueMode::Text,
                attribute: None,
                whitespace: CliWhitespaceMode::Preserve,
                rewrite_urls: false,
                output: None,
                bundle: None,
                preview_chars: DEFAULT_PREVIEW_CHARS,
                include_source_text: false,
                output_file: None,
            },
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
            output: ExtractOutputArgs {
                value: CliValueMode::Text,
                attribute: None,
                whitespace: CliWhitespaceMode::Preserve,
                rewrite_urls: false,
                output: None,
                bundle: None,
                preview_chars: DEFAULT_PREVIEW_CHARS,
                include_source_text: false,
                output_file: None,
            },
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
            output: ExtractOutputArgs {
                value: CliValueMode::Attribute,
                attribute: None,
                whitespace: CliWhitespaceMode::Preserve,
                rewrite_urls: false,
                output: None,
                bundle: None,
                preview_chars: DEFAULT_PREVIEW_CHARS,
                include_source_text: false,
                output_file: None,
            },
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
            output: ExtractOutputArgs {
                value: CliValueMode::Text,
                attribute: None,
                whitespace: CliWhitespaceMode::Preserve,
                rewrite_urls: false,
                output: None,
                bundle: None,
                preview_chars: DEFAULT_PREVIEW_CHARS,
                include_source_text: false,
                output_file: None,
            },
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
            output: ExtractOutputArgs {
                value: CliValueMode::Text,
                attribute: None,
                whitespace: CliWhitespaceMode::Preserve,
                rewrite_urls: false,
                output: None,
                bundle: None,
                preview_chars: DEFAULT_PREVIEW_CHARS,
                include_source_text: false,
                output_file: None,
            },
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
            output: ExtractOutputArgs {
                value: CliValueMode::Attribute,
                attribute: None,
                whitespace: CliWhitespaceMode::Preserve,
                rewrite_urls: false,
                output: None,
                bundle: None,
                preview_chars: DEFAULT_PREVIEW_CHARS,
                include_source_text: false,
                output_file: None,
            },
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

    assert!(!raw_args_prefers_json(&[
        "htmlcut".to_owned(),
        "select".to_owned(),
        "page.html".to_owned(),
        "--output=text".to_owned(),
    ]));
    assert!(raw_args_prefers_json(&[
        "htmlcut".to_owned(),
        "select".to_owned(),
        "page.html".to_owned(),
        "--output".to_owned(),
        "json".to_owned(),
    ]));
    assert!(!raw_args_prefers_json(&[
        "htmlcut".to_owned(),
        "select".to_owned(),
        "page.html".to_owned(),
        "--output".to_owned(),
        "html".to_owned(),
    ]));
    assert!(!raw_args_prefers_json(&[
        "htmlcut".to_owned(),
        "select".to_owned(),
        "page.html".to_owned(),
        "--output".to_owned(),
        "none".to_owned(),
    ]));
    assert!(!raw_args_prefers_json(&[
        "htmlcut".to_owned(),
        "select".to_owned(),
        "page.html".to_owned(),
        "--output".to_owned(),
        "mystery".to_owned(),
    ]));
    assert!(!raw_args_prefers_json(&[
        "htmlcut".to_owned(),
        "select".to_owned(),
        "page.html".to_owned(),
    ]));
    assert_eq!(
        command_name_from_raw_args(&[
            "htmlcut".to_owned(),
            "inspect".to_owned(),
            "mystery".to_owned(),
        ]),
        "inspect"
    );
    assert_eq!(
        command_name_from_raw_args(&["htmlcut".to_owned(), "--help".to_owned()]),
        "htmlcut"
    );

    let report = build_extraction_report(
        "select",
        fixture_result(Value::String("Hello".to_owned()), ValueType::Text),
        None,
    );
    assert_eq!(build_verbose_lines(&report, 1).len(), 1);

    let mut minimal_inspection = fixture_inspection();
    let document = minimal_inspection.document.as_mut().expect("document");
    document.top_tags.clear();
    document.top_classes.clear();
    document.headings.clear();
    document.links.clear();
    let rendered = render_source_inspection_text(&minimal_inspection, DEFAULT_PREVIEW_CHARS);
    assert!(!rendered.contains("Top tags:"));
    assert!(!rendered.contains("Top classes:"));
    assert!(!rendered.contains("Headings:"));
    assert!(!rendered.contains("Link previews:"));
}
