use super::*;

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
