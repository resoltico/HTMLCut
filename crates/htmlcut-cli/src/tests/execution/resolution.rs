use super::*;

#[test]
fn resolve_selection_spec_validates_index_rules() {
    assert_eq!(
        resolve_selection_spec(&SelectionArgs {
            r#match: CliMatchMode::Single,
            index: None,
        })
        .expect("selection"),
        SelectionSpec::single()
    );
    assert!(
        resolve_selection_spec(&SelectionArgs {
            r#match: CliMatchMode::Nth,
            index: None,
        })
        .is_err()
    );
    assert!(
        resolve_selection_spec(&SelectionArgs {
            r#match: CliMatchMode::First,
            index: Some(1),
        })
        .is_err()
    );
    assert_eq!(
        resolve_selection_spec(&SelectionArgs {
            r#match: CliMatchMode::Nth,
            index: Some(2),
        })
        .expect("selection")
        .index()
        .map(NonZeroUsize::get),
        Some(2usize)
    );
}

#[test]
fn parser_guarded_output_modes_still_panic_when_called_directly() {
    let catalog_panic = std::panic::catch_unwind(|| {
        run_catalog(
            CatalogArgs {
                output: CliCatalogOutputMode::Html,
                output_file: None,
                operation: None,
            },
            0,
            false,
        )
    });
    assert!(
        catalog_panic.is_err(),
        "catalog html should stay unreachable"
    );

    let schema_panic = std::panic::catch_unwind(|| {
        run_schema(
            SchemaArgs {
                output: CliSchemaOutputMode::None,
                output_file: None,
                name: None,
                schema_version: None,
            },
            0,
            false,
        )
    });
    assert!(schema_panic.is_err(), "schema none should stay unreachable");

    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture_file(tempdir.path(), "input.html", "<article>Hello</article>");
    let input = input_path.to_string_lossy().into_owned();

    let inspect_source_panic = std::panic::catch_unwind(|| {
        run_inspect_source(
            InspectSourceArgs {
                source: SourceArgs {
                    input: Some(input.clone()),
                    base_url: None,
                    max_bytes: DEFAULT_MAX_BYTES.to_string(),
                    fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
                    fetch_preflight: CliFetchPreflightMode::HeadFirst,
                },
                sample_limit: DEFAULT_INSPECTION_SAMPLE_LIMIT,
                output: CliInspectOutputMode::Html,
                include_source_text: false,
                preview_chars: DEFAULT_PREVIEW_CHARS,
                output_file: None,
            },
            0,
            false,
        )
    });
    assert!(
        inspect_source_panic.is_err(),
        "inspect source html should stay unreachable"
    );

    let prepared_preview = PreparedPreview::from_select(InspectSelectArgs {
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
        css: Some("article".to_owned()),
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
    .expect("preview builder");

    let preview_panic = std::panic::catch_unwind(|| {
        execute_preview(PreparedPreview {
            output: CliInspectOutputMode::None,
            ..prepared_preview
        })
    });
    assert!(
        preview_panic.is_err(),
        "inspect preview none should stay unreachable"
    );
}

#[test]
fn resolve_value_spec_validates_attribute_usage() {
    assert!(resolve_value_spec(CliValueMode::Attribute, None).is_err());
    assert!(resolve_value_spec(CliValueMode::Text, Some("href".to_owned())).is_err());
    assert_eq!(
        resolve_value_spec(CliValueMode::Attribute, Some("href".to_owned()))
            .expect("attribute value")
            .attribute_name()
            .map(|name| name.as_str()),
        Some("href")
    );
    assert_eq!(
        resolve_value_spec(CliValueMode::Text, None)
            .expect("text value")
            .value_type(),
        ValueType::Text
    );
    assert_eq!(
        resolve_value_spec(CliValueMode::Structured, None)
            .expect("value")
            .value_type(),
        ValueType::Structured
    );
}

#[test]
fn resolve_extract_output_mode_enforces_value_and_bundle_rules() {
    assert!(resolve_extract_output_mode(None, &ValueType::Text, None).is_ok());
    assert_eq!(
        resolve_extract_output_mode(
            Some(CliOutputMode::None),
            &ValueType::Text,
            Some(Path::new("/tmp/bundle"))
        )
        .expect("none with bundle"),
        CliOutputMode::None
    );
    assert_eq!(
        resolve_extract_output_mode(Some(CliOutputMode::Html), &ValueType::InnerHtml, None)
            .expect("html for html"),
        CliOutputMode::Html
    );
    assert_eq!(
        resolve_extract_output_mode(Some(CliOutputMode::Html), &ValueType::OuterHtml, None)
            .expect("html for outer html"),
        CliOutputMode::Html
    );
    assert_eq!(
        resolve_extract_output_mode(Some(CliOutputMode::Json), &ValueType::Structured, None)
            .expect("structured json"),
        CliOutputMode::Json
    );
    assert_eq!(
        resolve_extract_output_mode(
            Some(CliOutputMode::None),
            &ValueType::Structured,
            Some(Path::new("/tmp/bundle"))
        )
        .expect("structured none"),
        CliOutputMode::None
    );
    assert!(
        resolve_extract_output_mode(Some(CliOutputMode::None), &ValueType::Text, None).is_err()
    );
    assert!(
        resolve_extract_output_mode(
            Some(CliOutputMode::Html),
            &ValueType::Text,
            Some(Path::new("/tmp/bundle"))
        )
        .is_err()
    );
    assert!(
        resolve_extract_output_mode(
            Some(CliOutputMode::Text),
            &ValueType::Structured,
            Some(Path::new("/tmp/bundle"))
        )
        .is_err()
    );
}

#[test]
fn resolve_regex_flags_rejects_literal_mode_overrides() {
    assert_eq!(
        resolve_regex_flags(CliPatternMode::Regex, Some("us".to_owned())).expect("flags"),
        Some("us".to_owned())
    );
    assert_eq!(
        resolve_regex_flags(CliPatternMode::Regex, None).expect("default regex flags"),
        Some(DEFAULT_REGEX_FLAGS.to_owned())
    );
    assert!(resolve_regex_flags(CliPatternMode::Literal, Some("u".to_owned())).is_err());
    assert_eq!(
        resolve_regex_flags(CliPatternMode::Literal, None).expect("flags"),
        None
    );
}

#[test]
fn extract_prefers_json_matches_default_structured_behavior() {
    assert!(extract_prefers_json(&ExtractOutputArgs {
        value: CliValueMode::Structured,
        attribute: None,
        whitespace: CliWhitespaceMode::Preserve,
        rewrite_urls: false,
        output: None,
        bundle: None,
        preview_chars: DEFAULT_PREVIEW_CHARS,
        include_source_text: false,
        output_file: None,
    }));
    assert!(!extract_prefers_json(&ExtractOutputArgs {
        value: CliValueMode::Text,
        attribute: None,
        whitespace: CliWhitespaceMode::Preserve,
        rewrite_urls: false,
        output: Some(CliOutputMode::Text),
        bundle: None,
        preview_chars: DEFAULT_PREVIEW_CHARS,
        include_source_text: false,
        output_file: None,
    }));
}
