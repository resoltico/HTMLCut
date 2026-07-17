use super::*;

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
