use super::*;

#[test]
fn logging_aware_preparation_preserves_request_file_configuration() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture_file(tempdir.path(), "input.html", "<article>Hello</article>");

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

    let prepared_select = PreparedExtraction::from_select_with_logging(
        SelectArgs {
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
            output: ExtractOutputArgs {
                value: CliValueMode::Text,
                attribute: None,
                whitespace: CliWhitespaceMode::Preserve,
                rewrite_urls: false,
                output: Some(CliOutputMode::Text),
                bundle: None,
                preview_chars: DEFAULT_PREVIEW_CHARS,
                include_source_text: false,
                output_file: Some(tempdir.path().join("select-output.txt")),
            },
        },
        2,
        true,
    )
    .expect("select request file");
    assert_eq!(
        prepared_select.request.extraction.strategy(),
        ExtractionStrategy::Selector
    );
    assert_eq!(prepared_select.verbose, 2);
    assert!(prepared_select.quiet);

    let prepared_source = PreparedSourceInspection::new_with_logging(
        InspectSourceArgs {
            source: SourceArgs {
                input: Some(input_path.to_string_lossy().into_owned()),
                base_url: None,
                max_bytes: DEFAULT_MAX_BYTES.to_string(),
                fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
                fetch_preflight: CliFetchPreflightMode::HeadFirst,
            },
            output: CliInspectOutputMode::Text,
            include_source_text: false,
            output_file: Some(tempdir.path().join("inspect-source.txt")),
            sample_limit: DEFAULT_INSPECTION_SAMPLE_LIMIT,
            preview_chars: DEFAULT_PREVIEW_CHARS,
        },
        3,
        true,
    )
    .expect("inspect source");
    assert_eq!(prepared_source.verbose, 3);
    assert!(prepared_source.quiet);

    let prepared_preview_select = PreparedPreview::from_select_with_logging(
        InspectSelectArgs {
            definition: DefinitionArgs {
                request_file: Some(selector_definition_path),
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
                output: CliInspectOutputMode::Text,
                preview_chars: DEFAULT_PREVIEW_CHARS,
                include_source_text: false,
                output_file: Some(tempdir.path().join("inspect-select.txt")),
            },
        },
        2,
        true,
    )
    .expect("inspect select request file");
    assert_eq!(
        prepared_preview_select.request.extraction.strategy(),
        ExtractionStrategy::Selector
    );
    assert_eq!(prepared_preview_select.verbose, 2);
    assert!(prepared_preview_select.quiet);

    let prepared_preview_slice = PreparedPreview::from_slice_with_logging(
        InspectSliceArgs {
            definition: DefinitionArgs {
                request_file: Some(slice_definition_path),
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
                output: CliInspectOutputMode::Text,
                preview_chars: DEFAULT_PREVIEW_CHARS,
                include_source_text: false,
                output_file: Some(tempdir.path().join("inspect-slice.txt")),
            },
        },
        1,
        true,
    )
    .expect("inspect slice request file");
    assert_eq!(
        prepared_preview_slice.request.extraction.strategy(),
        ExtractionStrategy::Slice
    );
    assert_eq!(prepared_preview_slice.verbose, 1);
    assert!(prepared_preview_slice.quiet);
}
