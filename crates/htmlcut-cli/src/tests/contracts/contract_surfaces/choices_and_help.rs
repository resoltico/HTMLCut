use super::*;

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
