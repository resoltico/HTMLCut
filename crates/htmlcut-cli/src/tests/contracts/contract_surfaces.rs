use super::*;

#[test]
fn contract_lint_clap_choice_parsers_match_core_contract_domains() {
    let select_extract = htmlcut_core::cli_contract::cli_operation_contract(
        htmlcut_core::OperationId::SelectExtract,
    )
    .expect("select extract contract");
    let select_preview = htmlcut_core::cli_contract::cli_operation_contract(
        htmlcut_core::OperationId::SelectPreview,
    )
    .expect("select preview contract");

    assert_eq!(
        parser_value_names(crate::args::cli_choice_parser::<CliMatchMode>()),
        select_extract
            .selection_modes
            .iter()
            .copied()
            .map(|mode| {
                htmlcut_core::cli_contract::render_cli_value(
                    htmlcut_core::cli_contract::CliValue::SelectionMode(mode),
                )
            })
            .collect::<Vec<_>>()
    );
    assert_eq!(
        parser_value_names(crate::args::cli_choice_parser::<CliValueMode>()),
        select_extract
            .value_modes
            .iter()
            .copied()
            .map(|value| htmlcut_core::cli_contract::render_cli_value(
                htmlcut_core::cli_contract::CliValue::ValueType(value)
            ))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        parser_value_names(crate::args::cli_choice_parser::<CliOutputMode>()),
        select_extract
            .output_modes
            .iter()
            .copied()
            .map(|mode| htmlcut_core::cli_contract::render_cli_value(
                htmlcut_core::cli_contract::CliValue::OutputMode(mode)
            ))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        parser_value_names(crate::args::cli_choice_parser::<CliCatalogOutputMode>()),
        select_preview
            .output_modes
            .iter()
            .copied()
            .map(|mode| htmlcut_core::cli_contract::render_cli_value(
                htmlcut_core::cli_contract::CliValue::OutputMode(mode)
            ))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        parser_value_names(crate::args::cli_choice_parser::<CliSchemaOutputMode>()),
        vec!["text".to_owned(), "json".to_owned()]
    );
    assert_eq!(
        parser_value_names(crate::args::cli_choice_parser::<CliInspectOutputMode>()),
        vec!["text".to_owned(), "json".to_owned()]
    );
    assert_eq!(
        parser_value_names(crate::args::cli_choice_parser::<CliWhitespaceMode>()),
        vec![
            htmlcut_core::cli_contract::render_cli_value(
                htmlcut_core::cli_contract::CliValue::WhitespaceMode(WhitespaceMode::Preserve,)
            ),
            htmlcut_core::cli_contract::render_cli_value(
                htmlcut_core::cli_contract::CliValue::WhitespaceMode(WhitespaceMode::Normalize,)
            ),
        ]
    );
    assert_eq!(
        parser_value_names(crate::args::cli_choice_parser::<CliPatternMode>()),
        vec![
            htmlcut_core::cli_contract::render_cli_value(
                htmlcut_core::cli_contract::CliValue::PatternMode(PatternMode::Literal,)
            ),
            htmlcut_core::cli_contract::render_cli_value(
                htmlcut_core::cli_contract::CliValue::PatternMode(PatternMode::Regex,)
            ),
        ]
    );
    assert_eq!(
        parser_value_names(crate::args::cli_choice_parser::<CliFetchPreflightMode>()),
        vec![
            htmlcut_core::cli_contract::render_cli_value(
                htmlcut_core::cli_contract::CliValue::FetchPreflightMode(
                    FetchPreflightMode::HeadFirst,
                )
            ),
            htmlcut_core::cli_contract::render_cli_value(
                htmlcut_core::cli_contract::CliValue::FetchPreflightMode(
                    FetchPreflightMode::GetOnly,
                )
            ),
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
        htmlcut_core::cli_contract::cli_operation_catalog()
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
                let contract =
                    htmlcut_core::cli_contract::find_cli_operation_by_command_path(&command_path)
                        .expect("registered operation example");
                assert_eq!(
                    command_name_from_raw_args(&tokens),
                    contract.report_command(),
                    "report command drift for {example}"
                );

                if let Some(value) = option_value(&tokens, "--match") {
                    assert!(
                        parameter_allowed_values(
                            contract,
                            htmlcut_core::cli_contract::CliParameterId::Match
                        )
                        .contains(&value.to_owned()),
                        "unsupported --match {value} in {example}"
                    );
                }
                if let Some(value) = option_value(&tokens, "--value") {
                    assert!(
                        parameter_allowed_values(
                            contract,
                            htmlcut_core::cli_contract::CliParameterId::Value
                        )
                        .contains(&value.to_owned()),
                        "unsupported --value {value} in {example}"
                    );
                }
                if let Some(value) = option_value(&tokens, "--output") {
                    assert!(
                        parameter_allowed_values(
                            contract,
                            htmlcut_core::cli_contract::CliParameterId::Output
                        )
                        .contains(&value.to_owned()),
                        "unsupported --output {value} in {example}"
                    );
                }
                if let Some(value) = option_value(&tokens, "--pattern") {
                    assert!(
                        parameter_allowed_values(
                            contract,
                            htmlcut_core::cli_contract::CliParameterId::Pattern
                        )
                        .contains(&value.to_owned()),
                        "unsupported --pattern {value} in {example}"
                    );
                }
                if let Some(value) = option_value(&tokens, "--fetch-preflight") {
                    assert!(
                        parameter_allowed_values(
                            contract,
                            htmlcut_core::cli_contract::CliParameterId::FetchPreflight,
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

    let source_inspect = htmlcut_core::cli_contract::cli_operation_contract(
        htmlcut_core::OperationId::SourceInspect,
    )
    .expect("source inspect contract");
    let select_extract = htmlcut_core::cli_contract::cli_operation_contract(
        htmlcut_core::OperationId::SelectExtract,
    )
    .expect("select extract contract");
    let slice_extract =
        htmlcut_core::cli_contract::cli_operation_contract(htmlcut_core::OperationId::SliceExtract)
            .expect("slice extract contract");
    let select_preview = htmlcut_core::cli_contract::cli_operation_contract(
        htmlcut_core::OperationId::SelectPreview,
    )
    .expect("select preview contract");
    let slice_preview =
        htmlcut_core::cli_contract::cli_operation_contract(htmlcut_core::OperationId::SlicePreview)
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
        parameter_default_value(
            select_extract,
            htmlcut_core::cli_contract::CliParameterId::MaxBytes
        )
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
            htmlcut_core::cli_contract::CliParameterId::FetchTimeoutMs
        )
        .expect("select fetch-timeout default"),
    );
    assert_eq!(
        select_args.source.fetch_preflight.to_string(),
        parameter_default_value(
            select_extract,
            htmlcut_core::cli_contract::CliParameterId::FetchPreflight
        )
        .expect("select fetch-preflight default"),
    );
    assert_eq!(
        select_args.selection.r#match.to_string(),
        select_extract
            .default_match
            .map(|value| {
                htmlcut_core::cli_contract::render_cli_value(
                    htmlcut_core::cli_contract::CliValue::SelectionMode(value),
                )
            })
            .expect("select default match"),
    );
    assert_eq!(
        select_args.output.value.to_string(),
        select_extract
            .default_value
            .map(|value| htmlcut_core::cli_contract::render_cli_value(
                htmlcut_core::cli_contract::CliValue::ValueType(value)
            ))
            .expect("select default value"),
    );
    assert_eq!(
        select_args.output.whitespace.to_string(),
        parameter_default_value(
            select_extract,
            htmlcut_core::cli_contract::CliParameterId::Whitespace
        )
        .expect("select whitespace default"),
    );
    assert_eq!(select_args.output.preview_chars, DEFAULT_PREVIEW_CHARS);
    assert_eq!(
        select_args.output.preview_chars.to_string(),
        parameter_default_value(
            select_extract,
            htmlcut_core::cli_contract::CliParameterId::PreviewChars
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
        parameter_default_value(
            slice_extract,
            htmlcut_core::cli_contract::CliParameterId::Pattern
        )
        .expect("slice pattern default"),
    );
    assert_eq!(
        slice_args.selection.r#match.to_string(),
        slice_extract
            .default_match
            .map(|value| {
                htmlcut_core::cli_contract::render_cli_value(
                    htmlcut_core::cli_contract::CliValue::SelectionMode(value),
                )
            })
            .expect("slice default match"),
    );
    assert_eq!(
        slice_args.output.value.to_string(),
        slice_extract
            .default_value
            .map(|value| htmlcut_core::cli_contract::render_cli_value(
                htmlcut_core::cli_contract::CliValue::ValueType(value)
            ))
            .expect("slice default value"),
    );
    assert_eq!(
        slice_args.output.whitespace.to_string(),
        parameter_default_value(
            slice_extract,
            htmlcut_core::cli_contract::CliParameterId::Whitespace
        )
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
        parameter_default_value(
            source_inspect,
            htmlcut_core::cli_contract::CliParameterId::SampleLimit
        )
        .expect("inspect source sample-limit default"),
    );
    assert_eq!(
        inspect_source_args.output.to_string(),
        source_inspect
            .default_output
            .map(|value| {
                htmlcut_core::cli_contract::render_cli_value(
                    htmlcut_core::cli_contract::CliValue::OutputMode(value),
                )
            })
            .expect("inspect source default output"),
    );
    assert_eq!(inspect_source_args.preview_chars, DEFAULT_PREVIEW_CHARS);
    assert_eq!(
        inspect_source_args.preview_chars.to_string(),
        parameter_default_value(
            source_inspect,
            htmlcut_core::cli_contract::CliParameterId::PreviewChars
        )
        .expect("inspect source preview-chars default"),
    );
    assert_eq!(
        inspect_source_args.source.fetch_preflight.to_string(),
        parameter_default_value(
            source_inspect,
            htmlcut_core::cli_contract::CliParameterId::FetchPreflight
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
                htmlcut_core::cli_contract::render_cli_value(
                    htmlcut_core::cli_contract::CliValue::SelectionMode(value),
                )
            })
            .expect("inspect select default match"),
    );
    assert_eq!(
        inspect_select_args.whitespace.to_string(),
        parameter_default_value(
            select_preview,
            htmlcut_core::cli_contract::CliParameterId::Whitespace
        )
        .expect("inspect select whitespace default"),
    );
    assert_eq!(
        inspect_select_args.output.output.to_string(),
        select_preview
            .default_output
            .map(|value| {
                htmlcut_core::cli_contract::render_cli_value(
                    htmlcut_core::cli_contract::CliValue::OutputMode(value),
                )
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
        parameter_default_value(
            slice_preview,
            htmlcut_core::cli_contract::CliParameterId::Pattern
        )
        .expect("inspect slice pattern default"),
    );
    assert_eq!(
        inspect_slice_args.selection.r#match.to_string(),
        slice_preview
            .default_match
            .map(|value| {
                htmlcut_core::cli_contract::render_cli_value(
                    htmlcut_core::cli_contract::CliValue::SelectionMode(value),
                )
            })
            .expect("inspect slice default match"),
    );
    assert_eq!(
        inspect_slice_args.whitespace.to_string(),
        parameter_default_value(
            slice_preview,
            htmlcut_core::cli_contract::CliParameterId::Whitespace
        )
        .expect("inspect slice whitespace default"),
    );
    assert_eq!(
        inspect_slice_args.output.output.to_string(),
        slice_preview
            .default_output
            .map(|value| {
                htmlcut_core::cli_contract::render_cli_value(
                    htmlcut_core::cli_contract::CliValue::OutputMode(value),
                )
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
