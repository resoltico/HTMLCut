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

    let root_help = crate::cli_root_help_document();
    assert!(
        root_help
            .sections
            .iter()
            .any(|section| section.title.contains("operator-facing entry points")),
        "root help should describe the top-level inventory"
    );
    assert!(
        root_help
            .examples
            .iter()
            .all(|example| example.starts_with("htmlcut ")),
        "root help examples should stay executable htmlcut commands"
    );

    let inspect_help = crate::cli_aux_command_help_document(crate::CliAuxCommandId::Inspect);
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

    let select_help = crate::cli_operation_help_document(OperationId::SelectExtract)
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
    let mut contract = crate::cli_operation_contract(OperationId::SelectExtract)
        .expect("select contract")
        .clone();
    contract.command_path = &[];
    contract.invocation = "select";
    contract.examples = vec!["select ./page.html --css article"];
    contract.default_match = Some(crate::CliSelectionMode::Single);
    contract.selection_modes = vec![crate::CliSelectionMode::First];
    contract.default_value = Some(ValueType::Structured);
    contract.value_modes = vec![ValueType::Text];
    contract.default_output = Some(crate::CliOutputMode::Json);
    contract.output_modes = vec![crate::CliOutputMode::Text];
    contract.default_output_overrides = vec![
        crate::CliConditionalDefault {
            value: crate::CliValue::ValueType(ValueType::Text),
            when: crate::CliCondition {
                parameter: crate::CliParameterId::Pattern,
                values: vec![crate::CliValue::PatternMode(PatternMode::Regex)],
            },
        },
        crate::CliConditionalDefault {
            value: crate::CliValue::OutputMode(crate::CliOutputMode::Json),
            when: crate::CliCondition {
                parameter: crate::CliParameterId::Output,
                values: vec![crate::CliValue::OutputMode(crate::CliOutputMode::Html)],
            },
        },
    ];
    contract.parameters = vec![
        crate::CliParameterDescriptor {
            section: crate::CliParameterSection::Extraction,
            id: crate::CliParameterId::Output,
            kind: crate::CliParameterKind::Option,
            requirement: crate::CliParameterRequirement::Optional,
            value_hint: Some("<MODE>"),
            default: Some(crate::CliValue::OutputMode(crate::CliOutputMode::Json)),
            allowed_values: vec![crate::CliValue::OutputMode(crate::CliOutputMode::Text)],
            summary: "output",
        },
        crate::CliParameterDescriptor {
            section: crate::CliParameterSection::Extraction,
            id: crate::CliParameterId::Output,
            kind: crate::CliParameterKind::Option,
            requirement: crate::CliParameterRequirement::Optional,
            value_hint: Some("<MODE>"),
            default: None,
            allowed_values: vec![crate::CliValue::OutputMode(crate::CliOutputMode::Text)],
            summary: "duplicate output",
        },
        crate::CliParameterDescriptor {
            section: crate::CliParameterSection::Selection,
            id: crate::CliParameterId::Match,
            kind: crate::CliParameterKind::Option,
            requirement: crate::CliParameterRequirement::Optional,
            value_hint: Some("<MATCH>"),
            default: Some(crate::CliValue::SelectionMode(
                crate::CliSelectionMode::Single,
            )),
            allowed_values: vec![crate::CliValue::SelectionMode(
                crate::CliSelectionMode::First,
            )],
            summary: "match",
        },
        crate::CliParameterDescriptor {
            section: crate::CliParameterSection::Extraction,
            id: crate::CliParameterId::Value,
            kind: crate::CliParameterKind::Option,
            requirement: crate::CliParameterRequirement::Optional,
            value_hint: Some("<VALUE>"),
            default: Some(crate::CliValue::ValueType(ValueType::Structured)),
            allowed_values: vec![crate::CliValue::ValueType(ValueType::Text)],
            summary: "value",
        },
        crate::CliParameterDescriptor {
            section: crate::CliParameterSection::Extraction,
            id: crate::CliParameterId::RewriteUrls,
            kind: crate::CliParameterKind::Flag,
            requirement: crate::CliParameterRequirement::Optional,
            value_hint: Some("<BOOL>"),
            default: Some(crate::CliValue::Boolean(true)),
            allowed_values: Vec::new(),
            summary: "rewrite URLs",
        },
        crate::CliParameterDescriptor {
            section: crate::CliParameterSection::Extraction,
            id: crate::CliParameterId::Bundle,
            kind: crate::CliParameterKind::Option,
            requirement: crate::CliParameterRequirement::RequiredUnless(crate::CliParameterId::Css),
            value_hint: Some("<DIR>"),
            default: None,
            allowed_values: Vec::new(),
            summary: "bundle",
        },
        crate::CliParameterDescriptor {
            section: crate::CliParameterSection::Extraction,
            id: crate::CliParameterId::Attribute,
            kind: crate::CliParameterKind::Option,
            requirement: crate::CliParameterRequirement::AllowedOnlyWhen(crate::CliCondition {
                parameter: crate::CliParameterId::Bundle,
                values: vec![crate::CliValue::Boolean(true)],
            }),
            value_hint: Some("<NAME>"),
            default: None,
            allowed_values: Vec::new(),
            summary: "attribute",
        },
    ];
    contract.constraints = vec![
        crate::CliConstraint::RequiresParameter {
            parameter: crate::CliParameterId::Css,
            when: crate::CliCondition {
                parameter: crate::CliParameterId::Output,
                values: vec![crate::CliValue::OutputMode(crate::CliOutputMode::Text)],
            },
        },
        crate::CliConstraint::RestrictsParameterValues {
            parameter: crate::CliParameterId::Css,
            allowed_values: vec![crate::CliValue::OutputMode(crate::CliOutputMode::None)],
            when: crate::CliCondition {
                parameter: crate::CliParameterId::Output,
                values: vec![crate::CliValue::OutputMode(crate::CliOutputMode::Text)],
            },
        },
        crate::CliConstraint::RestrictsParameterValues {
            parameter: crate::CliParameterId::Output,
            allowed_values: vec![crate::CliValue::OutputMode(crate::CliOutputMode::None)],
            when: crate::CliCondition {
                parameter: crate::CliParameterId::Output,
                values: vec![crate::CliValue::OutputMode(crate::CliOutputMode::Html)],
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
        &crate::CliCondition {
            parameter: crate::CliParameterId::Pattern,
            values: vec![crate::CliValue::PatternMode(PatternMode::Regex)],
        },
        &contract.parameters,
    );
    assert!(
        missing_condition_errors
            .iter()
            .any(|error| error.contains("references missing condition parameter --pattern")),
        "missing explicit condition-lint error: {missing_condition_errors:#?}"
    );

    let mut match_drift = crate::cli_operation_contract(OperationId::SelectExtract)
        .expect("select contract")
        .clone();
    match_drift.selection_modes = vec![crate::CliSelectionMode::All];
    let match_drift_errors = crate::cli_contract::validate_command_contract_for_tests(&match_drift);
    assert!(
        match_drift_errors
            .iter()
            .any(|error| error.contains("parameter --match drifted from selection_modes")),
        "missing selection-mode drift error: {match_drift_errors:#?}"
    );

    let mut value_drift = crate::cli_operation_contract(OperationId::SelectExtract)
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

    let mut output_drift = crate::cli_operation_contract(OperationId::SelectExtract)
        .expect("select contract")
        .clone();
    output_drift.output_modes = vec![crate::CliOutputMode::Json];
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
    let base_contract = crate::cli_operation_contract(OperationId::SelectExtract)
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
        .retain(|parameter| parameter.id != crate::CliParameterId::Output);
    let without_output_parameter_errors =
        crate::cli_contract::validate_command_contract_for_tests(&without_output_parameter);
    assert!(
        without_output_parameter_errors.is_empty(),
        "unexpected errors for a contract without --output: {without_output_parameter_errors:#?}"
    );

    let mut empty_target_allowed_values = base_contract;
    empty_target_allowed_values.constraints =
        vec![crate::CliConstraint::RestrictsParameterValues {
            parameter: crate::CliParameterId::Bundle,
            allowed_values: vec![],
            when: crate::CliCondition {
                parameter: crate::CliParameterId::Output,
                values: vec![crate::CliValue::OutputMode(crate::CliOutputMode::Json)],
            },
        }];
    let empty_target_allowed_values_errors =
        crate::cli_contract::validate_command_contract_for_tests(&empty_target_allowed_values);
    assert!(
        empty_target_allowed_values_errors.is_empty(),
        "unexpected errors for empty restriction targets: {empty_target_allowed_values_errors:#?}"
    );
}
