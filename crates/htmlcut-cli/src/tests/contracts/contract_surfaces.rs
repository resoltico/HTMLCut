use super::*;
use std::panic::catch_unwind;

#[test]
fn contract_lint_clap_choice_parsers_match_core_contract_domains() {
    let select_extract =
        crate::contract::cli_operation_contract(htmlcut_core::OperationId::SelectExtract)
            .expect("select extract contract");
    let slice_extract =
        crate::contract::cli_operation_contract(htmlcut_core::OperationId::SliceExtract)
            .expect("slice extract contract");
    let select_preview =
        crate::contract::cli_operation_contract(htmlcut_core::OperationId::SelectPreview)
            .expect("select preview contract");

    assert_eq!(
        parser_value_names(crate::args::cli_choice_parser::<CliMatchMode>()),
        select_extract
            .selection_modes
            .iter()
            .copied()
            .map(|mode| {
                crate::contract::render_cli_value(crate::contract::CliValue::SelectionMode(mode))
            })
            .collect::<Vec<_>>()
    );
    assert_eq!(
        parser_value_names(crate::args::cli_choice_parser::<CliValueMode>()),
        select_extract
            .value_modes
            .iter()
            .copied()
            .map(
                |value| crate::contract::render_cli_value(crate::contract::CliValue::ValueType(
                    value
                ))
            )
            .collect::<Vec<_>>()
    );
    assert_eq!(
        parser_value_names(crate::args::cli_choice_parser::<CliSliceValueMode>()),
        slice_extract
            .value_modes
            .iter()
            .copied()
            .map(
                |value| crate::contract::render_cli_value(crate::contract::CliValue::ValueType(
                    value
                ))
            )
            .collect::<Vec<_>>()
    );
    assert_eq!(
        parser_value_names(crate::args::cli_choice_parser::<CliOutputMode>()),
        select_extract
            .output_modes
            .iter()
            .copied()
            .map(
                |mode| crate::contract::render_cli_value(crate::contract::CliValue::OutputMode(
                    mode
                ))
            )
            .collect::<Vec<_>>()
    );
    assert_eq!(
        parser_value_names(crate::args::cli_choice_parser::<CliCatalogOutputMode>()),
        select_preview
            .output_modes
            .iter()
            .copied()
            .map(
                |mode| crate::contract::render_cli_value(crate::contract::CliValue::OutputMode(
                    mode
                ))
            )
            .collect::<Vec<_>>()
    );
    assert_eq!(
        parser_value_names(crate::args::cli_choice_parser::<CliSchemaOutputMode>()),
        vec![
            "text".to_owned(),
            "json".to_owned(),
            "index-json".to_owned(),
        ]
    );
    assert_eq!(
        parser_value_names(crate::args::cli_choice_parser::<CliInspectOutputMode>()),
        vec!["text".to_owned(), "json".to_owned()]
    );
    assert_eq!(
        parser_value_names(crate::args::cli_choice_parser::<CliWhitespaceMode>()),
        vec![
            crate::contract::render_cli_value(crate::contract::CliValue::WhitespaceMode(
                WhitespaceMode::Rendered,
            )),
            crate::contract::render_cli_value(crate::contract::CliValue::WhitespaceMode(
                WhitespaceMode::Normalize,
            )),
        ]
    );
    assert_eq!(
        parser_value_names(crate::args::cli_choice_parser::<CliPatternMode>()),
        vec![
            crate::contract::render_cli_value(crate::contract::CliValue::PatternMode(
                PatternMode::Literal,
            )),
            crate::contract::render_cli_value(crate::contract::CliValue::PatternMode(
                PatternMode::Regex,
            )),
        ]
    );
    assert_eq!(
        parser_value_names(crate::args::cli_choice_parser::<CliFetchPreflightMode>()),
        vec![
            crate::contract::render_cli_value(crate::contract::CliValue::FetchPreflightMode(
                FetchPreflightMode::HeadFirst,
            )),
            crate::contract::render_cli_value(crate::contract::CliValue::FetchPreflightMode(
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
        crate::contract::cli_operation_catalog()
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
                let contract = crate::contract::find_cli_operation_by_command_path(&command_path)
                    .expect("registered operation example");
                assert_eq!(
                    command_name_from_raw_args(&tokens),
                    contract.report_command(),
                    "report command drift for {example}"
                );

                if let Some(value) = option_value(&tokens, "--match") {
                    assert!(
                        parameter_allowed_values(contract, crate::contract::CliParameterId::Match)
                            .contains(&value.to_owned()),
                        "unsupported --match {value} in {example}"
                    );
                }
                if let Some(value) = option_value(&tokens, "--value") {
                    assert!(
                        parameter_allowed_values(contract, crate::contract::CliParameterId::Value)
                            .contains(&value.to_owned()),
                        "unsupported --value {value} in {example}"
                    );
                }
                if let Some(value) = option_value(&tokens, "--output") {
                    assert!(
                        parameter_allowed_values(contract, crate::contract::CliParameterId::Output)
                            .contains(&value.to_owned()),
                        "unsupported --output {value} in {example}"
                    );
                }
                if let Some(value) = option_value(&tokens, "--pattern") {
                    assert!(
                        parameter_allowed_values(
                            contract,
                            crate::contract::CliParameterId::Pattern
                        )
                        .contains(&value.to_owned()),
                        "unsupported --pattern {value} in {example}"
                    );
                }
                if let Some(value) = option_value(&tokens, "--fetch-preflight") {
                    assert!(
                        parameter_allowed_values(
                            contract,
                            crate::contract::CliParameterId::FetchPreflight,
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
    let command = crate::command();
    assert_command_path_registered(&command, &["catalog"]);
    assert_command_path_registered(&command, &["schema"]);

    let source_inspect =
        crate::contract::cli_operation_contract(htmlcut_core::OperationId::SourceInspect)
            .expect("source inspect contract");
    let select_extract =
        crate::contract::cli_operation_contract(htmlcut_core::OperationId::SelectExtract)
            .expect("select extract contract");
    let slice_extract =
        crate::contract::cli_operation_contract(htmlcut_core::OperationId::SliceExtract)
            .expect("slice extract contract");
    let select_preview =
        crate::contract::cli_operation_contract(htmlcut_core::OperationId::SelectPreview)
            .expect("select preview contract");
    let slice_preview =
        crate::contract::cli_operation_contract(htmlcut_core::OperationId::SlicePreview)
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
        parameter_default_value(select_extract, crate::contract::CliParameterId::MaxBytes)
            .expect("select max-bytes default"),
    );
    assert_eq!(
        select_args.source.fetch_timeout_ms,
        DEFAULT_FETCH_TIMEOUT_MS
    );
    assert_eq!(
        select_args.source.fetch_timeout_ms.to_string(),
        parameter_default_value(
            select_extract,
            crate::contract::CliParameterId::FetchTimeoutMs
        )
        .expect("select fetch-timeout default"),
    );
    assert_eq!(
        select_args.source.fetch_preflight.to_string(),
        parameter_default_value(
            select_extract,
            crate::contract::CliParameterId::FetchPreflight
        )
        .expect("select fetch-preflight default"),
    );
    assert_eq!(
        select_args.selection.r#match.to_string(),
        select_extract
            .default_match
            .map(|value| {
                crate::contract::render_cli_value(crate::contract::CliValue::SelectionMode(value))
            })
            .expect("select default match"),
    );
    assert_eq!(
        select_args.output.value.to_string(),
        select_extract
            .default_value
            .map(
                |value| crate::contract::render_cli_value(crate::contract::CliValue::ValueType(
                    value
                ))
            )
            .expect("select default value"),
    );
    assert_eq!(
        select_args.output.whitespace.to_string(),
        parameter_default_value(select_extract, crate::contract::CliParameterId::Whitespace)
            .expect("select whitespace default"),
    );
    assert_eq!(select_args.output.preview_chars, DEFAULT_PREVIEW_CHARS);
    assert_eq!(
        select_args.output.preview_chars.to_string(),
        parameter_default_value(
            select_extract,
            crate::contract::CliParameterId::PreviewChars
        )
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
        parameter_default_value(slice_extract, crate::contract::CliParameterId::Pattern)
            .expect("slice pattern default"),
    );
    assert_eq!(
        slice_args.selection.r#match.to_string(),
        slice_extract
            .default_match
            .map(|value| {
                crate::contract::render_cli_value(crate::contract::CliValue::SelectionMode(value))
            })
            .expect("slice default match"),
    );
    assert_eq!(
        slice_args.output.value.to_string(),
        slice_extract
            .default_value
            .map(
                |value| crate::contract::render_cli_value(crate::contract::CliValue::ValueType(
                    value
                ))
            )
            .expect("slice default value"),
    );
    assert_eq!(
        slice_args.output.whitespace.to_string(),
        parameter_default_value(slice_extract, crate::contract::CliParameterId::Whitespace)
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
        parameter_default_value(source_inspect, crate::contract::CliParameterId::SampleLimit)
            .expect("inspect source sample-limit default"),
    );
    assert_eq!(
        inspect_source_args.output.to_string(),
        source_inspect
            .default_output
            .map(|value| {
                crate::contract::render_cli_value(crate::contract::CliValue::OutputMode(value))
            })
            .expect("inspect source default output"),
    );
    assert_eq!(inspect_source_args.preview_chars, DEFAULT_PREVIEW_CHARS);
    assert_eq!(
        inspect_source_args.preview_chars.to_string(),
        parameter_default_value(
            source_inspect,
            crate::contract::CliParameterId::PreviewChars
        )
        .expect("inspect source preview-chars default"),
    );
    assert_eq!(
        inspect_source_args.source.fetch_preflight.to_string(),
        parameter_default_value(
            source_inspect,
            crate::contract::CliParameterId::FetchPreflight
        )
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
                crate::contract::render_cli_value(crate::contract::CliValue::SelectionMode(value))
            })
            .expect("inspect select default match"),
    );
    assert_eq!(
        inspect_select_args.whitespace.to_string(),
        parameter_default_value(select_preview, crate::contract::CliParameterId::Whitespace)
            .expect("inspect select whitespace default"),
    );
    assert_eq!(
        inspect_select_args.output.output.to_string(),
        select_preview
            .default_output
            .map(|value| {
                crate::contract::render_cli_value(crate::contract::CliValue::OutputMode(value))
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
        parameter_default_value(slice_preview, crate::contract::CliParameterId::Pattern)
            .expect("inspect slice pattern default"),
    );
    assert_eq!(
        inspect_slice_args.selection.r#match.to_string(),
        slice_preview
            .default_match
            .map(|value| {
                crate::contract::render_cli_value(crate::contract::CliValue::SelectionMode(value))
            })
            .expect("inspect slice default match"),
    );
    assert_eq!(
        inspect_slice_args.whitespace.to_string(),
        parameter_default_value(slice_preview, crate::contract::CliParameterId::Whitespace)
            .expect("inspect slice whitespace default"),
    );
    assert_eq!(
        inspect_slice_args.output.output.to_string(),
        slice_preview
            .default_output
            .map(|value| {
                crate::contract::render_cli_value(crate::contract::CliValue::OutputMode(value))
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
fn contract_validation_helpers_accept_the_live_cli_catalogs() {
    assert!(
        crate::contract::cli_operation_catalog_validation_errors().is_empty(),
        "live CLI operation catalog should validate against OPERATION_CATALOG"
    );
    crate::contract::assert_cli_operation_catalog_consistency_for_tests(
        crate::contract::cli_operation_catalog(),
    );

    let select_extract =
        crate::contract::cli_operation_contract(htmlcut_core::OperationId::SelectExtract)
            .expect("select extract contract");
    assert!(
        crate::contract::validate_command_contract_for_tests(select_extract).is_empty(),
        "live select.extract contract should validate"
    );

    let valid_condition = crate::contract::CliCondition {
        parameter: crate::contract::CliParameterId::Value,
        values: vec![crate::contract::CliValue::ValueType(
            htmlcut_core::ValueType::Text,
        )],
    };
    assert!(
        crate::contract::validate_condition_for_tests(
            htmlcut_core::OperationId::SelectExtract,
            "test",
            &valid_condition,
            &select_extract.parameters,
        )
        .is_empty(),
        "live parameter metadata should accept valid conditions"
    );

    assert!(
        crate::contract::cli_help_catalog_validation_errors().is_empty(),
        "live CLI help catalog should validate"
    );
    crate::contract::assert_cli_help_catalog_errors_for_tests(Vec::new());

    assert!(
        crate::contract::cli_aux_command_catalog_validation_errors_for_tests(
            crate::contract::cli_aux_command_catalog(),
        )
        .is_empty(),
        "live auxiliary command catalog should validate"
    );
    crate::contract::assert_cli_aux_command_catalog_for_tests(
        crate::contract::cli_aux_command_catalog(),
    );
}

#[test]
fn contract_validation_helpers_report_catalog_membership_drift_and_assert_failures() {
    let select_extract =
        crate::contract::cli_operation_contract(htmlcut_core::OperationId::SelectExtract)
            .expect("select extract contract")
            .clone();
    let mut duplicate = select_extract.clone();
    duplicate.command_path = &["broken"];

    let mut core_only = select_extract.clone();
    core_only.operation_id = htmlcut_core::OperationId::DocumentParse;

    let document_parse =
        htmlcut_core::operation_descriptor(htmlcut_core::OperationId::DocumentParse)
            .copied()
            .expect("document.parse descriptor");

    let errors = crate::contract::cli_operation_catalog_validation_errors_for(
        &[document_parse],
        &[select_extract, duplicate.clone(), core_only],
    );

    for expected in [
        "select.extract is missing from OPERATION_CATALOG",
        "select.extract appears more than once in cli_operation_catalog()",
        "document.parse appears in cli_operation_catalog() but is marked core-only in OPERATION_CATALOG",
        "document.parse display command drifted",
    ] {
        assert!(
            errors.iter().any(|error| error.contains(expected)),
            "missing operation catalog error containing {expected:?}: {errors:#?}"
        );
    }

    assert!(
        catch_unwind(|| {
            crate::contract::assert_cli_operation_catalog_consistency_for_tests(&[duplicate])
        })
        .is_err(),
        "operation catalog assertion should panic on drift"
    );
}

#[test]
fn contract_validation_helpers_report_parameter_default_and_constraint_drift() {
    let mut contract =
        crate::contract::cli_operation_contract(htmlcut_core::OperationId::SelectExtract)
            .expect("select extract contract")
            .clone();
    contract.command_path = &[];
    contract.invocation = "htmlcut wrong";
    contract.examples = vec!["htmlcut bad"];
    contract.selection_modes = vec![crate::contract::CliSelectionMode::Single];
    contract.default_match = Some(crate::contract::CliSelectionMode::All);
    contract.value_modes = vec![htmlcut_core::ValueType::Text];
    contract.default_value = Some(htmlcut_core::ValueType::Attribute);
    contract.output_modes = vec![crate::contract::CliOutputMode::Text];
    contract.default_output = Some(crate::contract::CliOutputMode::Html);
    contract.default_output_overrides = vec![
        crate::contract::CliConditionalDefault {
            value: crate::contract::CliValue::Boolean(true),
            when: crate::contract::CliCondition {
                parameter: crate::contract::CliParameterId::Match,
                values: vec![crate::contract::CliValue::SelectionMode(
                    crate::contract::CliSelectionMode::Single,
                )],
            },
        },
        crate::contract::CliConditionalDefault {
            value: crate::contract::CliValue::OutputMode(crate::contract::CliOutputMode::Json),
            when: crate::contract::CliCondition {
                parameter: crate::contract::CliParameterId::Match,
                values: vec![crate::contract::CliValue::SelectionMode(
                    crate::contract::CliSelectionMode::Single,
                )],
            },
        },
    ];

    let match_index = contract
        .parameters
        .iter()
        .position(|parameter| parameter.id == crate::contract::CliParameterId::Match)
        .expect("match parameter");
    contract.parameters[match_index].allowed_values =
        vec![crate::contract::CliValue::SelectionMode(
            crate::contract::CliSelectionMode::First,
        )];

    let value_index = contract
        .parameters
        .iter()
        .position(|parameter| parameter.id == crate::contract::CliParameterId::Value)
        .expect("value parameter");
    contract.parameters[value_index].allowed_values = vec![crate::contract::CliValue::ValueType(
        htmlcut_core::ValueType::Attribute,
    )];
    contract.parameters[value_index].default = Some(crate::contract::CliValue::ValueType(
        htmlcut_core::ValueType::Structured,
    ));

    let output_index = contract
        .parameters
        .iter()
        .position(|parameter| parameter.id == crate::contract::CliParameterId::Output)
        .expect("output parameter");
    contract.parameters[output_index].allowed_values = vec![crate::contract::CliValue::OutputMode(
        crate::contract::CliOutputMode::Json,
    )];
    contract.parameters[output_index].default = Some(crate::contract::CliValue::OutputMode(
        crate::contract::CliOutputMode::None,
    ));

    let flag_index = contract
        .parameters
        .iter()
        .position(|parameter| matches!(parameter.kind, crate::contract::CliParameterKind::Flag))
        .expect("flag parameter");
    contract.parameters[flag_index].value_hint = Some("VALUE");
    contract.parameters[flag_index].default = Some(crate::contract::CliValue::Boolean(true));

    let requirement_index = contract
        .parameters
        .iter()
        .position(|parameter| parameter.id == crate::contract::CliParameterId::Attribute)
        .expect("attribute parameter");
    contract.parameters[requirement_index].requirement =
        crate::contract::CliParameterRequirement::RequiredUnless(
            crate::contract::CliParameterId::From,
        );

    contract
        .parameters
        .push(contract.parameters[match_index].clone());

    contract.constraints = vec![
        crate::contract::CliConstraint::RequiresParameter {
            parameter: crate::contract::CliParameterId::From,
            when: crate::contract::CliCondition {
                parameter: crate::contract::CliParameterId::Match,
                values: vec![crate::contract::CliValue::SelectionMode(
                    crate::contract::CliSelectionMode::Single,
                )],
            },
        },
        crate::contract::CliConstraint::RestrictsParameterValues {
            parameter: crate::contract::CliParameterId::Value,
            allowed_values: vec![crate::contract::CliValue::ValueType(
                htmlcut_core::ValueType::Structured,
            )],
            when: crate::contract::CliCondition {
                parameter: crate::contract::CliParameterId::Match,
                values: vec![crate::contract::CliValue::SelectionMode(
                    crate::contract::CliSelectionMode::Single,
                )],
            },
        },
    ];

    let errors = crate::contract::validate_command_contract_for_tests(&contract);

    for expected in [
        "select.extract has an empty command path",
        "select.extract default_match",
        "select.extract default_value",
        "select.extract default_output",
        "select.extract default_output_override \"true\" is not an output mode",
        "select.extract default_output_override \"json\" is not present in output_modes",
        "select.extract lists --match more than once",
        "select.extract flag",
        "select.extract parameter --value default \"structured\" is not present in allowed_values",
        "select.extract parameter --attribute depends on missing parameter --from",
        "select.extract parameter --match drifted from selection_modes",
        "select.extract parameter --value drifted from value_modes",
        "select.extract parameter --output drifted from output_modes",
        "select.extract constraint references missing parameter --from",
        "select.extract value restriction on --value references values outside its allowed_values",
    ] {
        assert!(
            errors.iter().any(|error| error.contains(expected)),
            "missing command contract error containing {expected:?}: {errors:#?}"
        );
    }

    let mut misrouted = contract.clone();
    misrouted.command_path = &["slice"];
    let route_errors = crate::contract::validate_command_contract_for_tests(&misrouted);
    assert!(
        route_errors
            .iter()
            .any(|error| error.contains("select.extract invocation")),
        "missing invocation drift error: {route_errors:#?}"
    );
    assert!(
        route_errors
            .iter()
            .any(|error| error.contains("select.extract example")),
        "missing example drift error: {route_errors:#?}"
    );

    let mut missing_restriction_target = misrouted.clone();
    missing_restriction_target.constraints =
        vec![crate::contract::CliConstraint::RestrictsParameterValues {
            parameter: crate::contract::CliParameterId::From,
            allowed_values: vec![crate::contract::CliValue::PatternMode(
                htmlcut_core::PatternMode::Literal,
            )],
            when: crate::contract::CliCondition {
                parameter: crate::contract::CliParameterId::Match,
                values: vec![crate::contract::CliValue::SelectionMode(
                    crate::contract::CliSelectionMode::Single,
                )],
            },
        }];
    let restriction_target_errors =
        crate::contract::validate_command_contract_for_tests(&missing_restriction_target);
    assert!(
        restriction_target_errors
            .iter()
            .any(|error| error.contains("value restriction references missing parameter --from")),
        "missing restriction-target error: {restriction_target_errors:#?}"
    );
}

#[test]
fn contract_validation_helpers_cover_missing_optional_output_and_empty_restriction_domains() {
    let select_extract =
        crate::contract::cli_operation_contract(htmlcut_core::OperationId::SelectExtract)
            .expect("select extract contract");

    let mut without_default_output = select_extract.clone();
    without_default_output.default_output = None;
    let missing_default_errors =
        crate::contract::validate_command_contract_for_tests(&without_default_output);
    assert!(
        !missing_default_errors
            .iter()
            .any(|error| error.contains("default_output")),
        "missing optional default_output should not be treated as drift: {missing_default_errors:#?}"
    );

    let mut without_output_parameter = select_extract.clone();
    without_output_parameter
        .parameters
        .retain(|parameter| parameter.id != crate::contract::CliParameterId::Output);
    let missing_output_parameter_errors =
        crate::contract::validate_command_contract_for_tests(&without_output_parameter);
    assert!(
        !missing_output_parameter_errors
            .iter()
            .any(|error| error.contains("parameter --output drifted")),
        "missing optional output parameter should skip output-domain comparison: {missing_output_parameter_errors:#?}"
    );

    let mut empty_restriction_domain = select_extract.clone();
    empty_restriction_domain.constraints =
        vec![crate::contract::CliConstraint::RestrictsParameterValues {
            parameter: crate::contract::CliParameterId::Value,
            allowed_values: vec![crate::contract::CliValue::ValueType(
                htmlcut_core::ValueType::Text,
            )],
            when: crate::contract::CliCondition {
                parameter: crate::contract::CliParameterId::Match,
                values: vec![crate::contract::CliValue::SelectionMode(
                    crate::contract::CliSelectionMode::Single,
                )],
            },
        }];
    if let Some(parameter) = empty_restriction_domain
        .parameters
        .iter_mut()
        .find(|parameter| parameter.id == crate::contract::CliParameterId::Value)
    {
        parameter.allowed_values.clear();
    }
    let empty_restriction_domain_errors =
        crate::contract::validate_command_contract_for_tests(&empty_restriction_domain);
    assert!(
        !empty_restriction_domain_errors.iter().any(|error| {
            error.contains(
                "value restriction on --value references values outside its allowed_values",
            )
        }),
        "empty restriction domains should skip impossible value-subset checks: {empty_restriction_domain_errors:#?}"
    );
}

#[test]
fn contract_validation_helpers_report_invalid_conditions() {
    let missing_parameter = crate::contract::validate_condition_for_tests(
        htmlcut_core::OperationId::SelectExtract,
        "scope",
        &crate::contract::CliCondition {
            parameter: crate::contract::CliParameterId::Output,
            values: vec![crate::contract::CliValue::OutputMode(
                crate::contract::CliOutputMode::Text,
            )],
        },
        &[],
    );
    assert!(
        missing_parameter
            .iter()
            .any(|error| error.contains("references missing condition parameter --output"))
    );

    let parameter_without_allowed_values = crate::contract::CliParameterDescriptor {
        section: crate::contract::CliParameterSection::Extraction,
        id: crate::contract::CliParameterId::Output,
        kind: crate::contract::CliParameterKind::Option,
        requirement: crate::contract::CliParameterRequirement::Optional,
        value_hint: Some("MODE"),
        default: None,
        allowed_values: Vec::new(),
        summary: "Output mode",
    };
    let no_allowed_values = crate::contract::validate_condition_for_tests(
        htmlcut_core::OperationId::SelectExtract,
        "scope",
        &crate::contract::CliCondition {
            parameter: crate::contract::CliParameterId::Output,
            values: vec![crate::contract::CliValue::OutputMode(
                crate::contract::CliOutputMode::Text,
            )],
        },
        std::slice::from_ref(&parameter_without_allowed_values),
    );
    assert!(no_allowed_values.iter().any(|error| {
        error.contains("references condition parameter --output without allowed_values")
    }));

    let unsupported_values = crate::contract::validate_condition_for_tests(
        htmlcut_core::OperationId::SelectExtract,
        "scope",
        &crate::contract::CliCondition {
            parameter: crate::contract::CliParameterId::Output,
            values: vec![crate::contract::CliValue::OutputMode(
                crate::contract::CliOutputMode::Json,
            )],
        },
        &[crate::contract::CliParameterDescriptor {
            allowed_values: vec![crate::contract::CliValue::OutputMode(
                crate::contract::CliOutputMode::Text,
            )],
            ..parameter_without_allowed_values
        }],
    );
    assert!(
        unsupported_values
            .iter()
            .any(|error| error.contains("references unsupported values for --output"))
    );
}
