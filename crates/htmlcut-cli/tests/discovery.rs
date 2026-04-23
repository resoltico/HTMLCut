mod support;
use support::*;

#[test]
fn catalog_json_surfaces_operation_catalog() {
    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    let report = parse_catalog_report(
        command
            .args(["catalog", "--output", "json"])
            .assert()
            .success(),
    );

    assert_eq!(report.tool, "htmlcut");
    assert_eq!(report.version, expected_version());
    assert_eq!(report.schema_name, CATALOG_REPORT_SCHEMA_NAME);
    assert_eq!(report.schema_version, CATALOG_SCHEMA_VERSION);
    assert_eq!(
        report.schema_profile,
        htmlcut_core::HTMLCUT_JSON_SCHEMA_PROFILE
    );
    assert_eq!(report.description, env!("CARGO_PKG_DESCRIPTION"));
    assert_eq!(report.command, "catalog");
    assert_eq!(
        report.operations.len(),
        htmlcut_core::operation_catalog().len()
    );
    assert_eq!(
        report.operations[0].operation_id,
        htmlcut_core::operation_catalog()[0].id
    );
    assert_eq!(
        report.operations[0].core_surface,
        htmlcut_core::operation_catalog()[0].core_surface
    );
    assert_eq!(
        report.operations[0].request_contract.rust_shape,
        htmlcut_core::operation_catalog()[0]
            .request_contract
            .rust_shape
    );
    assert_eq!(
        report.operations[0].result_contract.rust_shape,
        htmlcut_core::operation_catalog()[0]
            .result_contract
            .rust_shape
    );
    assert!(report.operations[0].command_contract.is_none());

    let select_extract = report
        .operations
        .iter()
        .find(|operation| operation.operation_id == htmlcut_core::OperationId::SelectExtract)
        .expect("select.extract should be cataloged");
    let command_contract = select_extract
        .command_contract
        .as_ref()
        .expect("cli operation should expose a command contract");
    assert_eq!(
        command_contract.invocation,
        "htmlcut select [OPTIONS] --css <CSS> [INPUT]"
    );
    assert_eq!(command_contract.default_match.as_deref(), Some("first"));
    assert_eq!(command_contract.default_value.as_deref(), Some("text"));
    assert_eq!(command_contract.default_output.as_deref(), Some("text"));
    assert_eq!(command_contract.default_output_overrides.len(), 2);
    assert_eq!(command_contract.default_output_overrides[0].value, "html");
    assert_eq!(
        command_contract.default_output_overrides[0].when.parameter,
        "--value"
    );
    assert_eq!(
        command_contract.default_output_overrides[0].when.values,
        vec!["inner-html".to_owned(), "outer-html".to_owned()]
    );
    assert_eq!(command_contract.default_output_overrides[1].value, "json");
    assert_eq!(
        command_contract.default_output_overrides[1].when.parameter,
        "--value"
    );
    assert_eq!(
        command_contract.default_output_overrides[1].when.values,
        vec!["structured".to_owned()]
    );
    assert!(
        command_contract
            .constraints
            .iter()
            .any(|constraint| matches!(
                constraint,
                htmlcut_cli::CatalogConstraint::RequiresParameter { parameter, when }
                    if parameter == "--bundle"
                        && when.parameter == "--output"
                        && when.values == vec!["none".to_owned()]
            ))
    );
    assert!(command_contract.parameters.iter().any(|parameter| {
        parameter.name == "--css"
            && parameter.requirement == htmlcut_cli::CatalogParameterRequirement::Conditional
            && parameter.requirement_note.as_deref()
                == Some("required unless --request-file is used")
    }));
    assert!(command_contract.parameters.iter().any(|parameter| {
        parameter.name == "--request-file"
            && parameter.requirement == htmlcut_cli::CatalogParameterRequirement::Optional
    }));
    assert!(command_contract.parameters.iter().any(|parameter| {
        parameter.name == "--fetch-preflight"
            && parameter.requirement == htmlcut_cli::CatalogParameterRequirement::Optional
    }));
    assert!(command_contract.parameters.iter().any(|parameter| {
        parameter.name == "--output-file"
            && parameter.requirement == htmlcut_cli::CatalogParameterRequirement::Optional
    }));
    assert!(command_contract.parameters.iter().any(|parameter| {
        parameter.name == "--attribute"
            && parameter.requirement == htmlcut_cli::CatalogParameterRequirement::Conditional
            && parameter.requirement_note.as_deref()
                == Some("required when --value attribute is used")
    }));
    assert!(command_contract.notes.iter().any(|note| {
        note.contains("Structured extraction only supports --output json or --output none.")
    }));
}

#[test]
fn schema_json_surfaces_registry_for_core_cli_and_interop() {
    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    let report = parse_schema_report(
        command
            .args(["schema", "--output", "json"])
            .assert()
            .success(),
    );

    assert_eq!(report.tool, "htmlcut");
    assert_eq!(report.version, expected_version());
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
        schema.schema_name == htmlcut_core::interop::v1::RESULT_SCHEMA_NAME
            && schema.owner_surface == "htmlcut_core::interop::v1"
    }));
    assert!(report.schemas.iter().any(|schema| {
        schema.schema_name == CATALOG_REPORT_SCHEMA_NAME && schema.owner_surface == "htmlcut-cli"
    }));
}

#[test]
fn inspect_source_directory_input_reports_directory_specific_failure() {
    let tempdir = tempdir().expect("tempdir");
    let input_dir = tempdir.path().join("input dir");
    fs::create_dir_all(&input_dir).expect("create dir");

    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    command
        .args(["inspect", "source"])
        .arg(&input_dir)
        .args(["--output", "text"])
        .assert()
        .failure()
        .code(3)
        .stderr(predicate::str::contains(
            "Input path is a directory, not a file:",
        ));
}

#[test]
fn inspect_source_invalid_utf8_input_reports_utf8_failure() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = tempdir.path().join("bad.bin");
    fs::write(&input_path, [0xff, 0xfe]).expect("write invalid utf8");

    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    command
        .args(["inspect", "source"])
        .arg(&input_path)
        .args(["--output", "text"])
        .assert()
        .failure()
        .code(3)
        .stderr(predicate::str::contains("File is not valid UTF-8:"));
}
