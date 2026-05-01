use crate::error::CliError;
use crate::lookup::unknown_operation_id_error;
use crate::metadata::{HTMLCUT_DESCRIPTION, HTMLCUT_VERSION, TOOL_NAME};
use crate::model::{
    CATALOG_REPORT_SCHEMA_NAME, CATALOG_SCHEMA_VERSION, CatalogAvailability,
    CatalogCommandContract, CatalogCommandReport, CatalogCondition, CatalogConditionalDefault,
    CatalogConstraint, CatalogContractSurface, CatalogOperationReport, CatalogParameterKind,
    CatalogParameterRequirement, CatalogParameterSpec, SchemaRefReport,
};

pub(crate) fn build_catalog_report(
    operation_filter: Option<&str>,
) -> Result<CatalogCommandReport, CliError> {
    let requested_operation = operation_filter
        .map(|operation_id| {
            operation_id
                .parse::<htmlcut_core::OperationId>()
                .map_err(|_| unknown_operation_id_error(operation_id))
        })
        .transpose()?;

    let operations = htmlcut_core::operation_catalog()
        .iter()
        .filter(|descriptor| {
            requested_operation.is_none_or(|operation_id| descriptor.id == operation_id)
        })
        .map(|descriptor| {
            let cli_contract = crate::contract::cli_operation_contract(descriptor.id);
            CatalogOperationReport {
                operation_id: descriptor.id,
                command: cli_contract.map(|contract| contract.display_command()),
                availability: match cli_contract {
                    Some(_) => CatalogAvailability::Cli,
                    None => CatalogAvailability::CoreOnly,
                },
                summary: descriptor.description.to_owned(),
                core_surface: descriptor.core_surface.to_owned(),
                request_contract: build_contract_surface(&descriptor.request_contract),
                result_contract: build_contract_surface(&descriptor.result_contract),
                command_contract: cli_contract.map(build_catalog_command_contract),
            }
        })
        .collect::<Vec<_>>();

    Ok(CatalogCommandReport {
        tool: TOOL_NAME.to_owned(),
        version: HTMLCUT_VERSION.to_owned(),
        schema_name: CATALOG_REPORT_SCHEMA_NAME.to_owned(),
        schema_version: CATALOG_SCHEMA_VERSION,
        schema_profile: htmlcut_core::HTMLCUT_JSON_SCHEMA_PROFILE.to_owned(),
        description: HTMLCUT_DESCRIPTION.to_owned(),
        command: "catalog".to_owned(),
        operations,
    })
}

fn build_contract_surface(contract: &htmlcut_core::OperationContract) -> CatalogContractSurface {
    CatalogContractSurface {
        rust_shape: contract.rust_shape.to_owned(),
        schema_refs: contract
            .schema_refs
            .iter()
            .map(build_schema_ref_report)
            .collect(),
    }
}

fn build_schema_ref_report(schema_ref: &htmlcut_core::SchemaRef) -> SchemaRefReport {
    SchemaRefReport {
        schema_name: schema_ref.schema_name.to_owned(),
        schema_version: schema_ref.schema_version,
    }
}

fn build_catalog_command_contract(
    descriptor: &crate::contract::OperationCliContract,
) -> CatalogCommandContract {
    CatalogCommandContract {
        invocation: descriptor.invocation.to_owned(),
        inputs: descriptor
            .inputs
            .iter()
            .copied()
            .map(|input| input.description().to_owned())
            .collect(),
        default_match: descriptor.default_match.map(render_selection_mode),
        selection_modes: descriptor
            .selection_modes
            .iter()
            .copied()
            .map(render_selection_mode)
            .collect(),
        default_value: descriptor.default_value.map(render_value_type),
        value_modes: descriptor
            .value_modes
            .iter()
            .copied()
            .map(render_value_type)
            .collect(),
        default_output: descriptor.default_output.map(render_output_mode),
        default_output_overrides: descriptor
            .default_output_overrides
            .iter()
            .map(build_conditional_default)
            .collect(),
        output_modes: descriptor
            .output_modes
            .iter()
            .copied()
            .map(render_output_mode)
            .collect(),
        constraints: descriptor
            .constraints
            .iter()
            .map(build_constraint)
            .collect(),
        notes: descriptor
            .notes
            .iter()
            .map(|note| (*note).to_owned())
            .collect(),
        examples: descriptor
            .examples
            .iter()
            .map(|example| (*example).to_owned())
            .collect(),
        parameters: descriptor
            .parameters
            .iter()
            .map(build_parameter_spec)
            .collect(),
    }
}

fn build_conditional_default(
    descriptor: &crate::contract::CliConditionalDefault,
) -> CatalogConditionalDefault {
    CatalogConditionalDefault {
        value: crate::contract::render_cli_value(descriptor.value),
        when: build_condition(&descriptor.when),
    }
}

fn build_constraint(descriptor: &crate::contract::CliConstraint) -> CatalogConstraint {
    match descriptor {
        crate::contract::CliConstraint::RequiresParameter { parameter, when } => {
            CatalogConstraint::RequiresParameter {
                parameter: parameter.to_string(),
                when: build_condition(when),
            }
        }
        crate::contract::CliConstraint::AllowedOnlyWhen { parameter, when } => {
            CatalogConstraint::AllowedOnlyWhen {
                parameter: parameter.to_string(),
                when: build_condition(when),
            }
        }
        crate::contract::CliConstraint::RestrictsParameterValues {
            parameter,
            allowed_values,
            when,
        } => CatalogConstraint::RestrictsParameterValues {
            parameter: parameter.to_string(),
            allowed_values: allowed_values
                .iter()
                .copied()
                .map(crate::contract::render_cli_value)
                .collect(),
            when: build_condition(when),
        },
    }
}

fn build_condition(condition: &crate::contract::CliCondition) -> CatalogCondition {
    CatalogCondition {
        parameter: condition.parameter.to_string(),
        values: condition
            .values
            .iter()
            .copied()
            .map(crate::contract::render_cli_value)
            .collect(),
    }
}

fn build_parameter_spec(
    parameter: &crate::contract::CliParameterDescriptor,
) -> CatalogParameterSpec {
    let (requirement, requirement_note) = render_parameter_requirement(&parameter.requirement);
    CatalogParameterSpec {
        section: parameter.section.to_string(),
        name: parameter.id.to_string(),
        kind: match parameter.kind {
            crate::contract::CliParameterKind::Positional => CatalogParameterKind::Positional,
            crate::contract::CliParameterKind::Option => CatalogParameterKind::Option,
            crate::contract::CliParameterKind::Flag => CatalogParameterKind::Flag,
        },
        requirement,
        requirement_note,
        value_hint: parameter.value_hint.map(str::to_owned),
        default: parameter.default.map(crate::contract::render_cli_value),
        allowed_values: parameter
            .allowed_values
            .iter()
            .copied()
            .map(crate::contract::render_cli_value)
            .collect(),
        summary: parameter.summary.to_owned(),
    }
}

fn render_parameter_requirement(
    requirement: &crate::contract::CliParameterRequirement,
) -> (CatalogParameterRequirement, Option<String>) {
    match requirement {
        crate::contract::CliParameterRequirement::Required => {
            (CatalogParameterRequirement::Required, None)
        }
        crate::contract::CliParameterRequirement::Optional => {
            (CatalogParameterRequirement::Optional, None)
        }
        crate::contract::CliParameterRequirement::RequiredUnless(parameter) => (
            CatalogParameterRequirement::Conditional,
            Some(format!("required unless {parameter} is used")),
        ),
        crate::contract::CliParameterRequirement::RequiredWhen(condition) => (
            CatalogParameterRequirement::Conditional,
            Some(format!(
                "required when {}",
                render_condition_expression(condition)
            )),
        ),
        crate::contract::CliParameterRequirement::AllowedOnlyWhen(condition) => (
            CatalogParameterRequirement::Conditional,
            Some(format!(
                "allowed only when {}",
                render_condition_expression(condition)
            )),
        ),
    }
}

fn render_condition_expression(condition: &crate::contract::CliCondition) -> String {
    let values = condition
        .values
        .iter()
        .copied()
        .map(crate::contract::render_cli_value)
        .collect::<Vec<_>>();

    match values.as_slice() {
        [single] => format!("{} {single} is used", condition.parameter),
        _ => format!("{} is one of {}", condition.parameter, values.join(", ")),
    }
}

#[cfg(test)]
pub(crate) fn render_condition_expression_for_tests(
    condition: &crate::contract::CliCondition,
) -> String {
    render_condition_expression(condition)
}

fn render_selection_mode(mode: crate::contract::CliSelectionMode) -> String {
    crate::contract::render_cli_value(crate::contract::CliValue::SelectionMode(mode))
}

fn render_value_type(value_type: htmlcut_core::ValueType) -> String {
    crate::contract::render_cli_value(crate::contract::CliValue::ValueType(value_type))
}

fn render_output_mode(mode: crate::contract::CliOutputMode) -> String {
    crate::contract::render_cli_value(crate::contract::CliValue::OutputMode(mode))
}
