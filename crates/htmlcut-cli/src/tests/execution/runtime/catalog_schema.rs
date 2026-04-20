use super::*;

#[test]
fn catalog_report_and_text_surface_core_operation_catalog() {
    let report = build_catalog_report(None).expect("catalog report");
    assert_eq!(report.tool, TOOL_NAME);
    assert_eq!(report.version, HTMLCUT_VERSION);
    assert_eq!(report.schema_name, CATALOG_REPORT_SCHEMA_NAME);
    assert_eq!(report.schema_version, crate::model::CATALOG_SCHEMA_VERSION);
    assert_eq!(
        report.schema_profile,
        htmlcut_core::HTMLCUT_JSON_SCHEMA_PROFILE
    );
    assert_eq!(report.description, HTMLCUT_DESCRIPTION);
    assert_eq!(report.command, "catalog");
    assert_eq!(
        report.operations.len(),
        htmlcut_core::operation_catalog().len()
    );
    assert_eq!(
        report.operations[0].operation_id,
        htmlcut_core::operation_catalog()[0].id
    );

    let text = render_catalog_text(&report);
    assert!(text.contains("Operations:"));
    assert!(text.contains("source.inspect | inspect source"));
    assert!(text.contains("document.parse | core only"));

    let filtered = build_catalog_report(Some("select.preview")).expect("filtered catalog");
    assert_eq!(filtered.operations.len(), 1);
    assert_eq!(
        filtered.operations[0].operation_id,
        htmlcut_core::OperationId::SelectPreview
    );
    assert_eq!(
        filtered.operations[0].core_surface,
        "preview_extraction(ExtractionRequest{kind=selector}, RuntimeOptions)"
    );
    assert_eq!(
        filtered.operations[0].request_contract.rust_shape,
        "ExtractionRequest + RuntimeOptions"
    );
    assert_eq!(
        filtered.operations[0].request_contract.schema_refs,
        vec![
            SchemaRefReport {
                schema_name: htmlcut_core::EXTRACTION_REQUEST_SCHEMA_NAME.to_owned(),
                schema_version: htmlcut_core::CORE_REQUEST_SCHEMA_VERSION,
            },
            SchemaRefReport {
                schema_name: htmlcut_core::RUNTIME_OPTIONS_SCHEMA_NAME.to_owned(),
                schema_version: htmlcut_core::CORE_REQUEST_SCHEMA_VERSION,
            },
        ]
    );
    assert_eq!(
        filtered.operations[0].result_contract.rust_shape,
        "ExtractionResult"
    );
    assert_eq!(
        filtered.operations[0].result_contract.schema_refs,
        vec![SchemaRefReport {
            schema_name: htmlcut_core::CORE_RESULT_SCHEMA_NAME.to_owned(),
            schema_version: htmlcut_core::CORE_RESULT_SCHEMA_VERSION,
        }]
    );
    let contract = filtered.operations[0]
        .command_contract
        .as_ref()
        .expect("filtered cli operation should expose a contract");
    assert_eq!(
        contract.invocation,
        "htmlcut inspect select [OPTIONS] --css <CSS> [INPUT]"
    );
    assert_eq!(contract.default_match.as_deref(), Some("first"));
    assert_eq!(contract.default_value.as_deref(), Some("structured"));
    assert_eq!(contract.default_output.as_deref(), Some("json"));
    assert!(contract.parameters.iter().any(|parameter| {
        parameter.name == "--css"
            && parameter.kind == crate::model::CatalogParameterKind::Option
            && parameter.requirement == crate::model::CatalogParameterRequirement::Conditional
            && parameter.requirement_note.as_deref()
                == Some("required unless --request-file is used")
    }));
    assert!(contract.parameters.iter().any(|parameter| {
        parameter.name == "--request-file"
            && parameter.kind == crate::model::CatalogParameterKind::Option
            && parameter.requirement == crate::model::CatalogParameterRequirement::Optional
    }));
    assert!(contract.parameters.iter().any(|parameter| {
        parameter.name == "--emit-request-file"
            && parameter.kind == crate::model::CatalogParameterKind::Option
            && parameter.requirement == crate::model::CatalogParameterRequirement::Optional
    }));
    assert!(contract.parameters.iter().any(|parameter| {
        parameter.name == "--index"
            && parameter.requirement == crate::model::CatalogParameterRequirement::Conditional
            && parameter.requirement_note.as_deref() == Some("required when --match nth is used")
    }));

    let error = build_catalog_report(Some("select.extrac")).expect_err("unknown op");
    assert_eq!(error.code, "CLI_OPERATION_ID_UNKNOWN");
    assert!(error.message.contains("Did you mean"));
    assert!(error.message.contains("`select.extract`"));
}

#[test]
fn schema_report_surfaces_core_cli_and_interop_contracts() {
    let report = build_schema_report(None, None).expect("schema report");
    assert_eq!(report.tool, TOOL_NAME);
    assert_eq!(report.version, HTMLCUT_VERSION);
    assert_eq!(report.schema_name, SCHEMA_COMMAND_REPORT_SCHEMA_NAME);
    assert_eq!(report.schema_version, SCHEMA_COMMAND_REPORT_SCHEMA_VERSION);
    assert_eq!(
        report.schema_profile,
        htmlcut_core::HTMLCUT_JSON_SCHEMA_PROFILE
    );
    assert!(report.schemas.iter().any(|schema| {
        schema.schema_name == htmlcut_core::EXTRACTION_REQUEST_SCHEMA_NAME
            && schema.schema_version == htmlcut_core::CORE_REQUEST_SCHEMA_VERSION
            && schema.owner_surface == "htmlcut-core"
    }));
    assert!(report.schemas.iter().any(|schema| {
        schema.schema_name == htmlcut_core::interop::v1::PLAN_SCHEMA_NAME
            && schema.owner_surface == "htmlcut_core::interop::v1"
            && schema.stability == htmlcut_core::SchemaStability::Frozen
    }));
    assert!(report.schemas.iter().any(|schema| {
        schema.schema_name == CATALOG_REPORT_SCHEMA_NAME && schema.owner_surface == "htmlcut-cli"
    }));

    let filtered = build_schema_report(Some("htmlcut.result"), Some(1)).expect("filtered schema");
    assert_eq!(filtered.schemas.len(), 1);
    assert_eq!(filtered.schemas[0].schema_name, "htmlcut.result");

    let error = build_schema_report(None, Some(1)).expect_err("version without name");
    assert_eq!(error.code, "CLI_SCHEMA_VERSION_REQUIRES_NAME");
    let version_error =
        build_schema_report(Some("htmlcut.result"), Some(99)).expect_err("unknown schema version");
    assert!(
        version_error
            .message
            .contains("Available versions for `htmlcut.result`: 1.")
    );
}
