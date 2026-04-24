use super::*;

#[test]
fn contract_lint_cli_operation_catalog_stays_consistent() {
    assert_eq!(
        crate::cli_contract::cli_operation_catalog_validation_errors(),
        Vec::<String>::new()
    );
}
#[test]
fn contract_lint_cli_help_catalog_stays_consistent() {
    assert_eq!(
        crate::cli_contract::cli_help_catalog_validation_errors(),
        Vec::<String>::new()
    );

    let root_help = crate::cli_contract::cli_root_help_document();
    assert!(
        root_help
            .sections
            .iter()
            .any(|section| section.title == "Start here"),
        "root help should start with a workflow guide"
    );
    assert!(
        root_help
            .sections
            .iter()
            .any(|section| section.title == "Reusable requests"),
        "root help should surface reusable request workflows"
    );
    assert!(
        root_help
            .examples
            .iter()
            .all(|example| example.starts_with("htmlcut ")),
        "root help examples should stay executable htmlcut commands"
    );
    assert!(
        root_help
            .examples
            .iter()
            .any(|example| example.contains("--emit-request-file")),
        "root help examples should show how to save a reusable request"
    );
    assert!(
        root_help
            .examples
            .iter()
            .any(|example| example.contains("--request-file")),
        "root help examples should show how to rerun a saved request"
    );

    let inspect_help = crate::cli_contract::cli_aux_command_help_document(
        crate::cli_contract::CliAuxCommandId::Inspect,
    );
    let inspect_lines = inspect_help
        .sections
        .iter()
        .flat_map(|section| section.lines.iter())
        .collect::<Vec<_>>();
    assert!(
        inspect_lines
            .iter()
            .any(|line| line.starts_with("inspect source"))
    );
    assert!(
        inspect_lines
            .iter()
            .any(|line| line.starts_with("inspect select"))
    );
    assert!(
        inspect_lines
            .iter()
            .any(|line| line.starts_with("inspect slice"))
    );

    let select_help = crate::cli_contract::cli_operation_help_document(OperationId::SelectExtract)
        .expect("select extract help should exist");
    assert!(
        select_help
            .sections
            .iter()
            .flat_map(|section| section.lines.iter())
            .any(|line| line.contains("CSS selector matches")),
        "operation help should carry the canonical extraction summary"
    );
}
#[test]
fn contract_lint_rejects_malformed_command_contracts_and_conditions() {
    let mut contract = crate::cli_contract::cli_operation_contract(OperationId::SelectExtract)
        .expect("select contract")
        .clone();
    contract.command_path = &[];
    contract.invocation = "select";
    contract.examples = vec!["select ./page.html --css article"];
    contract.default_match = Some(crate::cli_contract::CliSelectionMode::Single);
    contract.selection_modes = vec![crate::cli_contract::CliSelectionMode::First];
    contract.default_value = Some(ValueType::Structured);
    contract.value_modes = vec![ValueType::Text];
    contract.default_output = Some(crate::cli_contract::CliOutputMode::Json);
    contract.output_modes = vec![crate::cli_contract::CliOutputMode::Text];
    contract.default_output_overrides = vec![
        crate::cli_contract::CliConditionalDefault {
            value: crate::cli_contract::CliValue::ValueType(ValueType::Text),
            when: crate::cli_contract::CliCondition {
                parameter: crate::cli_contract::CliParameterId::Pattern,
                values: vec![crate::cli_contract::CliValue::PatternMode(
                    PatternMode::Regex,
                )],
            },
        },
        crate::cli_contract::CliConditionalDefault {
            value: crate::cli_contract::CliValue::OutputMode(
                crate::cli_contract::CliOutputMode::Json,
            ),
            when: crate::cli_contract::CliCondition {
                parameter: crate::cli_contract::CliParameterId::Output,
                values: vec![crate::cli_contract::CliValue::OutputMode(
                    crate::cli_contract::CliOutputMode::Html,
                )],
            },
        },
    ];
    contract.parameters = vec![
        crate::cli_contract::CliParameterDescriptor {
            section: crate::cli_contract::CliParameterSection::Extraction,
            id: crate::cli_contract::CliParameterId::Output,
            kind: crate::cli_contract::CliParameterKind::Option,
            requirement: crate::cli_contract::CliParameterRequirement::Optional,
            value_hint: Some("<MODE>"),
            default: Some(crate::cli_contract::CliValue::OutputMode(
                crate::cli_contract::CliOutputMode::Json,
            )),
            allowed_values: vec![crate::cli_contract::CliValue::OutputMode(
                crate::cli_contract::CliOutputMode::Text,
            )],
            summary: "output",
        },
        crate::cli_contract::CliParameterDescriptor {
            section: crate::cli_contract::CliParameterSection::Extraction,
            id: crate::cli_contract::CliParameterId::Output,
            kind: crate::cli_contract::CliParameterKind::Option,
            requirement: crate::cli_contract::CliParameterRequirement::Optional,
            value_hint: Some("<MODE>"),
            default: None,
            allowed_values: vec![crate::cli_contract::CliValue::OutputMode(
                crate::cli_contract::CliOutputMode::Text,
            )],
            summary: "duplicate output",
        },
        crate::cli_contract::CliParameterDescriptor {
            section: crate::cli_contract::CliParameterSection::Selection,
            id: crate::cli_contract::CliParameterId::Match,
            kind: crate::cli_contract::CliParameterKind::Option,
            requirement: crate::cli_contract::CliParameterRequirement::Optional,
            value_hint: Some("<MATCH>"),
            default: Some(crate::cli_contract::CliValue::SelectionMode(
                crate::cli_contract::CliSelectionMode::Single,
            )),
            allowed_values: vec![crate::cli_contract::CliValue::SelectionMode(
                crate::cli_contract::CliSelectionMode::First,
            )],
            summary: "match",
        },
        crate::cli_contract::CliParameterDescriptor {
            section: crate::cli_contract::CliParameterSection::Extraction,
            id: crate::cli_contract::CliParameterId::Value,
            kind: crate::cli_contract::CliParameterKind::Option,
            requirement: crate::cli_contract::CliParameterRequirement::Optional,
            value_hint: Some("<VALUE>"),
            default: Some(crate::cli_contract::CliValue::ValueType(
                ValueType::Structured,
            )),
            allowed_values: vec![crate::cli_contract::CliValue::ValueType(ValueType::Text)],
            summary: "value",
        },
        crate::cli_contract::CliParameterDescriptor {
            section: crate::cli_contract::CliParameterSection::Extraction,
            id: crate::cli_contract::CliParameterId::RewriteUrls,
            kind: crate::cli_contract::CliParameterKind::Flag,
            requirement: crate::cli_contract::CliParameterRequirement::Optional,
            value_hint: Some("<BOOL>"),
            default: Some(crate::cli_contract::CliValue::Boolean(true)),
            allowed_values: Vec::new(),
            summary: "rewrite URLs",
        },
        crate::cli_contract::CliParameterDescriptor {
            section: crate::cli_contract::CliParameterSection::Extraction,
            id: crate::cli_contract::CliParameterId::Bundle,
            kind: crate::cli_contract::CliParameterKind::Option,
            requirement: crate::cli_contract::CliParameterRequirement::RequiredUnless(
                crate::cli_contract::CliParameterId::Css,
            ),
            value_hint: Some("<DIR>"),
            default: None,
            allowed_values: Vec::new(),
            summary: "bundle",
        },
        crate::cli_contract::CliParameterDescriptor {
            section: crate::cli_contract::CliParameterSection::Extraction,
            id: crate::cli_contract::CliParameterId::Attribute,
            kind: crate::cli_contract::CliParameterKind::Option,
            requirement: crate::cli_contract::CliParameterRequirement::AllowedOnlyWhen(
                crate::cli_contract::CliCondition {
                    parameter: crate::cli_contract::CliParameterId::Bundle,
                    values: vec![crate::cli_contract::CliValue::Boolean(true)],
                },
            ),
            value_hint: Some("<NAME>"),
            default: None,
            allowed_values: Vec::new(),
            summary: "attribute",
        },
    ];
    contract.constraints = vec![
        crate::cli_contract::CliConstraint::RequiresParameter {
            parameter: crate::cli_contract::CliParameterId::Css,
            when: crate::cli_contract::CliCondition {
                parameter: crate::cli_contract::CliParameterId::Output,
                values: vec![crate::cli_contract::CliValue::OutputMode(
                    crate::cli_contract::CliOutputMode::Text,
                )],
            },
        },
        crate::cli_contract::CliConstraint::RestrictsParameterValues {
            parameter: crate::cli_contract::CliParameterId::Css,
            allowed_values: vec![crate::cli_contract::CliValue::OutputMode(
                crate::cli_contract::CliOutputMode::None,
            )],
            when: crate::cli_contract::CliCondition {
                parameter: crate::cli_contract::CliParameterId::Output,
                values: vec![crate::cli_contract::CliValue::OutputMode(
                    crate::cli_contract::CliOutputMode::Text,
                )],
            },
        },
        crate::cli_contract::CliConstraint::RestrictsParameterValues {
            parameter: crate::cli_contract::CliParameterId::Output,
            allowed_values: vec![crate::cli_contract::CliValue::OutputMode(
                crate::cli_contract::CliOutputMode::None,
            )],
            when: crate::cli_contract::CliCondition {
                parameter: crate::cli_contract::CliParameterId::Output,
                values: vec![crate::cli_contract::CliValue::OutputMode(
                    crate::cli_contract::CliOutputMode::Html,
                )],
            },
        },
    ];

    let errors = crate::cli_contract::validate_command_contract_for_tests(&contract);
    for expected in [
        "has an empty command path",
        "invocation \"select\" does not start with",
        "example \"select ./page.html --css article\" does not start with",
        "default_match \"single\" is not present in selection_modes",
        "default_value \"structured\" is not present in value_modes",
        "default_output \"json\" is not present in output_modes",
        "default_output_override \"text\" is not an output mode",
        "default_output_override \"json\" is not present in output_modes",
        "lists --output more than once",
        "flag --rewrite-urls carries a value_hint",
        "flag --rewrite-urls should default to false",
        "parameter --output default \"json\" is not present in allowed_values",
        "parameter --bundle depends on missing parameter --css",
        "references condition parameter --bundle without allowed_values",
        "constraint references missing parameter --css",
        "value restriction references missing parameter --css",
        "references unsupported values for --output",
        "value restriction on --output references values outside its allowed_values",
    ] {
        assert!(
            errors.iter().any(|error| error.contains(expected)),
            "missing validation error containing {expected:?}: {errors:#?}"
        );
    }

    let missing_condition_errors = crate::cli_contract::validate_condition_for_tests(
        OperationId::SelectExtract,
        "default_output_override",
        &crate::cli_contract::CliCondition {
            parameter: crate::cli_contract::CliParameterId::Pattern,
            values: vec![crate::cli_contract::CliValue::PatternMode(
                PatternMode::Regex,
            )],
        },
        &contract.parameters,
    );
    assert!(
        missing_condition_errors
            .iter()
            .any(|error| error.contains("references missing condition parameter --pattern")),
        "missing explicit condition-lint error: {missing_condition_errors:#?}"
    );

    let mut match_drift = crate::cli_contract::cli_operation_contract(OperationId::SelectExtract)
        .expect("select contract")
        .clone();
    match_drift.selection_modes = vec![crate::cli_contract::CliSelectionMode::All];
    let match_drift_errors = crate::cli_contract::validate_command_contract_for_tests(&match_drift);
    assert!(
        match_drift_errors
            .iter()
            .any(|error| error.contains("parameter --match drifted from selection_modes")),
        "missing selection-mode drift error: {match_drift_errors:#?}"
    );

    let mut value_drift = crate::cli_contract::cli_operation_contract(OperationId::SelectExtract)
        .expect("select contract")
        .clone();
    value_drift.value_modes = vec![ValueType::Structured];
    let value_drift_errors = crate::cli_contract::validate_command_contract_for_tests(&value_drift);
    assert!(
        value_drift_errors
            .iter()
            .any(|error| error.contains("parameter --value drifted from value_modes")),
        "missing value-mode drift error: {value_drift_errors:#?}"
    );

    let mut output_drift = crate::cli_contract::cli_operation_contract(OperationId::SelectExtract)
        .expect("select contract")
        .clone();
    output_drift.output_modes = vec![crate::cli_contract::CliOutputMode::Json];
    let output_drift_errors =
        crate::cli_contract::validate_command_contract_for_tests(&output_drift);
    assert!(
        output_drift_errors
            .iter()
            .any(|error| error.contains("parameter --output drifted from output_modes")),
        "missing output-mode drift error: {output_drift_errors:#?}"
    );
}
#[test]
fn contract_lint_covers_optional_output_and_empty_restriction_target_branches() {
    let base_contract = crate::cli_contract::cli_operation_contract(OperationId::SelectExtract)
        .expect("select contract")
        .clone();

    let mut no_default_output = base_contract.clone();
    no_default_output.default_output = None;
    let no_default_output_errors =
        crate::cli_contract::validate_command_contract_for_tests(&no_default_output);
    assert!(
        no_default_output_errors.is_empty(),
        "unexpected errors for optional default_output: {no_default_output_errors:#?}"
    );

    let mut without_output_parameter = base_contract.clone();
    without_output_parameter.default_output = None;
    without_output_parameter.default_output_overrides.clear();
    without_output_parameter.constraints.clear();
    without_output_parameter
        .parameters
        .retain(|parameter| parameter.id != crate::cli_contract::CliParameterId::Output);
    let without_output_parameter_errors =
        crate::cli_contract::validate_command_contract_for_tests(&without_output_parameter);
    assert!(
        without_output_parameter_errors.is_empty(),
        "unexpected errors for a contract without --output: {without_output_parameter_errors:#?}"
    );

    let mut empty_target_allowed_values = base_contract;
    empty_target_allowed_values.constraints = vec![
        crate::cli_contract::CliConstraint::RestrictsParameterValues {
            parameter: crate::cli_contract::CliParameterId::Bundle,
            allowed_values: vec![],
            when: crate::cli_contract::CliCondition {
                parameter: crate::cli_contract::CliParameterId::Output,
                values: vec![crate::cli_contract::CliValue::OutputMode(
                    crate::cli_contract::CliOutputMode::Json,
                )],
            },
        },
    ];
    let empty_target_allowed_values_errors =
        crate::cli_contract::validate_command_contract_for_tests(&empty_target_allowed_values);
    assert!(
        empty_target_allowed_values_errors.is_empty(),
        "unexpected errors for empty restriction targets: {empty_target_allowed_values_errors:#?}"
    );
}
