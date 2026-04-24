use super::*;

#[test]
fn contract_lint_rendered_root_help_carries_manifest_identity_and_clap_commands() {
    let mut command = Cli::command();
    let rendered_help = render_long_help(&mut command);
    let subcommands = Cli::command()
        .get_subcommands()
        .map(|subcommand| {
            (
                subcommand.get_name().to_owned(),
                subcommand
                    .get_about()
                    .expect("top-level subcommand about")
                    .to_string(),
            )
        })
        .collect::<Vec<_>>();

    assert!(
        rendered_help.starts_with(&format!(
            "{DISPLAY_NAME} {HTMLCUT_VERSION}\n{HTMLCUT_DESCRIPTION}\n"
        )),
        "root help drifted package banner: {rendered_help}"
    );
    let usage_index = rendered_help
        .find("Usage: htmlcut [OPTIONS] <COMMAND>")
        .expect("root help usage");
    let start_here_index = rendered_help.find("Start here:").expect("root help flow");
    assert!(
        usage_index < start_here_index,
        "root help should present usage before workflow detail: {rendered_help}"
    );
    assert!(
        crate::help::root_long_about().contains("Start here:"),
        "root help lost the workflow opener: {}",
        crate::help::root_long_about()
    );
    assert!(
        crate::help::root_long_about().contains("Reusable requests:"),
        "root help lost reusable request guidance: {}",
        crate::help::root_long_about()
    );

    for (name, about) in subcommands {
        assert!(
            rendered_help.contains(&name),
            "root help drifted subcommand name {name}: {rendered_help}"
        );
        assert!(
            rendered_help.contains(&about),
            "root help drifted subcommand summary {about:?}: {rendered_help}"
        );
    }
}

#[test]
fn contract_lint_clap_help_summaries_match_core_help_contracts() {
    let command = Cli::command();
    let catalog = command
        .get_subcommands()
        .find(|subcommand| subcommand.get_name() == "catalog")
        .expect("catalog command");
    assert_eq!(
        catalog.get_about().expect("catalog about").to_string(),
        htmlcut_core::cli_contract::cli_aux_command_descriptor(
            htmlcut_core::cli_contract::CliAuxCommandId::Catalog
        )
        .about
    );

    let schema = command
        .get_subcommands()
        .find(|subcommand| subcommand.get_name() == "schema")
        .expect("schema command");
    assert_eq!(
        schema.get_about().expect("schema about").to_string(),
        htmlcut_core::cli_contract::cli_aux_command_descriptor(
            htmlcut_core::cli_contract::CliAuxCommandId::Schema
        )
        .about
    );

    let inspect = command
        .get_subcommands()
        .find(|subcommand| subcommand.get_name() == "inspect")
        .expect("inspect command");
    assert_eq!(
        inspect.get_about().expect("inspect about").to_string(),
        htmlcut_core::cli_contract::cli_aux_command_descriptor(
            htmlcut_core::cli_contract::CliAuxCommandId::Inspect
        )
        .about
    );

    for (command_path, operation_id) in [
        (&["select"][..], htmlcut_core::OperationId::SelectExtract),
        (&["slice"][..], htmlcut_core::OperationId::SliceExtract),
        (
            &["inspect", "source"][..],
            htmlcut_core::OperationId::SourceInspect,
        ),
        (
            &["inspect", "select"][..],
            htmlcut_core::OperationId::SelectPreview,
        ),
        (
            &["inspect", "slice"][..],
            htmlcut_core::OperationId::SlicePreview,
        ),
    ] {
        let help_command = command_for_path(command_path);
        assert_eq!(
            help_command
                .get_about()
                .expect("operation about")
                .to_string(),
            htmlcut_core::operation_descriptor(operation_id).description
        );
    }
}

#[test]
fn contract_lint_rendered_help_catalog_and_error_surfaces_reference_registered_contracts() {
    let known_schemas = known_schema_names();
    let known_operations = known_operation_ids();
    let schema_report = build_schema_report(None, None).expect("schema report");
    let invalid_definition_dir = tempdir().expect("tempdir");
    let invalid_json_path = invalid_definition_dir.path().join("invalid.json");
    fs::write(&invalid_json_path, "{").expect("write invalid json");
    let invalid_json_error = load_extraction_definition_for_tests(
        &invalid_json_path,
        ExtractionStrategy::Selector,
        "select",
    )
    .expect_err("invalid request file should fail");

    let unsupported_schema_path = invalid_definition_dir.path().join("unsupported.json");
    fs::write(
        &unsupported_schema_path,
        r#"{
  "schema_name": "not_a_schema",
  "schema_version": 1,
  "request": {
    "spec_version": 4,
    "source": {"input": "-"},
    "extraction": {"selector": "a"}
  },
  "runtime": {}
}"#,
    )
    .expect("write unsupported schema");
    let unsupported_schema_error = load_extraction_definition_for_tests(
        &unsupported_schema_path,
        ExtractionStrategy::Selector,
        "select",
    )
    .expect_err("unsupported request schema should fail");

    let surfaces = vec![
        (
            "root help".to_owned(),
            render_long_help(&mut Cli::command()),
        ),
        (
            "catalog help".to_owned(),
            render_long_help(&mut command_for_path(&["catalog"])),
        ),
        (
            "schema help".to_owned(),
            render_long_help(&mut command_for_path(&["schema"])),
        ),
        (
            "inspect help".to_owned(),
            render_long_help(&mut command_for_path(&["inspect"])),
        ),
        (
            "inspect source help".to_owned(),
            render_long_help(&mut command_for_path(&["inspect", "source"])),
        ),
        (
            "inspect select help".to_owned(),
            render_long_help(&mut command_for_path(&["inspect", "select"])),
        ),
        (
            "inspect slice help".to_owned(),
            render_long_help(&mut command_for_path(&["inspect", "slice"])),
        ),
        (
            "select help".to_owned(),
            render_long_help(&mut command_for_path(&["select"])),
        ),
        (
            "slice help".to_owned(),
            render_long_help(&mut command_for_path(&["slice"])),
        ),
        (
            "catalog text".to_owned(),
            render_catalog_text(&build_catalog_report(None).expect("catalog report")),
        ),
        ("schema text".to_owned(), render_schema_text(&schema_report)),
        (
            "unknown operation error".to_owned(),
            crate::lookup::unknown_operation_id_error("bad-op").message,
        ),
        (
            "unknown schema error".to_owned(),
            crate::lookup::unknown_schema_error("not-a-schema", None, &schema_report.schemas)
                .message,
        ),
        (
            "invalid request-file error".to_owned(),
            invalid_json_error.message,
        ),
        (
            "unsupported request-file schema error".to_owned(),
            unsupported_schema_error.message,
        ),
    ];

    for (label, text) in surfaces {
        assert_surface_identifiers_registered(&label, &text, &known_schemas, &known_operations);
    }
}

#[cfg(unix)]
#[test]
fn cli_choice_parser_rejects_invalid_utf8_and_invalid_values() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let parser = crate::args::cli_choice_parser::<CliValueMode>();
    let command = crate::command();
    let argument = clap::Arg::new("value");

    let utf8_error = parser
        .parse_ref(&command, Some(&argument), OsStr::from_bytes(b"\xFF"))
        .expect_err("invalid UTF-8 should fail");
    assert_eq!(utf8_error.kind(), clap::error::ErrorKind::InvalidUtf8);
    assert!(utf8_error.to_string().contains("valid UTF-8"));

    let value_error = parser
        .parse_ref(&command, Some(&argument), OsStr::new("bogus"))
        .expect_err("invalid value should fail");
    assert_eq!(value_error.kind(), clap::error::ErrorKind::InvalidValue);
    assert!(
        value_error
            .to_string()
            .contains("invalid value 'bogus' for value")
    );
    assert!(value_error.to_string().contains("possible values:"));

    let anonymous_error = parser
        .parse_ref(&command, None, OsStr::new("bogus"))
        .expect_err("invalid anonymous value should fail");
    assert_eq!(anonymous_error.kind(), clap::error::ErrorKind::InvalidValue);
    assert!(
        anonymous_error
            .to_string()
            .contains("invalid value 'bogus'")
    );
    assert!(!anonymous_error.to_string().contains("for value"));
}

#[test]
fn help_renderers_cover_numbered_sections_and_multi_value_output_overrides() {
    let section = htmlcut_core::cli_contract::CliHelpSection {
        title: "Steps".to_owned(),
        style: htmlcut_core::cli_contract::CliHelpSectionStyle::Numbered,
        lines: vec!["first".to_owned(), "second".to_owned()],
    };
    assert_eq!(
        crate::help::render_help_section_for_tests(&section),
        "Steps:\n1. first\n2. second"
    );

    let mut contract = htmlcut_core::cli_contract::cli_operation_contract(
        htmlcut_core::OperationId::SelectExtract,
    )
    .expect("select extract contract")
    .clone();
    contract.default_output_overrides = vec![htmlcut_core::cli_contract::CliConditionalDefault {
        value: htmlcut_core::cli_contract::CliValue::OutputMode(
            htmlcut_core::cli_contract::CliOutputMode::Json,
        ),
        when: htmlcut_core::cli_contract::CliCondition {
            parameter: htmlcut_core::cli_contract::CliParameterId::Value,
            values: vec![
                htmlcut_core::cli_contract::CliValue::ValueType(
                    htmlcut_core::ValueType::Structured,
                ),
                htmlcut_core::cli_contract::CliValue::ValueType(htmlcut_core::ValueType::OuterHtml),
            ],
        },
    }];

    let summary = crate::help::render_contract_mode_summary_for_tests(&contract);
    assert!(summary.contains("Default output mode: text."));
    assert!(summary.contains("Supported output modes: text, html, json, none."));
    assert!(
        summary.contains(
            "Output default override: json when --value is one of structured, outer-html."
        )
    );

    let empty_summary = crate::help::build_operation_long_about_from_parts_for_tests(
        Vec::new(),
        &htmlcut_core::cli_contract::OperationCliContract {
            notes: Vec::new(),
            output_modes: Vec::new(),
            default_output: None,
            default_output_overrides: Vec::new(),
            selection_modes: Vec::new(),
            default_match: None,
            value_modes: Vec::new(),
            default_value: None,
            ..contract.clone()
        },
    );
    assert_eq!(empty_summary, "");

    assert!(crate::help::select_long_about().contains("Notes:"));
    assert!(crate::help::inspect_source_long_about().contains("Modes:"));
}
