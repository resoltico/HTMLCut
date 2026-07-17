use super::*;

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
