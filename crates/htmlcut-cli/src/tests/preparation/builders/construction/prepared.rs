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
            value: CliValueMode::Text,
            attribute: None,
            whitespace: CliWhitespaceMode::Rendered,
            rewrite_urls: false,
            output: None,
            bundle: None,
            preview_chars: DEFAULT_PREVIEW_CHARS,
            include_source_text: false,
            output_file: None,
        },
        file_write: default_file_write_args(),
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
            fetch_connect_timeout_ms: htmlcut_core::DEFAULT_FETCH_CONNECT_TIMEOUT_MS,
            tls_trust: CliTlsTrustMode::WebPki,
            tls_ca_bundle: None,
            fetch_preflight: CliFetchPreflightMode::HeadFirst,
        },
        from: Some("<article>".to_owned()),
        to: Some("</article>".to_owned()),
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
            output: Some(CliOutputMode::Json),
            bundle: None,
            preview_chars: DEFAULT_PREVIEW_CHARS,
            include_source_text: false,
            output_file: None,
        },
        file_write: default_file_write_args(),
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
        whitespace: CliWhitespaceMode::Normalize,
        rewrite_urls: false,
        output: InspectOutputArgs {
            output: CliInspectOutputMode::Text,
            preview_chars: DEFAULT_PREVIEW_CHARS,
            include_source_text: false,
            output_file: None,
        },
        file_write: default_preview_file_write_args(),
    })
    .expect("preview builder");
    assert_eq!(
        preview.request.output.rendering.whitespace,
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
                fetch_connect_timeout_ms: htmlcut_core::DEFAULT_FETCH_CONNECT_TIMEOUT_MS,
                tls_trust: CliTlsTrustMode::WebPki,
                tls_ca_bundle: None,
                fetch_preflight: CliFetchPreflightMode::HeadFirst,
            },
            from: Some("<article>".to_owned()),
            to: Some("</article>".to_owned()),
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
        },
        2,
        true,
    )
    .expect("slice preview builder");
    assert_eq!(slice_preview.command, "inspect-slice");
    assert_eq!(slice_preview.verbose, 2);
    assert!(slice_preview.quiet);
}
