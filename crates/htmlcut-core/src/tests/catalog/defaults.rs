use super::*;

#[test]
fn defaults_cover_public_default_contracts() {
    assert_eq!(WhitespaceMode::default(), WhitespaceMode::Preserve);
    assert_eq!(SelectionSpec::default(), SelectionSpec::First);
    assert_eq!(ValueSpec::default().value_type(), ValueType::Text);
    let slice = slice_spec("<p>", "</p>");
    assert_eq!(slice.mode(), PatternMode::Literal);
    assert!(!slice.include_start);
    assert!(!slice.include_end);
    assert_eq!(slice.flags(), None);
    assert_eq!(
        ExtractionSpec::selector(selector_query("article")).strategy(),
        ExtractionStrategy::Selector
    );
    assert!(!NormalizationOptions::default().rewrite_urls);
    assert_eq!(
        OutputOptions::default().preview_chars,
        NonZeroUsize::new(DEFAULT_PREVIEW_CHARS).expect("preview chars")
    );
    assert_eq!(SourceRequest::stdin().kind(), SourceKind::Stdin);
    assert_eq!(
        selector_request("<article />").spec_version,
        CORE_SPEC_VERSION
    );
    assert_eq!(RuntimeOptions::default().max_bytes, DEFAULT_MAX_BYTES);
    assert_eq!(default_spec_version(), CORE_SPEC_VERSION);
    assert_eq!(default_preview_chars(), DEFAULT_PREVIEW_CHARS);
    assert_eq!(
        InspectionOptions::default().sample_limit,
        DEFAULT_INSPECTION_SAMPLE_LIMIT
    );
    assert_eq!(
        default_inspection_sample_limit(),
        DEFAULT_INSPECTION_SAMPLE_LIMIT
    );
    assert!(default_true());
    assert_eq!(default_max_bytes(), DEFAULT_MAX_BYTES);
    assert_eq!(default_fetch_timeout_ms(), DEFAULT_FETCH_TIMEOUT_MS);
    assert_eq!(format_byte_size(1), "1 byte");
    assert_eq!(format_byte_size(1024), "1 KiB");
    assert_eq!(format_byte_size(1024 * 1024), "1 MiB");
    assert_eq!(format_byte_size(1024 * 1024 * 1024), "1 GiB");
    assert_eq!(format_byte_size(1024 * 1024 + 1), "1 MiB");
    assert_eq!(format_byte_size(1024 * 1024 * 1024 + 1), "1 GiB");
    assert_eq!(format_byte_size(1536), "1.5 KiB");
}
#[test]
fn operation_catalog_is_unique_and_complete() {
    let ids = operation_catalog()
        .iter()
        .map(|descriptor| descriptor.id)
        .collect::<BTreeSet<_>>();
    let id_strings = operation_catalog()
        .iter()
        .map(|descriptor| descriptor.id.as_str())
        .collect::<BTreeSet<_>>();

    assert_eq!(operation_catalog().len(), 6);
    assert_eq!(ids.len(), operation_catalog().len());
    assert_eq!(
        id_strings,
        BTreeSet::from([
            "document.parse",
            "source.inspect",
            "select.preview",
            "slice.preview",
            "select.extract",
            "slice.extract",
        ])
    );
    let document_parse = operation_catalog()
        .iter()
        .find(|descriptor| descriptor.id == OperationId::DocumentParse)
        .expect("document.parse should stay in the catalog");
    let select_extract = operation_catalog()
        .iter()
        .find(|descriptor| descriptor.id == OperationId::SelectExtract)
        .expect("select.extract should stay in the catalog");
    let select_preview = operation_catalog()
        .iter()
        .find(|descriptor| descriptor.id == OperationId::SelectPreview)
        .expect("select.preview should stay in the catalog");

    assert_eq!(document_parse.cli_surface, None);
    assert_eq!(select_extract.cli_surface, Some("select"));
    assert!(select_extract.core_surface.contains("extract"));
    assert_eq!(select_preview.cli_surface, Some("inspect select"));
    assert!(select_preview.core_surface.contains("preview_extraction"));
    assert_eq!(OperationId::DocumentParse.to_string(), "document.parse");
    assert_eq!(
        "slice.extract"
            .parse::<OperationId>()
            .expect("operation id"),
        OperationId::SliceExtract
    );
    assert!(matches!(
        "nope".parse::<OperationId>(),
        Err(OperationIdParseError)
    ));
    assert_eq!(
        OperationIdParseError.to_string(),
        "unknown HTMLCut operation ID"
    );
    assert_eq!(
        operation_descriptor(OperationId::SourceInspect)
            .expect("source.inspect descriptor")
            .cli_surface,
        Some("inspect source")
    );

    let select_contract = crate::cli_contract::cli_operation_contract(OperationId::SelectExtract)
        .expect("select cli contract");
    assert_eq!(select_contract.display_command(), "select");
    assert_eq!(select_contract.report_command(), "select");
    assert_eq!(
        crate::cli_contract::find_cli_operation_by_command_path(&["inspect", "slice"])
            .expect("inspect slice contract")
            .operation_id,
        OperationId::SlicePreview
    );
}
