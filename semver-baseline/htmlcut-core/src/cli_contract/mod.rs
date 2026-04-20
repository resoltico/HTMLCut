use std::sync::LazyLock;

use serde::Serialize;

use crate::catalog::OperationId;
#[cfg(test)]
use crate::catalog::{operation_catalog, operation_descriptor};
#[cfg(test)]
use std::collections::BTreeSet;

mod help;
mod operations;
mod parameters;
mod types;

pub use help::{
    CliAuxCommandDescriptor, CliAuxCommandId, CliHelpDocument, CliHelpSection, CliHelpSectionStyle,
    cli_aux_command_catalog, cli_aux_command_descriptor, cli_aux_command_display_command,
    cli_aux_command_help_document, cli_operation_help_document, cli_root_help_document,
};
pub use types::{
    CliCondition, CliConditionalDefault, CliConstraint, CliInputForm, CliOutputMode,
    CliParameterDescriptor, CliParameterId, CliParameterKind, CliParameterRequirement,
    CliParameterSection, CliSelectionMode, CliValue, OperationCliContract,
};

static CLI_OPERATION_CATALOG: LazyLock<Vec<OperationCliContract>> =
    LazyLock::new(operations::build_cli_operation_catalog);

/// Returns the canonical CLI operation-contract catalog.
pub fn cli_operation_catalog() -> &'static [OperationCliContract] {
    CLI_OPERATION_CATALOG.as_slice()
}

/// Returns the canonical CLI command contract for one operation, when the CLI exposes it.
pub fn cli_operation_contract(operation_id: OperationId) -> Option<&'static OperationCliContract> {
    cli_operation_catalog()
        .iter()
        .find(|contract| contract.operation_id == operation_id)
}

/// Returns the canonical display-form CLI command for one operation, when the CLI exposes it.
pub fn cli_operation_display_command(operation_id: OperationId) -> Option<String> {
    cli_operation_contract(operation_id).map(OperationCliContract::display_command)
}

/// Returns the canonical report-form CLI command for one operation, when the CLI exposes it.
pub fn cli_operation_report_command(operation_id: OperationId) -> Option<String> {
    cli_operation_contract(operation_id).map(OperationCliContract::report_command)
}

/// Finds the canonical CLI operation contract that matches one concrete command path.
pub fn find_cli_operation_by_command_path(
    command_path: &[&str],
) -> Option<&'static OperationCliContract> {
    cli_operation_catalog()
        .iter()
        .find(|contract| contract.command_path == command_path)
}

/// Renders one typed CLI contract value as the stable user-facing string form.
pub fn render_cli_value(value: CliValue) -> String {
    match value {
        CliValue::SelectionMode(mode) => render_enum_name(mode),
        CliValue::ValueType(value_type) => render_enum_name(value_type),
        CliValue::OutputMode(mode) => render_enum_name(mode),
        CliValue::WhitespaceMode(mode) => render_enum_name(mode),
        CliValue::PatternMode(mode) => render_enum_name(mode),
        CliValue::FetchPreflightMode(mode) => render_enum_name(mode),
        CliValue::Boolean(value) => value.to_string(),
        CliValue::Usize(value) => value.to_string(),
        CliValue::U64(value) => value.to_string(),
    }
}

#[cfg(test)]
pub(crate) fn cli_operation_catalog_validation_errors() -> Vec<String> {
    cli_operation_catalog_validation_errors_for(operation_catalog(), cli_operation_catalog())
}

#[cfg(test)]
pub(crate) fn cli_operation_catalog_validation_errors_for(
    operation_descriptors: &[crate::catalog::OperationDescriptor],
    cli_contracts: &[OperationCliContract],
) -> Vec<String> {
    let mut errors = Vec::new();
    let mut seen_ids = BTreeSet::new();
    let cli_ids = cli_contracts
        .iter()
        .map(|contract| contract.operation_id)
        .collect::<BTreeSet<_>>();

    for descriptor in operation_descriptors {
        match (descriptor.cli_surface, cli_ids.contains(&descriptor.id)) {
            (Some(_), false) => errors.push(format!(
                "{} is marked CLI-visible in OPERATION_CATALOG but missing from cli_operation_catalog()",
                descriptor.id
            )),
            (None, true) => errors.push(format!(
                "{} appears in cli_operation_catalog() but is marked core-only in OPERATION_CATALOG",
                descriptor.id
            )),
            (Some(_), true) | (None, false) => {}
        }
    }

    for contract in cli_contracts {
        if !seen_ids.insert(contract.operation_id) {
            errors.push(format!(
                "{} appears more than once in cli_operation_catalog()",
                contract.operation_id
            ));
        }

        let descriptor = operation_descriptor(contract.operation_id);
        let display_command = contract.display_command();
        if descriptor.cli_surface != Some(display_command.as_str()) {
            errors.push(format!(
                "{} display command drifted: OPERATION_CATALOG={:?}, cli contract={display_command:?}",
                contract.operation_id, descriptor.cli_surface
            ));
        }

        validate_command_contract(contract, &mut errors);
    }

    errors
}

#[cfg(test)]
pub(crate) fn cli_help_catalog_validation_errors() -> Vec<String> {
    help::cli_help_catalog_validation_errors()
}

#[cfg(test)]
pub(crate) fn validate_command_contract_for_tests(contract: &OperationCliContract) -> Vec<String> {
    let mut errors = Vec::new();
    validate_command_contract(contract, &mut errors);
    errors
}

#[cfg(test)]
pub(crate) fn validate_condition_for_tests(
    operation_id: OperationId,
    scope: &str,
    condition: &CliCondition,
    parameters: &[CliParameterDescriptor],
) -> Vec<String> {
    let mut errors = Vec::new();
    validate_condition(operation_id, scope, condition, parameters, &mut errors);
    errors
}

#[cfg(test)]
fn validate_command_contract(contract: &OperationCliContract, errors: &mut Vec<String>) {
    let scope = contract.operation_id;
    let display_command = contract.display_command();
    let parameter_ids = contract
        .parameters
        .iter()
        .map(|parameter| parameter.id)
        .collect::<BTreeSet<_>>();

    if contract.command_path.is_empty() {
        errors.push(format!("{scope} has an empty command path"));
    }

    if !contract
        .invocation
        .starts_with(&format!("htmlcut {display_command}"))
    {
        errors.push(format!(
            "{scope} invocation {:?} does not start with \"htmlcut {display_command}\"",
            contract.invocation
        ));
    }

    for example in &contract.examples {
        if !example.starts_with(&format!("htmlcut {display_command}")) {
            errors.push(format!(
                "{scope} example {example:?} does not start with \"htmlcut {display_command}\""
            ));
        }
    }

    if let Some(default_match) = contract.default_match
        && !contract.selection_modes.contains(&default_match)
    {
        errors.push(format!(
            "{scope} default_match {:?} is not present in selection_modes",
            render_cli_value(CliValue::SelectionMode(default_match))
        ));
    }

    if let Some(default_value) = contract.default_value
        && !contract.value_modes.contains(&default_value)
    {
        errors.push(format!(
            "{scope} default_value {:?} is not present in value_modes",
            render_cli_value(CliValue::ValueType(default_value))
        ));
    }

    if let Some(default_output) = contract.default_output
        && !contract.output_modes.contains(&default_output)
    {
        errors.push(format!(
            "{scope} default_output {:?} is not present in output_modes",
            render_cli_value(CliValue::OutputMode(default_output))
        ));
    }

    for override_default in &contract.default_output_overrides {
        if !contract
            .output_modes
            .contains(&match override_default.value {
                CliValue::OutputMode(mode) => mode,
                _ => {
                    errors.push(format!(
                        "{scope} default_output_override {:?} is not an output mode",
                        render_cli_value(override_default.value)
                    ));
                    continue;
                }
            })
        {
            errors.push(format!(
                "{scope} default_output_override {:?} is not present in output_modes",
                render_cli_value(override_default.value)
            ));
        }

        validate_condition(
            scope,
            "default_output_override",
            &override_default.when,
            &contract.parameters,
            errors,
        );
    }

    let mut seen_parameters = BTreeSet::new();
    for parameter in &contract.parameters {
        if !seen_parameters.insert(parameter.id) {
            errors.push(format!("{scope} lists {} more than once", parameter.id));
        }

        if matches!(parameter.kind, CliParameterKind::Flag) {
            if parameter.value_hint.is_some() {
                errors.push(format!(
                    "{scope} flag {} carries a value_hint",
                    parameter.id
                ));
            }
            if parameter.default != Some(CliValue::Boolean(false)) {
                errors.push(format!(
                    "{scope} flag {} should default to false in the catalog metadata",
                    parameter.id
                ));
            }
        }

        if !parameter.allowed_values.is_empty()
            && let Some(default) = parameter.default
            && !parameter.allowed_values.contains(&default)
        {
            errors.push(format!(
                "{scope} parameter {} default {:?} is not present in allowed_values",
                parameter.id,
                render_cli_value(default)
            ));
        }

        match &parameter.requirement {
            CliParameterRequirement::Required | CliParameterRequirement::Optional => {}
            CliParameterRequirement::RequiredUnless(other) => {
                if !parameter_ids.contains(other) {
                    errors.push(format!(
                        "{scope} parameter {} depends on missing parameter {}",
                        parameter.id, other
                    ));
                }
            }
            CliParameterRequirement::RequiredWhen(condition)
            | CliParameterRequirement::AllowedOnlyWhen(condition) => {
                validate_condition(
                    scope,
                    parameter.id.as_str(),
                    condition,
                    &contract.parameters,
                    errors,
                );
            }
        }
    }

    if let Some(parameter) = parameters::parameter_descriptor(contract, CliParameterId::Match)
        && parameter.allowed_values != parameters::selection_mode_values(&contract.selection_modes)
    {
        errors.push(format!(
            "{scope} parameter --match drifted from selection_modes"
        ));
    }

    if let Some(parameter) = parameters::parameter_descriptor(contract, CliParameterId::Value)
        && parameter.allowed_values != parameters::value_type_values(&contract.value_modes)
    {
        errors.push(format!(
            "{scope} parameter --value drifted from value_modes"
        ));
    }

    if let Some(parameter) = parameters::parameter_descriptor(contract, CliParameterId::Output)
        && parameter.allowed_values != parameters::output_mode_values(&contract.output_modes)
    {
        errors.push(format!(
            "{scope} parameter --output drifted from output_modes"
        ));
    }

    for constraint in &contract.constraints {
        match constraint {
            CliConstraint::RequiresParameter { parameter, when }
            | CliConstraint::AllowedOnlyWhen { parameter, when } => {
                if !parameter_ids.contains(parameter) {
                    errors.push(format!(
                        "{scope} constraint references missing parameter {parameter}"
                    ));
                }
                validate_condition(scope, "constraint", when, &contract.parameters, errors);
            }
            CliConstraint::RestrictsParameterValues {
                parameter,
                allowed_values,
                when,
            } => {
                if !parameter_ids.contains(parameter) {
                    errors.push(format!(
                        "{scope} value restriction references missing parameter {parameter}"
                    ));
                }
                validate_condition(scope, "constraint", when, &contract.parameters, errors);
                if let Some(target) = parameters::parameter_descriptor(contract, *parameter)
                    && !target.allowed_values.is_empty()
                    && !allowed_values
                        .iter()
                        .all(|value| target.allowed_values.contains(value))
                {
                    errors.push(format!(
                        "{scope} value restriction on {parameter} references values outside its allowed_values"
                    ));
                }
            }
        }
    }
}

#[cfg(test)]
fn validate_condition(
    operation_id: OperationId,
    scope: &str,
    condition: &CliCondition,
    parameters: &[CliParameterDescriptor],
    errors: &mut Vec<String>,
) {
    let Some(parameter) = parameters
        .iter()
        .find(|parameter| parameter.id == condition.parameter)
    else {
        errors.push(format!(
            "{operation_id} {scope} references missing condition parameter {}",
            condition.parameter
        ));
        return;
    };

    if parameter.allowed_values.is_empty() {
        errors.push(format!(
            "{operation_id} {scope} references condition parameter {} without allowed_values",
            condition.parameter
        ));
        return;
    }

    if !condition
        .values
        .iter()
        .all(|value| parameter.allowed_values.contains(value))
    {
        errors.push(format!(
            "{operation_id} {scope} references unsupported values for {}",
            condition.parameter
        ));
    }
}

fn render_enum_name<T: Serialize>(value: T) -> String {
    serde_json::to_string(&value)
        .expect("catalog enum values should always serialize")
        .trim_matches('"')
        .to_owned()
}
