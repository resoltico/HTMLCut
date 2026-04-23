pub(super) use super::*;

#[test]
fn prepared_builders_cover_select_slice_and_preview_variants() {
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
                input: Some(input),
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
}
