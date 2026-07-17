use super::*;

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
