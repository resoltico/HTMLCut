use super::*;

#[test]
fn schema_and_catalog_renderers_cover_optional_surfaces() {
    let core_only_operation = CatalogOperationReport {
        operation_id: htmlcut_core::OperationId::DocumentParse,
        command: None,
        availability: CatalogAvailability::EngineOnly,
        summary: "Core-only parse".to_owned(),
        engine_capability: "parse_document(SourceRequest, RuntimeOptions)".to_owned(),
        request_contract: CatalogContractSurface {
            artifact: "SourceRequest + RuntimeOptions".to_owned(),
            schema_refs: Vec::new(),
        },
        result_contract: CatalogContractSurface {
            artifact: "ParseDocumentResult".to_owned(),
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
        engine_capability: "extract(ExtractionRequest{kind=selector}, RuntimeOptions)".to_owned(),
        request_contract: CatalogContractSurface {
            artifact: "ExtractionRequest + RuntimeOptions".to_owned(),
            schema_refs: vec![SchemaRefReport {
                schema_name: "htmlcut.extraction_request".to_owned(),
                schema_version: 2,
            }],
        },
        result_contract: CatalogContractSurface {
            artifact: "ExtractionResult".to_owned(),
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
    assert!(rendered_catalog.contains("document.parse | engine only"));
    assert!(
        rendered_catalog
            .contains("engine capability: parse_document(SourceRequest, RuntimeOptions)")
    );
    assert!(
        rendered_catalog
            .contains("Use `--output json` for parameters, defaults, constraints, and examples.")
    );

    let single_operation_catalog = CatalogCommandReport {
        operations: vec![CatalogOperationReport {
            operation_id: htmlcut_core::OperationId::SelectExtract,
            command: Some("select".to_owned()),
            availability: CatalogAvailability::Cli,
            summary: "Synthetic contract".to_owned(),
            engine_capability: "extract(ExtractionRequest{kind=selector}, RuntimeOptions)"
                .to_owned(),
            request_contract: CatalogContractSurface {
                artifact: "ExtractionRequest + RuntimeOptions".to_owned(),
                schema_refs: vec![SchemaRefReport {
                    schema_name: "htmlcut.extraction_request".to_owned(),
                    schema_version: 2,
                }],
            },
            result_contract: CatalogContractSurface {
                artifact: "ExtractionResult".to_owned(),
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
    assert!(rendered_single_catalog.contains("select"));
    assert!(
        rendered_single_catalog.contains(
            "engine capability: extract(ExtractionRequest{kind=selector}, RuntimeOptions)"
        )
    );
    assert!(rendered_single_catalog.contains("request: ExtractionRequest + RuntimeOptions"));
    assert!(rendered_single_catalog.contains("result: ExtractionResult"));
    assert!(
        rendered_single_catalog
            .contains("Use `--output json` for parameters, defaults, constraints, and examples.")
    );
    assert!(!rendered_single_catalog.contains("inputs: file | url | stdin"));
    assert!(!rendered_single_catalog.contains("default match: first"));

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
            surface: "tests".to_owned(),
            profile: None,
            artifact: "Synthetic".to_owned(),
            stability: htmlcut_core::SchemaStability::Versioned,
            json_schema: Value::String("not an object".to_owned()),
        }],
    };
    let rendered_single_schema = render_schema_text(&single_schema);
    assert!(rendered_single_schema.contains("Schema:"));
    assert!(rendered_single_schema.contains("synthetic.single@7 | tests | Synthetic | versioned"));
    assert!(rendered_single_schema.contains("json schema keys: (not-an-object)"));

    let multi_schema = SchemaCommandReport {
        schemas: vec![
            SchemaDocumentReport {
                schema_name: "synthetic.a".to_owned(),
                schema_version: 1,
                surface: "tests".to_owned(),
                profile: None,
                artifact: "A".to_owned(),
                stability: htmlcut_core::SchemaStability::Versioned,
                json_schema: serde_json::json!({ "type": "object" }),
            },
            SchemaDocumentReport {
                schema_name: "synthetic.b".to_owned(),
                schema_version: 2,
                surface: "tests".to_owned(),
                profile: None,
                artifact: "B".to_owned(),
                stability: htmlcut_core::SchemaStability::Versioned,
                json_schema: serde_json::json!({ "type": "object" }),
            },
        ],
        ..single_schema
    };
    let rendered_multi_schema = render_schema_text(&multi_schema);
    assert!(rendered_multi_schema.contains("Schemas:"));
    assert!(rendered_multi_schema.contains("synthetic.a@1 | tests | A | versioned"));
    assert!(rendered_multi_schema.contains("synthetic.b@2 | tests | B | versioned"));
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
            .any(|operation| operation.availability == CatalogAvailability::EngineOnly)
    );

    let text_outcome = run_schema(
        SchemaArgs {
            output: CliSchemaOutputMode::Text,
            output_file: None,
            file_write: default_output_file_write_args(),
            filter: crate::args::SchemaFilterArgs {
                name: Some(htmlcut_core::interop::v1::RESULT_SCHEMA_NAME.to_owned()),
                schema_version: Some(htmlcut_core::interop::v1::RESULT_SCHEMA_VERSION),
            },
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
            file_write: default_output_file_write_args(),
            filter: crate::args::SchemaFilterArgs {
                name: Some("synthetic.missing".to_owned()),
                schema_version: Some(99),
            },
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
            file_write: default_output_file_write_args(),
            filter: crate::args::SchemaFilterArgs {
                name: None,
                schema_version: Some(1),
            },
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

    let index_json_outcome = run_schema(
        SchemaArgs {
            output: CliSchemaOutputMode::IndexJson,
            output_file: None,
            file_write: default_output_file_write_args(),
            filter: crate::args::SchemaFilterArgs {
                name: Some(htmlcut_core::interop::v1::RESULT_SCHEMA_NAME.to_owned()),
                schema_version: Some(htmlcut_core::interop::v1::RESULT_SCHEMA_VERSION),
            },
        },
        0,
        false,
    );
    assert_eq!(index_json_outcome.exit_code, 0);
    assert!(
        index_json_outcome
            .stdout
            .as_deref()
            .is_some_and(|stdout| stdout.contains("\"schema_name\": \"htmlcut.result\""))
    );

    let tempdir = tempdir().expect("tempdir");
    let blocked_output = tempdir.path().join("inventory.json");
    fs::write(&blocked_output, "occupied").expect("blocked output");
    let json_path_error = run_schema(
        SchemaArgs {
            output: CliSchemaOutputMode::Json,
            output_file: Some(blocked_output.clone()),
            file_write: default_output_file_write_args(),
            filter: crate::args::SchemaFilterArgs {
                name: Some(htmlcut_core::EXTRACTION_REQUEST_SCHEMA_NAME.to_owned()),
                schema_version: Some(htmlcut_core::CORE_REQUEST_SCHEMA_VERSION),
            },
        },
        0,
        false,
    );
    assert_eq!(json_path_error.exit_code, EXIT_CODE_OUTPUT);
    assert!(
        json_path_error
            .stdout
            .as_deref()
            .is_some_and(|stdout| stdout.contains("CLI_OUTPUT_FILE_EXISTS"))
    );

    let index_json_path_error = run_schema(
        SchemaArgs {
            output: CliSchemaOutputMode::IndexJson,
            output_file: Some(blocked_output),
            file_write: default_output_file_write_args(),
            filter: crate::args::SchemaFilterArgs {
                name: None,
                schema_version: None,
            },
        },
        0,
        false,
    );
    assert_eq!(index_json_path_error.exit_code, EXIT_CODE_OUTPUT);
    assert!(
        index_json_path_error
            .stdout
            .as_deref()
            .is_some_and(|stdout| stdout.contains("CLI_OUTPUT_FILE_EXISTS"))
    );

    let source = build_source_request(&SourceArgs {
        input: Some("https://example.com/docs/page.html".to_owned()),
        base_url: Some("https://base.example/root/".to_owned()),
        max_bytes: DEFAULT_MAX_BYTES.to_string(),
        fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
        fetch_connect_timeout_ms: htmlcut_core::DEFAULT_FETCH_CONNECT_TIMEOUT_MS,
        tls_trust: CliTlsTrustMode::WebPki,
        tls_ca_bundle: None,
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
        fetch_connect_timeout_ms: htmlcut_core::DEFAULT_FETCH_CONNECT_TIMEOUT_MS,
        tls_trust: CliTlsTrustMode::WebPki,
        tls_ca_bundle: None,
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
        fetch_connect_timeout_ms: htmlcut_core::DEFAULT_FETCH_CONNECT_TIMEOUT_MS,
        tls_trust: CliTlsTrustMode::WebPki,
        tls_ca_bundle: None,
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
        build_schema_report(
            Some(htmlcut_core::interop::v1::RESULT_SCHEMA_NAME),
            Some(htmlcut_core::interop::v1::RESULT_SCHEMA_VERSION),
        )
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
    assert_cli_schema_catalog_for_tests(cli_schema_catalog_for_tests());
}

#[test]
fn cli_schema_catalog_guards_reject_drift() {
    let malformed = [htmlcut_core::SchemaDescriptor {
        schema_ref: htmlcut_core::SchemaRef::new(
            ERROR_COMMAND_REPORT_SCHEMA_NAME,
            ERROR_COMMAND_REPORT_SCHEMA_VERSION,
        ),
        owner: "wrong-owner",
        contract_family: "wrong-shape",
        stability: htmlcut_core::SchemaStability::Versioned,
        json_schema: || Ok(serde_json::json!({})),
    }];

    let errors = cli_schema_catalog_validation_errors_for_tests(&malformed);
    assert!(errors.iter().any(|error| error.contains("owner drifted")));
    let duplicate_errors =
        cli_schema_catalog_validation_errors_for_tests(&[malformed[0], malformed[0]]);
    assert!(
        duplicate_errors
            .iter()
            .any(|error| error.contains("appears more than once"))
    );

    let panic = std::panic::catch_unwind(|| {
        assert_cli_schema_catalog_for_tests(&malformed);
    })
    .expect_err("CLI schema assertion should reject drift");
    let panic_text = if let Some(message) = panic.downcast_ref::<String>() {
        message.clone()
    } else if let Some(message) = panic.downcast_ref::<&str>() {
        (*message).to_owned()
    } else {
        "<non-string panic>".to_owned()
    };
    assert!(panic_text.contains("cli schema catalog drifted"));

    let unknown_errors =
        cli_schema_catalog_validation_errors_for_tests(&[htmlcut_core::SchemaDescriptor {
            schema_ref: htmlcut_core::SchemaRef::new("htmlcut.unknown_report", 99),
            owner: "cli",
            contract_family: "unknown report",
            stability: htmlcut_core::SchemaStability::Versioned,
            json_schema: malformed[0].json_schema,
        }]);
    assert!(
        unknown_errors
            .iter()
            .any(|error| error.contains("is not part of the maintained CLI schema inventory"))
    );
}

#[test]
fn cli_schema_descriptor_constructor_preserves_fields() {
    fn synthetic_schema() -> Result<Value, htmlcut_core::SchemaExportError> {
        Ok(serde_json::json!({ "type": "object" }))
    }

    let descriptor = cli_schema_descriptor_for_tests(
        htmlcut_core::SchemaRef::new("htmlcut.synthetic_cli_report", 1),
        "synthetic cli report",
        synthetic_schema,
    );
    assert_eq!(
        descriptor.schema_ref,
        htmlcut_core::SchemaRef::new("htmlcut.synthetic_cli_report", 1)
    );
    assert_eq!(descriptor.owner, "cli");
    assert_eq!(descriptor.contract_family, "synthetic cli report");
    assert_eq!(
        descriptor.stability,
        htmlcut_core::SchemaStability::Versioned
    );
    assert_eq!(
        (descriptor.json_schema)().expect("synthetic schema")["type"],
        "object"
    );
}

#[test]
fn schema_export_errors_map_to_typed_cli_internal_errors() {
    let schema_ref = htmlcut_core::SchemaRef::new("htmlcut.synthetic_report", 7);
    let export_error = schema_export_serialize_error_for_tests(schema_ref);

    assert_eq!(
        export_error,
        htmlcut_core::SchemaExportError::Serialize {
            schema_name: "htmlcut.synthetic_report",
            schema_version: 7,
            message: "synthetic schema serialization failure".to_owned(),
        }
    );

    let cli_error = schema_export_error_for_tests(export_error.clone());
    assert_eq!(cli_error.code, "CLI_SCHEMA_EXPORT_FAILED");
    assert!(
        cli_error
            .message
            .contains("Could not serialize JSON schema htmlcut.synthetic_report@7")
    );
    assert!(
        cli_error
            .message
            .contains("synthetic schema serialization failure")
    );
}

#[test]
fn catalog_and_schema_commands_fall_back_to_human_errors_when_json_rendering_breaks() {
    let catalog_outcome = with_json_render_failure_for_tests(|| {
        run_catalog(
            CatalogArgs {
                output: CliCatalogOutputMode::Json,
                output_file: None,
                file_write: default_output_file_write_args(),
                filter: crate::args::CatalogFilterArgs { operation: None },
            },
            0,
            false,
        )
    });
    assert_eq!(catalog_outcome.exit_code, EXIT_CODE_INTERNAL);
    assert!(catalog_outcome.stdout.is_none());
    assert!(
        catalog_outcome
            .stderr
            .iter()
            .any(|line| line.contains("Could not render CLI JSON payload"))
    );

    let schema_outcome = with_json_render_failure_for_tests(|| {
        run_schema(
            SchemaArgs {
                output: CliSchemaOutputMode::Json,
                output_file: None,
                file_write: default_output_file_write_args(),
                filter: crate::args::SchemaFilterArgs {
                    name: Some(htmlcut_core::interop::v1::RESULT_SCHEMA_NAME.to_owned()),
                    schema_version: Some(htmlcut_core::interop::v1::RESULT_SCHEMA_VERSION),
                },
            },
            0,
            false,
        )
    });
    assert_eq!(schema_outcome.exit_code, EXIT_CODE_INTERNAL);
    assert!(schema_outcome.stdout.is_none());

    let schema_inventory_outcome = with_json_render_failure_for_tests(|| {
        run_schema(
            SchemaArgs {
                output: CliSchemaOutputMode::IndexJson,
                output_file: None,
                file_write: default_output_file_write_args(),
                filter: crate::args::SchemaFilterArgs {
                    name: Some(htmlcut_core::interop::v1::RESULT_SCHEMA_NAME.to_owned()),
                    schema_version: Some(htmlcut_core::interop::v1::RESULT_SCHEMA_VERSION),
                },
            },
            0,
            false,
        )
    });
    assert_eq!(schema_inventory_outcome.exit_code, EXIT_CODE_INTERNAL);
    assert!(schema_inventory_outcome.stdout.is_none());
}
