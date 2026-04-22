use super::*;

#[test]
fn parse_byte_size_accepts_units() {
    assert_eq!(parse_byte_size("1kb").expect("byte size"), 1024);
    assert_eq!(parse_byte_size("1.5mb").expect("byte size"), 1_572_864);
    assert_eq!(parse_byte_size("0.5kb").expect("byte size"), 512);
    assert_eq!(parse_byte_size(".5kb").expect("byte size"), 512);
    assert_eq!(parse_byte_size("1gb").expect("byte size"), 1_073_741_824);
    assert!(parse_byte_size("banana").is_err());
    assert!(parse_byte_size("1tb").is_err());
    assert!(parse_byte_size("1..0kb").is_err());
    assert!(parse_byte_size(".kb").is_err());
    assert!(parse_byte_size("0.5b").is_err());
    assert!(parse_byte_size("0.1kb").is_err());
    assert!(parse_byte_size("0").is_err());
}

#[test]
fn preview_and_manifest_helpers_cover_remaining_branches() {
    assert_eq!(
        validate_preview_chars(32).expect("preview chars"),
        NonZeroUsize::new(32).expect("preview chars")
    );
    assert!(validate_preview_chars(0).is_err());
    assert_eq!(render_text_preview("short", 32), "short");
    assert_eq!(render_text_preview("preview", 3), "pre...");
    assert_eq!(
        workspace_package_field("[workspace.package]\nversion = \"3.0.0\"\n", "description"),
        None
    );
    assert_eq!(
        workspace_package_field(
            "[package]\ndescription = \"wrong\"\n[workspace.package]\ndescription = \"right\"\n",
            "description"
        ),
        Some("right".to_owned())
    );
    assert_eq!(
        workspace_package_field(
            "[workspace.package]\ndescription = \"broken\n",
            "description"
        ),
        None
    );

    let mut input_only = fixture_inspection();
    input_only.source.effective_base_url = None;
    let rendered = render_source_inspection_text(&input_only, DEFAULT_PREVIEW_CHARS);
    assert!(rendered.contains("Input base URL: https://example.com/docs/start.html"));
    assert!(!rendered.contains("Effective base URL: https://example.com/docs/start.html"));
}

#[test]
fn catalog_and_preview_renderers_cover_remaining_branches() {
    let empty_catalog = CatalogCommandReport {
        tool: TOOL_NAME.to_owned(),
        version: HTMLCUT_VERSION.to_owned(),
        schema_name: CATALOG_REPORT_SCHEMA_NAME.to_owned(),
        schema_version: crate::model::CATALOG_SCHEMA_VERSION,
        schema_profile: htmlcut_core::HTMLCUT_JSON_SCHEMA_PROFILE.to_owned(),
        description: HTMLCUT_DESCRIPTION.to_owned(),
        command: "catalog".to_owned(),
        operations: Vec::new(),
    };
    assert_eq!(
        render_catalog_text(&empty_catalog),
        format!(
            "{TOOL_NAME} {HTMLCUT_VERSION}\n{HTMLCUT_DESCRIPTION}\nCatalog: 0 operations.\nUse `htmlcut catalog --operation <OPERATION_ID> --output json` for one exact contract."
        )
    );
    assert_eq!(
        render_catalog_surface(None, &CatalogAvailability::Cli),
        "cli".to_owned()
    );

    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "catalog".to_owned(),
        "--output".to_owned(),
        "text".to_owned(),
    ]);
    assert_eq!(exit_code, 0);
    assert!(stdout.contains("Operations:"));
    assert!(stderr.is_empty());

    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "catalog".to_owned(),
        "--operation".to_owned(),
        "slice.extract".to_owned(),
    ]);
    assert_eq!(exit_code, 0);
    assert!(stdout.contains("Operation:"));
    assert!(stdout.contains("core: extract(ExtractionRequest{kind=slice}, RuntimeOptions)"));
    assert!(stdout.contains("request: ExtractionRequest + RuntimeOptions"));
    assert!(
        stdout.contains("request schemas: htmlcut.extraction_request@4, htmlcut.runtime_options@4")
    );
    assert!(stdout.contains("result: ExtractionResult"));
    assert!(stdout.contains("result schemas: htmlcut.extraction_result@5"));
    assert!(stdout.contains("usage: htmlcut slice [OPTIONS] --from <FROM> --to <TO> [INPUT]"));
    assert!(stdout.contains("default output: text"));
    assert!(stdout.contains("default output overrides:"));
    assert!(stdout.contains("when --value is structured => json"));
    assert!(stdout.contains("constraints:"));
    assert!(stdout.contains("requires --bundle when --output is none"));
    assert!(stdout.contains("restricts --output to json, none when --value is structured"));
    assert!(stdout.contains("parameters:"));
    assert!(stdout.contains("option --request-file <PATH> | optional"));
    assert!(stdout.contains("option --fetch-preflight <FETCH_PREFLIGHT> | optional"));
    assert!(
        stdout
            .contains("positional <INPUT> | conditional (required unless --request-file is used)")
    );
    assert!(
        stdout.contains(
            "option --from <FROM> | conditional (required unless --request-file is used)"
        )
    );
    assert!(stdout.contains("option --regex-flags <REGEX_FLAGS> | conditional (allowed only when --pattern regex is used)"));
    assert!(stdout.contains("option --output-file <PATH> | optional"));
    assert!(stdout.contains(
        "For --value outer-html, HTMLCut returns the full outer matched range including both boundaries."
    ));
    assert!(stderr.is_empty());

    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "catalog".to_owned(),
        "--output".to_owned(),
        "json".to_owned(),
        "--operation".to_owned(),
        "unknown.operation".to_owned(),
    ]);
    assert_eq!(exit_code, EXIT_CODE_USAGE);
    assert!(stdout.contains("\"code\": \"CLI_OPERATION_ID_UNKNOWN\""));
    assert!(stderr.is_empty());

    let select_preview_lines = render_preview_match_lines(
        htmlcut_core::OperationId::SelectPreview,
        &ExtractionMatch {
            index: 2,
            path: None,
            value_type: ValueType::Structured,
            value: serde_json::json!({}),
            html: None,
            text: None,
            preview: "structured preview".to_owned(),
            metadata: selector_metadata(1, 1, "article:nth-of-type(1)", "article", &[]),
        },
    );
    assert!(
        select_preview_lines
            .iter()
            .any(|line| line == "2. (no path)")
    );
    assert!(
        select_preview_lines
            .iter()
            .any(|line| line == "   preview: structured preview")
    );

    let rich_select_preview_lines = render_preview_match_lines(
        htmlcut_core::OperationId::SelectPreview,
        &ExtractionMatch {
            index: 4,
            path: Some("article:nth-of-type(1)".to_owned()),
            value_type: ValueType::Structured,
            value: serde_json::json!({}),
            html: None,
            text: Some("Alpha beta".to_owned()),
            preview: "unused".to_owned(),
            metadata: selector_metadata(
                3,
                2,
                "article:nth-of-type(1)",
                "article",
                &[("class", "card featured")],
            ),
        },
    );
    assert!(
        rich_select_preview_lines
            .iter()
            .any(|line| line == "   attributes: class=\"card featured\"")
    );
    assert!(
        rich_select_preview_lines
            .iter()
            .any(|line| line == "   text: Alpha beta")
    );

    let slice_preview_lines = render_preview_match_lines(
        htmlcut_core::OperationId::SlicePreview,
        &ExtractionMatch {
            index: 3,
            path: None,
            value_type: ValueType::Structured,
            value: serde_json::json!({}),
            html: None,
            text: None,
            preview: "slice preview".to_owned(),
            metadata: delimiter_metadata(9, 7, (1, 12), (4, 9), (1, 12), true, true),
        },
    );
    assert!(
        slice_preview_lines
            .iter()
            .any(|line| line == "3. range 1..12")
    );
    assert!(
        slice_preview_lines
            .iter()
            .any(|line| line == "   candidate index: 7")
    );
    assert!(
        slice_preview_lines
            .iter()
            .any(|line| line == "   selected range: 1..12")
    );
    assert!(
        slice_preview_lines
            .iter()
            .any(|line| line == "   inner range: 4..9")
    );
    assert!(
        slice_preview_lines
            .iter()
            .any(|line| line == "   outer range: 1..12")
    );
    assert!(
        slice_preview_lines
            .iter()
            .any(|line| line == "   include start: true")
    );
    assert!(
        slice_preview_lines
            .iter()
            .any(|line| line == "   include end: true")
    );
    assert!(
        slice_preview_lines
            .iter()
            .any(|line| line == "   preview: slice preview")
    );

    let rich_slice_preview_lines = render_preview_match_lines(
        htmlcut_core::OperationId::SlicePreview,
        &ExtractionMatch {
            index: 5,
            path: None,
            value_type: ValueType::Structured,
            value: serde_json::json!({}),
            html: None,
            text: Some("alpha beta".to_owned()),
            preview: "unused".to_owned(),
            metadata: delimiter_metadata(10, 8, (2, 7), (2, 7), (1, 8), false, false),
        },
    );
    assert!(
        rich_slice_preview_lines
            .iter()
            .any(|line| line == "   candidate index: 8")
    );
    assert!(
        rich_slice_preview_lines
            .iter()
            .any(|line| line == "   include start: false")
    );
    assert!(
        rich_slice_preview_lines
            .iter()
            .any(|line| line == "   include end: false")
    );
    assert!(
        rich_slice_preview_lines
            .iter()
            .any(|line| line == "   inner range: 2..7")
    );
    assert!(
        rich_slice_preview_lines
            .iter()
            .any(|line| line == "   outer range: 1..8")
    );
    assert!(
        rich_slice_preview_lines
            .iter()
            .any(|line| line == "   text: alpha beta")
    );

    let fragment_signal_slice_preview_lines = render_preview_match_lines(
        htmlcut_core::OperationId::SlicePreview,
        &ExtractionMatch {
            index: 8,
            path: None,
            value_type: ValueType::Structured,
            value: serde_json::json!({}),
            html: Some("START::Alpha::END".to_owned()),
            text: Some(String::new()),
            preview: "unused".to_owned(),
            metadata: delimiter_metadata(2, 1, (12, 12), (12, 12), (5, 17), false, false),
        },
    );
    assert!(
        fragment_signal_slice_preview_lines
            .iter()
            .any(|line| line == "   fragment: START::Alpha::END")
    );
    assert!(
        fragment_signal_slice_preview_lines
            .iter()
            .any(|line| line == "   text: ")
    );

    let sparse_slice_preview_lines = render_preview_match_lines(
        htmlcut_core::OperationId::SlicePreview,
        &ExtractionMatch {
            index: 6,
            path: None,
            value_type: ValueType::Structured,
            value: serde_json::json!({}),
            html: None,
            text: Some("fallback branch coverage".to_owned()),
            preview: "unused".to_owned(),
            metadata: delimiter_metadata(1, 1, (10, 20), (10, 20), (9, 21), false, false),
        },
    );
    assert!(
        sparse_slice_preview_lines
            .iter()
            .any(|line| line == "6. range 10..20")
    );
    assert!(
        sparse_slice_preview_lines
            .iter()
            .all(|line| !line.contains("source index:"))
    );
    assert!(
        sparse_slice_preview_lines
            .iter()
            .any(|line| line == "   candidate index: 1")
    );
    assert!(
        sparse_slice_preview_lines
            .iter()
            .any(|line| line == "   selected range: 10..20")
    );
    assert!(
        sparse_slice_preview_lines
            .iter()
            .any(|line| line == "   text: fallback branch coverage")
    );
    assert_eq!(
        render_preview_location(
            htmlcut_core::OperationId::SlicePreview,
            &ExtractionMatch {
                index: 7,
                path: None,
                value_type: ValueType::Structured,
                value: serde_json::json!({}),
                html: None,
                text: None,
                preview: "unused".to_owned(),
                metadata: selector_metadata(1, 1, "article:nth-of-type(1)", "article", &[]),
            }
        ),
        "(no path)".to_owned()
    );

    let fallback_preview_lines = render_preview_match_lines(
        htmlcut_core::OperationId::SelectExtract,
        &ExtractionMatch {
            index: 1,
            path: None,
            value_type: ValueType::Structured,
            value: serde_json::json!({}),
            html: None,
            text: None,
            preview: "fallback".to_owned(),
            metadata: selector_metadata(1, 1, "article:nth-of-type(1)", "article", &[]),
        },
    );
    assert!(
        fallback_preview_lines
            .iter()
            .any(|line| line == "   preview: fallback")
    );

    assert_eq!(render_attribute_summary(&attribute_map(&[])), None);
    assert_eq!(
        render_attribute_summary(&attribute_map(&[("count", "1")])),
        Some("count=\"1\"".to_owned())
    );
    assert_eq!(
        render_attribute_summary(&attribute_map(&[("class", "card")])),
        Some("class=\"card\"".to_owned())
    );
    assert_eq!(
        render_range_summary(Some(&Range { start: 9, end: 12 })),
        Some("9..12".to_owned())
    );
    assert_eq!(render_range_summary(None), None);
    assert_eq!(
        compact_inline_preview("alpha beta gamma", 5),
        "alpha...".to_owned()
    );
}

#[test]
fn schema_and_catalog_renderers_cover_optional_surfaces() {
    let core_only_operation = CatalogOperationReport {
        operation_id: htmlcut_core::OperationId::DocumentParse,
        command: None,
        availability: CatalogAvailability::CoreOnly,
        summary: "Core-only parse".to_owned(),
        core_surface: "parse_document(SourceRequest, RuntimeOptions)".to_owned(),
        request_contract: CatalogContractSurface {
            rust_shape: "SourceRequest + RuntimeOptions".to_owned(),
            schema_refs: Vec::new(),
        },
        result_contract: CatalogContractSurface {
            rust_shape: "ParseDocumentResult".to_owned(),
            schema_refs: vec![SchemaRefReport {
                schema_name: "htmlcut.parse_document_result".to_owned(),
                schema_version: 1,
            }],
        },
        command_contract: None,
    };
    let contract_operation = CatalogOperationReport {
        operation_id: htmlcut_core::OperationId::SelectExtract,
        command: Some("select".to_owned()),
        availability: CatalogAvailability::Cli,
        summary: "Synthetic contract".to_owned(),
        core_surface: "extract(ExtractionRequest{kind=selector}, RuntimeOptions)".to_owned(),
        request_contract: CatalogContractSurface {
            rust_shape: "ExtractionRequest + RuntimeOptions".to_owned(),
            schema_refs: vec![SchemaRefReport {
                schema_name: "htmlcut.extraction_request".to_owned(),
                schema_version: 2,
            }],
        },
        result_contract: CatalogContractSurface {
            rust_shape: "ExtractionResult".to_owned(),
            schema_refs: vec![SchemaRefReport {
                schema_name: "htmlcut.extraction_result".to_owned(),
                schema_version: 3,
            }],
        },
        command_contract: Some(CatalogCommandContract {
            invocation: "htmlcut select [OPTIONS] --css <CSS> [INPUT]".to_owned(),
            inputs: vec!["file".to_owned(), "url".to_owned(), "stdin".to_owned()],
            default_match: Some("first".to_owned()),
            selection_modes: vec!["single".to_owned(), "all".to_owned()],
            default_value: Some("text".to_owned()),
            value_modes: vec!["text".to_owned(), "structured".to_owned()],
            default_output: Some("text".to_owned()),
            default_output_overrides: vec![CatalogConditionalDefault {
                value: "json".to_owned(),
                when: CatalogCondition {
                    parameter: "--value".to_owned(),
                    values: vec!["structured".to_owned()],
                },
            }],
            output_modes: vec!["text".to_owned(), "json".to_owned(), "none".to_owned()],
            constraints: vec![
                CatalogConstraint::RequiresParameter {
                    parameter: "--attribute".to_owned(),
                    when: CatalogCondition {
                        parameter: "--value".to_owned(),
                        values: vec!["attribute".to_owned()],
                    },
                },
                CatalogConstraint::AllowedOnlyWhen {
                    parameter: "--regex-flags".to_owned(),
                    when: CatalogCondition {
                        parameter: "--pattern".to_owned(),
                        values: vec!["regex".to_owned()],
                    },
                },
                CatalogConstraint::RestrictsParameterValues {
                    parameter: "--output".to_owned(),
                    allowed_values: vec!["json".to_owned(), "none".to_owned()],
                    when: CatalogCondition {
                        parameter: "--value".to_owned(),
                        values: vec!["structured".to_owned()],
                    },
                },
            ],
            notes: vec!["Synthetic note".to_owned()],
            examples: vec!["htmlcut select ./page.html --css article".to_owned()],
            parameters: vec![
                CatalogParameterSpec {
                    section: "Source".to_owned(),
                    name: "<INPUT>".to_owned(),
                    kind: CatalogParameterKind::Positional,
                    requirement: CatalogParameterRequirement::Conditional,
                    requirement_note: Some("required unless --request-file is used".to_owned()),
                    value_hint: None,
                    default: None,
                    allowed_values: Vec::new(),
                    summary: "HTML input source.".to_owned(),
                },
                CatalogParameterSpec {
                    section: "Extraction".to_owned(),
                    name: "--value".to_owned(),
                    kind: CatalogParameterKind::Option,
                    requirement: CatalogParameterRequirement::Optional,
                    requirement_note: None,
                    value_hint: Some("VALUE".to_owned()),
                    default: Some("text".to_owned()),
                    allowed_values: vec!["text".to_owned(), "structured".to_owned()],
                    summary: "Choose the extracted value.".to_owned(),
                },
                CatalogParameterSpec {
                    section: "Extraction".to_owned(),
                    name: "--attribute".to_owned(),
                    kind: CatalogParameterKind::Option,
                    requirement: CatalogParameterRequirement::Conditional,
                    requirement_note: Some("required when --value attribute is used".to_owned()),
                    value_hint: Some("ATTRIBUTE".to_owned()),
                    default: None,
                    allowed_values: Vec::new(),
                    summary: "Attribute name.".to_owned(),
                },
            ],
        }),
    };
    let report = CatalogCommandReport {
        tool: TOOL_NAME.to_owned(),
        version: HTMLCUT_VERSION.to_owned(),
        schema_name: CATALOG_REPORT_SCHEMA_NAME.to_owned(),
        schema_version: crate::model::CATALOG_SCHEMA_VERSION,
        schema_profile: htmlcut_core::HTMLCUT_JSON_SCHEMA_PROFILE.to_owned(),
        description: HTMLCUT_DESCRIPTION.to_owned(),
        command: "catalog".to_owned(),
        operations: vec![core_only_operation, contract_operation],
    };
    let rendered_catalog = render_catalog_text(&report);
    assert!(rendered_catalog.contains("Core-only parse"));
    assert!(rendered_catalog.contains("inputs: file | url | stdin"));

    let single_operation_catalog = CatalogCommandReport {
        operations: vec![CatalogOperationReport {
            operation_id: htmlcut_core::OperationId::SelectExtract,
            command: Some("select".to_owned()),
            availability: CatalogAvailability::Cli,
            summary: "Synthetic contract".to_owned(),
            core_surface: "extract(ExtractionRequest{kind=selector}, RuntimeOptions)".to_owned(),
            request_contract: CatalogContractSurface {
                rust_shape: "ExtractionRequest + RuntimeOptions".to_owned(),
                schema_refs: vec![SchemaRefReport {
                    schema_name: "htmlcut.extraction_request".to_owned(),
                    schema_version: 2,
                }],
            },
            result_contract: CatalogContractSurface {
                rust_shape: "ExtractionResult".to_owned(),
                schema_refs: vec![SchemaRefReport {
                    schema_name: "htmlcut.extraction_result".to_owned(),
                    schema_version: 3,
                }],
            },
            command_contract: Some(CatalogCommandContract {
                invocation: "htmlcut select [OPTIONS] --css <CSS> [INPUT]".to_owned(),
                inputs: vec!["file".to_owned(), "url".to_owned(), "stdin".to_owned()],
                default_match: Some("first".to_owned()),
                selection_modes: vec!["single".to_owned(), "all".to_owned()],
                default_value: Some("text".to_owned()),
                value_modes: vec!["text".to_owned(), "structured".to_owned()],
                default_output: Some("text".to_owned()),
                default_output_overrides: vec![CatalogConditionalDefault {
                    value: "json".to_owned(),
                    when: CatalogCondition {
                        parameter: "--value".to_owned(),
                        values: vec!["structured".to_owned()],
                    },
                }],
                output_modes: vec!["text".to_owned(), "json".to_owned(), "none".to_owned()],
                constraints: vec![
                    CatalogConstraint::RequiresParameter {
                        parameter: "--attribute".to_owned(),
                        when: CatalogCondition {
                            parameter: "--value".to_owned(),
                            values: vec!["attribute".to_owned()],
                        },
                    },
                    CatalogConstraint::AllowedOnlyWhen {
                        parameter: "--regex-flags".to_owned(),
                        when: CatalogCondition {
                            parameter: "--pattern".to_owned(),
                            values: vec!["regex".to_owned()],
                        },
                    },
                    CatalogConstraint::RestrictsParameterValues {
                        parameter: "--output".to_owned(),
                        allowed_values: vec!["json".to_owned(), "none".to_owned()],
                        when: CatalogCondition {
                            parameter: "--value".to_owned(),
                            values: vec!["structured".to_owned()],
                        },
                    },
                ],
                notes: vec!["Synthetic note".to_owned()],
                examples: vec!["htmlcut select ./page.html --css article".to_owned()],
                parameters: vec![
                    CatalogParameterSpec {
                        section: "Source".to_owned(),
                        name: "<INPUT>".to_owned(),
                        kind: CatalogParameterKind::Positional,
                        requirement: CatalogParameterRequirement::Required,
                        requirement_note: None,
                        value_hint: None,
                        default: None,
                        allowed_values: Vec::new(),
                        summary: "HTML input source.".to_owned(),
                    },
                    CatalogParameterSpec {
                        section: "Extraction".to_owned(),
                        name: "--value".to_owned(),
                        kind: CatalogParameterKind::Option,
                        requirement: CatalogParameterRequirement::Optional,
                        requirement_note: None,
                        value_hint: Some("VALUE".to_owned()),
                        default: Some("text".to_owned()),
                        allowed_values: vec!["text".to_owned(), "structured".to_owned()],
                        summary: "Choose the extracted value.".to_owned(),
                    },
                    CatalogParameterSpec {
                        section: "Extraction".to_owned(),
                        name: "--attribute".to_owned(),
                        kind: CatalogParameterKind::Option,
                        requirement: CatalogParameterRequirement::Conditional,
                        requirement_note: Some(
                            "required when --value attribute is used".to_owned(),
                        ),
                        value_hint: Some("ATTRIBUTE".to_owned()),
                        default: None,
                        allowed_values: Vec::new(),
                        summary: "Attribute name.".to_owned(),
                    },
                ],
            }),
        }],
        ..report
    };
    let rendered_single_catalog = render_catalog_text(&single_operation_catalog);
    assert!(rendered_single_catalog.contains("inputs: file | url | stdin"));
    assert!(rendered_single_catalog.contains("default match: first"));
    assert!(rendered_single_catalog.contains("match modes: single, all"));
    assert!(rendered_single_catalog.contains("default value: text"));
    assert!(rendered_single_catalog.contains("value modes: text, structured"));
    assert!(rendered_single_catalog.contains("default output: text"));
    assert!(rendered_single_catalog.contains("default output overrides:"));
    assert!(rendered_single_catalog.contains("requires --attribute when --value is attribute"));
    assert!(rendered_single_catalog.contains("allows --regex-flags only when --pattern is regex"));
    assert!(
        rendered_single_catalog
            .contains("restricts --output to json, none when --value is structured")
    );
    assert!(rendered_single_catalog.contains("option --attribute <ATTRIBUTE> | conditional"));
    assert!(rendered_single_catalog.contains("default: text"));
    assert!(rendered_single_catalog.contains("values: text, structured"));

    let single_schema = SchemaCommandReport {
        tool: TOOL_NAME.to_owned(),
        version: HTMLCUT_VERSION.to_owned(),
        schema_name: SCHEMA_COMMAND_REPORT_SCHEMA_NAME.to_owned(),
        schema_version: SCHEMA_COMMAND_REPORT_SCHEMA_VERSION,
        schema_profile: htmlcut_core::HTMLCUT_JSON_SCHEMA_PROFILE.to_owned(),
        description: HTMLCUT_DESCRIPTION.to_owned(),
        command: "schema".to_owned(),
        schemas: vec![SchemaDocumentReport {
            schema_name: "synthetic.single".to_owned(),
            schema_version: 7,
            owner_surface: "tests".to_owned(),
            rust_shape: "Synthetic".to_owned(),
            stability: htmlcut_core::SchemaStability::Frozen,
            json_schema: Value::String("not an object".to_owned()),
        }],
    };
    let rendered_single_schema = render_schema_text(&single_schema);
    assert!(rendered_single_schema.contains("Schema:"));
    assert!(rendered_single_schema.contains("synthetic.single@7 | tests | frozen"));
    assert!(rendered_single_schema.contains("json schema keys: (not-an-object)"));

    let multi_schema = SchemaCommandReport {
        schemas: vec![
            SchemaDocumentReport {
                schema_name: "synthetic.a".to_owned(),
                schema_version: 1,
                owner_surface: "tests".to_owned(),
                rust_shape: "A".to_owned(),
                stability: htmlcut_core::SchemaStability::Versioned,
                json_schema: serde_json::json!({ "type": "object" }),
            },
            SchemaDocumentReport {
                schema_name: "synthetic.b".to_owned(),
                schema_version: 2,
                owner_surface: "tests".to_owned(),
                rust_shape: "B".to_owned(),
                stability: htmlcut_core::SchemaStability::Frozen,
                json_schema: serde_json::json!({ "type": "object" }),
            },
        ],
        ..single_schema
    };
    let rendered_multi_schema = render_schema_text(&multi_schema);
    assert!(rendered_multi_schema.contains("Schemas:"));
    assert!(rendered_multi_schema.contains("synthetic.a@1 | tests | versioned"));
    assert!(rendered_multi_schema.contains("synthetic.b@2 | tests | frozen"));
    assert!(!rendered_multi_schema.contains("json schema keys:"));
}

#[test]
fn direct_render_helpers_cover_empty_optional_branches() {
    let minimal_contract = CatalogCommandContract {
        invocation: "htmlcut select <INPUT>".to_owned(),
        inputs: Vec::new(),
        default_match: None,
        selection_modes: Vec::new(),
        default_value: None,
        value_modes: Vec::new(),
        default_output: None,
        default_output_overrides: Vec::new(),
        output_modes: Vec::new(),
        constraints: Vec::new(),
        notes: Vec::new(),
        examples: Vec::new(),
        parameters: Vec::new(),
    };
    let minimal_report = CatalogCommandReport {
        tool: TOOL_NAME.to_owned(),
        version: HTMLCUT_VERSION.to_owned(),
        schema_name: CATALOG_REPORT_SCHEMA_NAME.to_owned(),
        schema_version: crate::model::CATALOG_SCHEMA_VERSION,
        schema_profile: htmlcut_core::HTMLCUT_JSON_SCHEMA_PROFILE.to_owned(),
        description: HTMLCUT_DESCRIPTION.to_owned(),
        command: "catalog".to_owned(),
        operations: vec![CatalogOperationReport {
            operation_id: htmlcut_core::OperationId::DocumentParse,
            command: Some("select".to_owned()),
            availability: CatalogAvailability::Cli,
            summary: "Minimal".to_owned(),
            core_surface: "BareCoreSurface".to_owned(),
            request_contract: CatalogContractSurface {
                rust_shape: "BareShape".to_owned(),
                schema_refs: Vec::new(),
            },
            result_contract: CatalogContractSurface {
                rust_shape: "BareResult".to_owned(),
                schema_refs: Vec::new(),
            },
            command_contract: Some(minimal_contract),
        }],
    };
    let minimal_render = render_catalog_text(&minimal_report);
    assert!(minimal_render.contains("usage: htmlcut select <INPUT>"));
    assert!(minimal_render.contains("request: BareShape"));
    assert!(minimal_render.contains("result: BareResult"));
    assert!(!minimal_render.contains("inputs:"));
    assert!(!minimal_render.contains("default output:"));
    assert!(!minimal_render.contains("constraints:"));
    assert!(!minimal_render.contains("parameters:"));

    let focused_render = render_catalog_text(&CatalogCommandReport {
        operations: vec![CatalogOperationReport {
            operation_id: htmlcut_core::OperationId::SelectExtract,
            command: Some("select".to_owned()),
            availability: CatalogAvailability::Cli,
            summary: "Focused".to_owned(),
            core_surface: "FocusedCoreSurface".to_owned(),
            request_contract: CatalogContractSurface {
                rust_shape: "FocusedRequest".to_owned(),
                schema_refs: Vec::new(),
            },
            result_contract: CatalogContractSurface {
                rust_shape: "FocusedResult".to_owned(),
                schema_refs: Vec::new(),
            },
            command_contract: Some(CatalogCommandContract {
                invocation: "htmlcut select <INPUT>".to_owned(),
                inputs: vec!["file".to_owned(), "url".to_owned()],
                default_match: None,
                selection_modes: Vec::new(),
                default_value: None,
                value_modes: Vec::new(),
                default_output: Some("text".to_owned()),
                default_output_overrides: Vec::new(),
                output_modes: Vec::new(),
                constraints: vec![CatalogConstraint::RequiresParameter {
                    parameter: "--thing".to_owned(),
                    when: CatalogCondition {
                        parameter: "--mode".to_owned(),
                        values: Vec::new(),
                    },
                }],
                notes: Vec::new(),
                examples: Vec::new(),
                parameters: vec![
                    CatalogParameterSpec {
                        section: "Synthetic".to_owned(),
                        name: "--flag".to_owned(),
                        kind: CatalogParameterKind::Flag,
                        requirement: CatalogParameterRequirement::Optional,
                        requirement_note: None,
                        value_hint: Some("IGNORED".to_owned()),
                        default: None,
                        allowed_values: Vec::new(),
                        summary: "Synthetic flag.".to_owned(),
                    },
                    CatalogParameterSpec {
                        section: "Synthetic".to_owned(),
                        name: "--conditional".to_owned(),
                        kind: CatalogParameterKind::Option,
                        requirement: CatalogParameterRequirement::Conditional,
                        requirement_note: None,
                        value_hint: Some("VALUE".to_owned()),
                        default: None,
                        allowed_values: Vec::new(),
                        summary: "Synthetic conditional.".to_owned(),
                    },
                ],
            }),
        }],
        ..minimal_report
    });
    assert!(focused_render.contains("inputs: file | url"));
    assert!(focused_render.contains("default output: text"));
    assert!(focused_render.contains("requires --thing when --mode"));
    assert!(focused_render.contains("flag --flag | optional"));
    assert!(
        focused_render.contains("option --conditional <VALUE> | conditional (see command notes)")
    );

    let empty_schema_report = SchemaCommandReport {
        tool: TOOL_NAME.to_owned(),
        version: HTMLCUT_VERSION.to_owned(),
        schema_name: SCHEMA_COMMAND_REPORT_SCHEMA_NAME.to_owned(),
        schema_version: SCHEMA_COMMAND_REPORT_SCHEMA_VERSION,
        schema_profile: htmlcut_core::HTMLCUT_JSON_SCHEMA_PROFILE.to_owned(),
        description: HTMLCUT_DESCRIPTION.to_owned(),
        command: "schema".to_owned(),
        schemas: Vec::new(),
    };
    let empty_schema_text = render_schema_text(&empty_schema_report);
    assert!(!empty_schema_text.contains("Schema:"));
    assert!(!empty_schema_text.contains("Schemas:"));
    assert!(empty_schema_text.contains("Schema profile:"));
}

#[test]
fn preview_helpers_cover_metadata_mismatches_and_empty_reports() {
    let empty_preview = build_extraction_report(
        "inspect-select",
        fixture_result(
            serde_json::json!({"tagName":"article"}),
            ValueType::Structured,
        ),
        None,
    );
    let mut empty_preview = empty_preview;
    empty_preview.matches.clear();
    empty_preview.diagnostics.clear();
    let empty_preview_text = render_preview_text(&empty_preview);
    assert!(!empty_preview_text.contains("Diagnostics:"));
    assert!(!empty_preview_text.contains("Matches:"));

    let select_preview_with_slice_metadata = render_preview_match_lines(
        htmlcut_core::OperationId::SelectPreview,
        &ExtractionMatch {
            index: 1,
            path: Some("explicit-path".to_owned()),
            value_type: ValueType::Structured,
            value: serde_json::json!({}),
            html: None,
            text: None,
            preview: "fallback select preview".to_owned(),
            metadata: delimiter_metadata(1, 1, (1, 3), (1, 3), (1, 3), false, false),
        },
    );
    assert_eq!(select_preview_with_slice_metadata[0], "1. explicit-path");
    assert!(
        select_preview_with_slice_metadata
            .iter()
            .any(|line| line == "   preview: fallback select preview")
    );
    assert!(
        select_preview_with_slice_metadata
            .iter()
            .all(|line| !line.contains("tag:"))
    );

    let slice_preview_with_selector_metadata = render_preview_match_lines(
        htmlcut_core::OperationId::SlicePreview,
        &ExtractionMatch {
            index: 2,
            path: Some("slice-path".to_owned()),
            value_type: ValueType::Structured,
            value: serde_json::json!({}),
            html: Some("same".to_owned()),
            text: Some("same".to_owned()),
            preview: "unused".to_owned(),
            metadata: selector_metadata(1, 1, "article:nth-of-type(1)", "article", &[]),
        },
    );
    assert_eq!(slice_preview_with_selector_metadata[0], "2. slice-path");
    assert!(
        slice_preview_with_selector_metadata
            .iter()
            .any(|line| line == "   text: same")
    );
    assert!(
        slice_preview_with_selector_metadata
            .iter()
            .all(|line| !line.contains("candidate index:"))
    );
    assert!(
        slice_preview_with_selector_metadata
            .iter()
            .all(|line| !line.contains("fragment:"))
    );
}

#[test]
fn schema_execution_and_prepare_helpers_cover_remaining_branches() {
    let catalog_report = build_catalog_report(None).expect("full catalog");
    assert!(
        catalog_report
            .operations
            .iter()
            .any(|operation| operation.availability == CatalogAvailability::Cli)
    );
    assert!(
        catalog_report
            .operations
            .iter()
            .any(|operation| operation.availability == CatalogAvailability::CoreOnly)
    );

    let text_outcome = run_schema(
        SchemaArgs {
            output: CliSchemaOutputMode::Text,
            output_file: None,
            name: Some("htmlcut.result".to_owned()),
            schema_version: Some(1),
        },
        0,
        false,
    );
    assert_eq!(text_outcome.exit_code, 0);
    assert!(
        text_outcome
            .stdout
            .as_deref()
            .is_some_and(|stdout| stdout.contains("Schema:"))
    );

    let json_error_outcome = run_schema(
        SchemaArgs {
            output: CliSchemaOutputMode::Json,
            output_file: None,
            name: Some("synthetic.missing".to_owned()),
            schema_version: Some(99),
        },
        0,
        false,
    );
    assert_eq!(json_error_outcome.exit_code, EXIT_CODE_USAGE);
    assert!(
        json_error_outcome
            .stdout
            .as_deref()
            .is_some_and(|stdout| stdout.contains("\"code\": \"CLI_SCHEMA_UNKNOWN\""))
    );

    let text_error_outcome = run_schema(
        SchemaArgs {
            output: CliSchemaOutputMode::Text,
            output_file: None,
            name: None,
            schema_version: Some(1),
        },
        0,
        false,
    );
    assert_eq!(text_error_outcome.exit_code, EXIT_CODE_USAGE);
    assert!(
        text_error_outcome
            .stderr
            .iter()
            .any(|line| line.contains("`--schema-version` requires `--name`."))
    );

    let source = build_source_request(&SourceArgs {
        input: Some("https://example.com/docs/page.html".to_owned()),
        base_url: Some("https://base.example/root/".to_owned()),
        max_bytes: DEFAULT_MAX_BYTES.to_string(),
        fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
        fetch_preflight: CliFetchPreflightMode::HeadFirst,
    })
    .expect("url source request");
    assert!(matches!(
        source.input,
        htmlcut_core::SourceInput::Url { .. }
    ));
    assert_eq!(
        source.base_url.as_ref().map(ToString::to_string).as_deref(),
        Some("https://base.example/root/")
    );
    let http_source = build_source_request(&SourceArgs {
        input: Some("http://example.com/docs/page.html".to_owned()),
        base_url: None,
        max_bytes: DEFAULT_MAX_BYTES.to_string(),
        fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
        fetch_preflight: CliFetchPreflightMode::HeadFirst,
    })
    .expect("http url source request");
    assert!(matches!(
        http_source.input,
        htmlcut_core::SourceInput::Url { .. }
    ));

    let invalid_base_url = build_source_request(&SourceArgs {
        input: Some("-".to_owned()),
        base_url: Some("ftp://example.com".to_owned()),
        max_bytes: DEFAULT_MAX_BYTES.to_string(),
        fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
        fetch_preflight: CliFetchPreflightMode::HeadFirst,
    })
    .expect_err("invalid base url");
    assert_eq!(invalid_base_url.code, "CLI_BASE_URL_SCHEME_INVALID");

    assert!(
        !build_schema_report(None, None)
            .expect("full schema catalog")
            .schemas
            .is_empty()
    );
    assert_eq!(
        build_schema_report(Some("htmlcut.result"), Some(1))
            .expect("filtered schema")
            .schemas
            .len(),
        1
    );
    assert_eq!(
        build_schema_report(Some("synthetic.missing"), None)
            .expect_err("missing schema by name")
            .code,
        "CLI_SCHEMA_UNKNOWN"
    );
    assert_eq!(
        build_schema_report(Some("synthetic.missing"), Some(99))
            .expect_err("missing schema by name and version")
            .code,
        "CLI_SCHEMA_UNKNOWN"
    );

    assert_eq!(
        resolve_selection_spec(&SelectionArgs {
            r#match: CliMatchMode::All,
            index: None,
        })
        .expect("all selection"),
        SelectionSpec::All
    );
    assert_eq!(
        resolve_selection_spec(&SelectionArgs {
            r#match: CliMatchMode::Single,
            index: Some(1),
        })
        .expect_err("single index conflict")
        .code,
        "CLI_MATCH_INDEX_CONFLICT"
    );
    assert_eq!(
        resolve_selection_spec(&SelectionArgs {
            r#match: CliMatchMode::All,
            index: Some(1),
        })
        .expect_err("all index conflict")
        .code,
        "CLI_MATCH_INDEX_CONFLICT"
    );
    assert_eq!(
        resolve_selection_spec(&SelectionArgs {
            r#match: CliMatchMode::Nth,
            index: Some(0),
        })
        .expect_err("zero index invalid")
        .code,
        "CLI_MATCH_INDEX_INVALID"
    );
}
