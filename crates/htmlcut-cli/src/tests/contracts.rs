use super::*;
use regex::Regex;

#[test]
fn raw_args_prefers_json_tracks_output_and_inspect_modes() {
    assert!(raw_args_prefers_json(&[
        "htmlcut".to_owned(),
        "inspect".to_owned(),
        "source".to_owned(),
        "page.html".to_owned(),
    ]));
    assert!(raw_args_prefers_json(&[
        "htmlcut".to_owned(),
        "select".to_owned(),
        "page.html".to_owned(),
        "--value".to_owned(),
        "structured".to_owned(),
    ]));
    assert!(!raw_args_prefers_json(&[
        "htmlcut".to_owned(),
        "inspect".to_owned(),
        "source".to_owned(),
        "page.html".to_owned(),
        "--output".to_owned(),
        "text".to_owned(),
    ]));
}

#[test]
fn raw_arg_helpers_detect_global_help_and_version_anywhere() {
    assert!(raw_args_requests_version(&[
        "htmlcut".to_owned(),
        "select".to_owned(),
        "--version".to_owned(),
    ]));
    assert!(raw_args_requests_version(&[
        "htmlcut".to_owned(),
        "inspect".to_owned(),
        "source".to_owned(),
        "-V".to_owned(),
    ]));
    assert!(raw_args_requests_help(&[
        "htmlcut".to_owned(),
        "slice".to_owned(),
        "--help".to_owned(),
    ]));
    assert!(!raw_args_requests_help(&[
        "htmlcut".to_owned(),
        "catalog".to_owned(),
    ]));
    assert!(!raw_args_requests_version(&[
        "htmlcut".to_owned(),
        "--".to_owned(),
        "--version".to_owned(),
    ]));
}

#[test]
fn command_name_from_raw_args_recognizes_nested_commands() {
    assert_eq!(
        command_name_from_raw_args(&["htmlcut".to_owned()]),
        "htmlcut"
    );
    assert_eq!(
        command_name_from_raw_args(&[
            "htmlcut".to_owned(),
            "inspect".to_owned(),
            "source".to_owned(),
        ]),
        "inspect-source"
    );
    assert_eq!(
        command_name_from_raw_args(&[
            "htmlcut".to_owned(),
            "inspect".to_owned(),
            "select".to_owned(),
        ]),
        "inspect-select"
    );
    assert_eq!(
        command_name_from_raw_args(&[
            "htmlcut".to_owned(),
            "inspect".to_owned(),
            "slice".to_owned(),
        ]),
        "inspect-slice"
    );
    assert_eq!(
        command_name_from_raw_args(&[
            "htmlcut".to_owned(),
            "-vv".to_owned(),
            "inspect".to_owned(),
            "slice".to_owned(),
            "page.html".to_owned(),
        ]),
        "inspect-slice"
    );
    assert_eq!(
        command_name_from_raw_args(&[
            "htmlcut".to_owned(),
            "--quiet".to_owned(),
            "select".to_owned(),
            "-".to_owned(),
        ]),
        "select"
    );
    assert_eq!(
        command_name_from_raw_args(&["htmlcut".to_owned(), "inspect".to_owned()]),
        "inspect"
    );
    assert_eq!(
        command_name_from_raw_args(&["htmlcut".to_owned(), "select".to_owned()]),
        "select"
    );
    assert_eq!(
        command_name_from_raw_args(&["htmlcut".to_owned(), "slice".to_owned()]),
        "slice"
    );
    assert_eq!(
        command_name_from_raw_args(&["htmlcut".to_owned(), "mystery".to_owned()]),
        "mystery"
    );

    let multi_value_condition = htmlcut_core::CliCondition {
        parameter: htmlcut_core::CliParameterId::Output,
        values: vec![
            htmlcut_core::CliValue::OutputMode(htmlcut_core::CliOutputMode::Json),
            htmlcut_core::CliValue::OutputMode(htmlcut_core::CliOutputMode::None),
        ],
    };
    assert_eq!(
        crate::prepare::render_condition_expression_for_tests(&multi_value_condition),
        "--output is one of json, none"
    );
}

#[test]
fn contract_lint_clap_choice_parsers_match_core_contract_domains() {
    let select_extract =
        htmlcut_core::cli_operation_contract(htmlcut_core::OperationId::SelectExtract)
            .expect("select extract contract");
    let select_preview =
        htmlcut_core::cli_operation_contract(htmlcut_core::OperationId::SelectPreview)
            .expect("select preview contract");

    assert_eq!(
        parser_value_names(crate::args::cli_choice_parser::<CliMatchMode>()),
        select_extract
            .selection_modes
            .iter()
            .copied()
            .map(|mode| {
                htmlcut_core::render_cli_value(htmlcut_core::CliValue::SelectionMode(mode))
            })
            .collect::<Vec<_>>()
    );
    assert_eq!(
        parser_value_names(crate::args::cli_choice_parser::<CliValueMode>()),
        select_extract
            .value_modes
            .iter()
            .copied()
            .map(|value| htmlcut_core::render_cli_value(htmlcut_core::CliValue::ValueType(value)))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        parser_value_names(crate::args::cli_choice_parser::<CliOutputMode>()),
        select_extract
            .output_modes
            .iter()
            .copied()
            .map(|mode| htmlcut_core::render_cli_value(htmlcut_core::CliValue::OutputMode(mode)))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        parser_value_names(crate::args::cli_choice_subset_parser(
            crate::args::TEXT_JSON_OUTPUT_MODES,
        )),
        select_preview
            .output_modes
            .iter()
            .copied()
            .map(|mode| htmlcut_core::render_cli_value(htmlcut_core::CliValue::OutputMode(mode)))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        parser_value_names(crate::args::cli_choice_subset_parser(
            crate::args::TEXT_JSON_OUTPUT_MODES,
        )),
        vec!["text".to_owned(), "json".to_owned()]
    );
    assert_eq!(
        parser_value_names(crate::args::cli_choice_subset_parser(
            crate::args::TEXT_JSON_OUTPUT_MODES,
        )),
        vec!["text".to_owned(), "json".to_owned()]
    );
    assert_eq!(
        parser_value_names(crate::args::cli_choice_parser::<CliWhitespaceMode>()),
        vec![
            htmlcut_core::render_cli_value(htmlcut_core::CliValue::WhitespaceMode(
                WhitespaceMode::Preserve,
            )),
            htmlcut_core::render_cli_value(htmlcut_core::CliValue::WhitespaceMode(
                WhitespaceMode::Normalize,
            )),
        ]
    );
    assert_eq!(
        parser_value_names(crate::args::cli_choice_parser::<CliPatternMode>()),
        vec![
            htmlcut_core::render_cli_value(htmlcut_core::CliValue::PatternMode(
                PatternMode::Literal,
            )),
            htmlcut_core::render_cli_value(
                htmlcut_core::CliValue::PatternMode(PatternMode::Regex,)
            ),
        ]
    );
    assert_eq!(
        parser_value_names(crate::args::cli_choice_parser::<CliFetchPreflightMode>()),
        vec![
            htmlcut_core::render_cli_value(htmlcut_core::CliValue::FetchPreflightMode(
                FetchPreflightMode::HeadFirst,
            )),
            htmlcut_core::render_cli_value(htmlcut_core::CliValue::FetchPreflightMode(
                FetchPreflightMode::GetOnly,
            )),
        ]
    );
}

#[test]
fn contract_lint_help_and_catalog_examples_reference_registered_contracts() {
    let known_schemas = known_schema_names();
    let mut examples = vec![
        crate::help::root_after_help(),
        crate::help::catalog_after_help(),
        crate::help::schema_after_help(),
        crate::help::select_after_help(),
        crate::help::slice_after_help(),
        crate::help::inspect_source_after_help(),
        crate::help::inspect_select_after_help(),
        crate::help::inspect_slice_after_help(),
    ]
    .into_iter()
    .flat_map(|help| {
        help.lines()
            .map(str::trim)
            .filter(|line| line.starts_with("htmlcut "))
            .map(str::to_owned)
            .collect::<Vec<_>>()
    })
    .collect::<Vec<_>>();
    examples.extend(
        htmlcut_core::cli_operation_catalog()
            .iter()
            .flat_map(|contract| {
                contract
                    .examples
                    .iter()
                    .map(|example| (*example).to_owned())
            }),
    );

    for example in examples {
        let tokens = shell_words(&example);
        assert_eq!(tokens.first().map(String::as_str), Some("htmlcut"));
        let top_level = tokens.get(1).map(String::as_str).expect("command");

        match top_level {
            "catalog" => {
                if let Some(operation_id) = option_value(&tokens, "--operation") {
                    operation_id
                        .parse::<htmlcut_core::OperationId>()
                        .expect("registered catalog operation id");
                }
            }
            "schema" => {
                if let Some(schema_name) = option_value(&tokens, "--name") {
                    assert!(
                        known_schemas.contains(schema_name),
                        "unknown schema {schema_name} in {example}"
                    );
                }
            }
            "inspect" | "select" | "slice" => {
                let command_path = if top_level == "inspect" {
                    vec![
                        "inspect",
                        tokens
                            .get(2)
                            .map(String::as_str)
                            .expect("inspect subcommand"),
                    ]
                } else {
                    vec![top_level]
                };
                let contract = htmlcut_core::find_cli_operation_by_command_path(&command_path)
                    .expect("registered operation example");
                assert_eq!(
                    command_name_from_raw_args(&tokens),
                    contract.report_command(),
                    "report command drift for {example}"
                );

                if let Some(value) = option_value(&tokens, "--match") {
                    assert!(
                        parameter_allowed_values(contract, htmlcut_core::CliParameterId::Match)
                            .contains(&value.to_owned()),
                        "unsupported --match {value} in {example}"
                    );
                }
                if let Some(value) = option_value(&tokens, "--value") {
                    assert!(
                        parameter_allowed_values(contract, htmlcut_core::CliParameterId::Value)
                            .contains(&value.to_owned()),
                        "unsupported --value {value} in {example}"
                    );
                }
                if let Some(value) = option_value(&tokens, "--output") {
                    assert!(
                        parameter_allowed_values(contract, htmlcut_core::CliParameterId::Output)
                            .contains(&value.to_owned()),
                        "unsupported --output {value} in {example}"
                    );
                }
                if let Some(value) = option_value(&tokens, "--pattern") {
                    assert!(
                        parameter_allowed_values(contract, htmlcut_core::CliParameterId::Pattern)
                            .contains(&value.to_owned()),
                        "unsupported --pattern {value} in {example}"
                    );
                }
                if let Some(value) = option_value(&tokens, "--fetch-preflight") {
                    assert!(
                        parameter_allowed_values(
                            contract,
                            htmlcut_core::CliParameterId::FetchPreflight,
                        )
                        .contains(&value.to_owned()),
                        "unsupported --fetch-preflight {value} in {example}"
                    );
                }
            }
            other => panic!("unexpected help example command {other}"),
        }
    }
}

#[test]
fn contract_lint_clap_defaults_and_command_surfaces_match_core_contracts() {
    let command = Cli::command();
    assert_command_path_registered(&command, &["catalog"]);
    assert_command_path_registered(&command, &["schema"]);

    let source_inspect =
        htmlcut_core::cli_operation_contract(htmlcut_core::OperationId::SourceInspect)
            .expect("source inspect contract");
    let select_extract =
        htmlcut_core::cli_operation_contract(htmlcut_core::OperationId::SelectExtract)
            .expect("select extract contract");
    let slice_extract =
        htmlcut_core::cli_operation_contract(htmlcut_core::OperationId::SliceExtract)
            .expect("slice extract contract");
    let select_preview =
        htmlcut_core::cli_operation_contract(htmlcut_core::OperationId::SelectPreview)
            .expect("select preview contract");
    let slice_preview =
        htmlcut_core::cli_operation_contract(htmlcut_core::OperationId::SlicePreview)
            .expect("slice preview contract");

    for contract in [
        source_inspect,
        select_extract,
        slice_extract,
        select_preview,
        slice_preview,
    ] {
        assert_command_path_registered(&command, contract.command_path);
    }

    let select_args = match Cli::try_parse_from(["htmlcut", "select", "page.html", "--css", "a"]) {
        Ok(Cli {
            command: Commands::Select(args),
            ..
        }) => args,
        other => panic!("unexpected select parse result {other:?}"),
    };
    assert_eq!(select_args.source.max_bytes, DEFAULT_MAX_BYTES.to_string());
    assert_eq!(
        select_args.source.max_bytes,
        parameter_default_value(select_extract, htmlcut_core::CliParameterId::MaxBytes)
            .expect("select max-bytes default"),
    );
    assert_eq!(
        select_args.source.fetch_timeout_ms,
        DEFAULT_FETCH_TIMEOUT_MS
    );
    assert_eq!(
        select_args.source.fetch_timeout_ms.to_string(),
        parameter_default_value(select_extract, htmlcut_core::CliParameterId::FetchTimeoutMs)
            .expect("select fetch-timeout default"),
    );
    assert_eq!(
        select_args.source.fetch_preflight.to_string(),
        parameter_default_value(select_extract, htmlcut_core::CliParameterId::FetchPreflight)
            .expect("select fetch-preflight default"),
    );
    assert_eq!(
        select_args.selection.r#match.to_string(),
        select_extract
            .default_match
            .map(|value| {
                htmlcut_core::render_cli_value(htmlcut_core::CliValue::SelectionMode(value))
            })
            .expect("select default match"),
    );
    assert_eq!(
        select_args.output.value.to_string(),
        select_extract
            .default_value
            .map(|value| htmlcut_core::render_cli_value(htmlcut_core::CliValue::ValueType(value)))
            .expect("select default value"),
    );
    assert_eq!(
        select_args.output.whitespace.to_string(),
        parameter_default_value(select_extract, htmlcut_core::CliParameterId::Whitespace)
            .expect("select whitespace default"),
    );
    assert_eq!(select_args.output.preview_chars, DEFAULT_PREVIEW_CHARS);
    assert_eq!(
        select_args.output.preview_chars.to_string(),
        parameter_default_value(select_extract, htmlcut_core::CliParameterId::PreviewChars)
            .expect("select preview-chars default"),
    );

    let slice_args = match Cli::try_parse_from([
        "htmlcut",
        "slice",
        "page.html",
        "--from",
        "<a>",
        "--to",
        "</a>",
    ]) {
        Ok(Cli {
            command: Commands::Slice(args),
            ..
        }) => args,
        other => panic!("unexpected slice parse result {other:?}"),
    };
    assert_eq!(
        slice_args.pattern.to_string(),
        parameter_default_value(slice_extract, htmlcut_core::CliParameterId::Pattern)
            .expect("slice pattern default"),
    );
    assert_eq!(
        slice_args.selection.r#match.to_string(),
        slice_extract
            .default_match
            .map(|value| {
                htmlcut_core::render_cli_value(htmlcut_core::CliValue::SelectionMode(value))
            })
            .expect("slice default match"),
    );
    assert_eq!(
        slice_args.output.value.to_string(),
        slice_extract
            .default_value
            .map(|value| htmlcut_core::render_cli_value(htmlcut_core::CliValue::ValueType(value)))
            .expect("slice default value"),
    );
    assert_eq!(
        slice_args.output.whitespace.to_string(),
        parameter_default_value(slice_extract, htmlcut_core::CliParameterId::Whitespace)
            .expect("slice whitespace default"),
    );

    let inspect_source_args =
        match Cli::try_parse_from(["htmlcut", "inspect", "source", "page.html"]) {
            Ok(Cli {
                command:
                    Commands::Inspect(InspectArgs {
                        command: InspectCommands::Source(args),
                    }),
                ..
            }) => args,
            other => panic!("unexpected inspect source parse result {other:?}"),
        };
    assert_eq!(
        inspect_source_args.sample_limit,
        DEFAULT_INSPECTION_SAMPLE_LIMIT
    );
    assert_eq!(
        inspect_source_args.sample_limit.to_string(),
        parameter_default_value(source_inspect, htmlcut_core::CliParameterId::SampleLimit)
            .expect("inspect source sample-limit default"),
    );
    assert_eq!(
        inspect_source_args.output.to_string(),
        source_inspect
            .default_output
            .map(|value| {
                htmlcut_core::render_cli_value(htmlcut_core::CliValue::OutputMode(value))
            })
            .expect("inspect source default output"),
    );
    assert_eq!(inspect_source_args.preview_chars, DEFAULT_PREVIEW_CHARS);
    assert_eq!(
        inspect_source_args.preview_chars.to_string(),
        parameter_default_value(source_inspect, htmlcut_core::CliParameterId::PreviewChars)
            .expect("inspect source preview-chars default"),
    );
    assert_eq!(
        inspect_source_args.source.fetch_preflight.to_string(),
        parameter_default_value(source_inspect, htmlcut_core::CliParameterId::FetchPreflight)
            .expect("inspect source fetch-preflight default"),
    );

    let inspect_select_args =
        match Cli::try_parse_from(["htmlcut", "inspect", "select", "page.html", "--css", "a"]) {
            Ok(Cli {
                command:
                    Commands::Inspect(InspectArgs {
                        command: InspectCommands::Select(args),
                    }),
                ..
            }) => args,
            other => panic!("unexpected inspect select parse result {other:?}"),
        };
    assert_eq!(
        inspect_select_args.selection.r#match.to_string(),
        select_preview
            .default_match
            .map(|value| {
                htmlcut_core::render_cli_value(htmlcut_core::CliValue::SelectionMode(value))
            })
            .expect("inspect select default match"),
    );
    assert_eq!(
        inspect_select_args.whitespace.to_string(),
        parameter_default_value(select_preview, htmlcut_core::CliParameterId::Whitespace)
            .expect("inspect select whitespace default"),
    );
    assert_eq!(
        inspect_select_args.output.output.to_string(),
        select_preview
            .default_output
            .map(|value| {
                htmlcut_core::render_cli_value(htmlcut_core::CliValue::OutputMode(value))
            })
            .expect("inspect select default output"),
    );
    assert_eq!(
        inspect_select_args.output.preview_chars,
        DEFAULT_PREVIEW_CHARS
    );

    let inspect_slice_args = match Cli::try_parse_from([
        "htmlcut",
        "inspect",
        "slice",
        "page.html",
        "--from",
        "<a>",
        "--to",
        "</a>",
    ]) {
        Ok(Cli {
            command:
                Commands::Inspect(InspectArgs {
                    command: InspectCommands::Slice(args),
                }),
            ..
        }) => args,
        other => panic!("unexpected inspect slice parse result {other:?}"),
    };
    assert_eq!(
        inspect_slice_args.pattern.to_string(),
        parameter_default_value(slice_preview, htmlcut_core::CliParameterId::Pattern)
            .expect("inspect slice pattern default"),
    );
    assert_eq!(
        inspect_slice_args.selection.r#match.to_string(),
        slice_preview
            .default_match
            .map(|value| {
                htmlcut_core::render_cli_value(htmlcut_core::CliValue::SelectionMode(value))
            })
            .expect("inspect slice default match"),
    );
    assert_eq!(
        inspect_slice_args.whitespace.to_string(),
        parameter_default_value(slice_preview, htmlcut_core::CliParameterId::Whitespace)
            .expect("inspect slice whitespace default"),
    );
    assert_eq!(
        inspect_slice_args.output.output.to_string(),
        slice_preview
            .default_output
            .map(|value| {
                htmlcut_core::render_cli_value(htmlcut_core::CliValue::OutputMode(value))
            })
            .expect("inspect slice default output"),
    );
    assert_eq!(
        inspect_slice_args.output.preview_chars,
        DEFAULT_PREVIEW_CHARS
    );

    let catalog_args = match Cli::try_parse_from(["htmlcut", "catalog"]) {
        Ok(Cli {
            command: Commands::Catalog(args),
            ..
        }) => args,
        other => panic!("unexpected catalog parse result {other:?}"),
    };
    assert_eq!(catalog_args.output.to_string(), "text");

    let schema_args = match Cli::try_parse_from(["htmlcut", "schema"]) {
        Ok(Cli {
            command: Commands::Schema(args),
            ..
        }) => args,
        other => panic!("unexpected schema parse result {other:?}"),
    };
    assert_eq!(schema_args.output.to_string(), "text");
}

#[test]
fn contract_lint_root_help_inventory_matches_clap_subcommands() {
    let command = Cli::command();
    let root_help = crate::help::root_long_about();
    let subcommands = command
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
        root_help.contains(&format!(
            "HTMLCut has {} operator-facing entry points:",
            subcommands.len()
        )),
        "root help drifted command count: {root_help}"
    );

    for (name, about) in subcommands {
        assert!(
            root_help.contains(&name),
            "root help drifted subcommand name {name}: {root_help}"
        );
        assert!(
            root_help.contains(&about),
            "root help drifted subcommand summary {about:?}: {root_help}"
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
        htmlcut_core::cli_aux_command_descriptor(htmlcut_core::CliAuxCommandId::Catalog).about
    );

    let schema = command
        .get_subcommands()
        .find(|subcommand| subcommand.get_name() == "schema")
        .expect("schema command");
    assert_eq!(
        schema.get_about().expect("schema about").to_string(),
        htmlcut_core::cli_aux_command_descriptor(htmlcut_core::CliAuxCommandId::Schema).about
    );

    let inspect = command
        .get_subcommands()
        .find(|subcommand| subcommand.get_name() == "inspect")
        .expect("inspect command");
    assert_eq!(
        inspect.get_about().expect("inspect about").to_string(),
        htmlcut_core::cli_aux_command_descriptor(htmlcut_core::CliAuxCommandId::Inspect).about
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
    let section = htmlcut_core::CliHelpSection {
        title: "Steps".to_owned(),
        style: htmlcut_core::CliHelpSectionStyle::Numbered,
        lines: vec!["first".to_owned(), "second".to_owned()],
    };
    assert_eq!(
        crate::help::render_help_section_for_tests(&section),
        "Steps:\n1. first\n2. second"
    );

    let mut contract =
        htmlcut_core::cli_operation_contract(htmlcut_core::OperationId::SelectExtract)
            .expect("select extract contract")
            .clone();
    contract.default_output_overrides = vec![htmlcut_core::CliConditionalDefault {
        value: htmlcut_core::CliValue::OutputMode(htmlcut_core::CliOutputMode::Json),
        when: htmlcut_core::CliCondition {
            parameter: htmlcut_core::CliParameterId::Value,
            values: vec![
                htmlcut_core::CliValue::ValueType(htmlcut_core::ValueType::Structured),
                htmlcut_core::CliValue::ValueType(htmlcut_core::ValueType::OuterHtml),
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
        &htmlcut_core::OperationCliContract {
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

fn command_for_path(command_path: &[&str]) -> clap::Command {
    let mut command = Cli::command();
    for segment in command_path {
        let next = {
            command
                .get_subcommands()
                .find(|subcommand| subcommand.get_name() == *segment)
                .unwrap_or_else(|| panic!("missing command path segment {segment}"))
                .clone()
        };
        command = next;
    }
    command
}

fn assert_surface_identifiers_registered(
    label: &str,
    text: &str,
    known_schemas: &std::collections::BTreeSet<String>,
    known_operations: &std::collections::BTreeSet<String>,
) {
    let schema_pattern = Regex::new(r"\bhtmlcut\.[a-z_]+\b").expect("schema regex");
    let operation_pattern =
        Regex::new(r"\b(?:document|source|select|slice)\.(?:parse|inspect|preview|extract)\b")
            .expect("operation regex");

    for schema_name in schema_pattern
        .find_iter(text)
        .map(|capture| capture.as_str())
    {
        assert!(
            known_schemas.contains(schema_name),
            "{label} referenced unknown schema name {schema_name}: {text}"
        );
    }

    for operation_id in operation_pattern
        .find_iter(text)
        .map(|capture| capture.as_str())
    {
        assert!(
            known_operations.contains(operation_id),
            "{label} referenced unknown operation ID {operation_id}: {text}"
        );
    }
}
