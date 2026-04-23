use crate::model::{
    CatalogAvailability, CatalogCommandContract, CatalogCommandReport, CatalogCondition,
    CatalogConstraint, CatalogContractSurface, CatalogParameterKind, CatalogParameterRequirement,
};

use super::shared::render_schema_ref;

pub(crate) fn render_catalog_text(report: &CatalogCommandReport) -> String {
    let catalog_command =
        htmlcut_core::cli_aux_command_display_command(htmlcut_core::CliAuxCommandId::Catalog);
    let mut lines = vec![
        format!("{} {}", report.tool, report.version),
        report.description.clone(),
    ];

    let operation_count = report.operations.len();
    lines.push(format!(
        "Catalog: {operation_count} operation{}.",
        if operation_count == 1 { "" } else { "s" }
    ));
    lines.push(
        format!(
            "Use `htmlcut {catalog_command} --operation <OPERATION_ID> --output json` for one exact contract."
        ),
    );

    if report.operations.is_empty() {
        return lines.join("\n");
    }

    lines.push(if report.operations.len() == 1 {
        "Operation:".to_owned()
    } else {
        "Operations:".to_owned()
    });

    for (index, operation) in report.operations.iter().enumerate() {
        if index > 0 {
            lines.push(String::new());
        }
        lines.extend(render_catalog_operation_lines(operation));
    }

    lines.join("\n")
}

fn render_catalog_operation_lines(operation: &crate::model::CatalogOperationReport) -> Vec<String> {
    let mut lines = vec![
        format!(
            "- {} | {}",
            operation.operation_id,
            render_catalog_surface(operation.command.as_deref(), &operation.availability)
        ),
        format!("  {}", operation.summary),
        format!("  core: {}", operation.core_surface),
    ];
    lines.extend(render_catalog_contract_surface_lines(
        "request",
        &operation.request_contract,
    ));
    lines.extend(render_catalog_contract_surface_lines(
        "result",
        &operation.result_contract,
    ));
    if let Some(command_contract) = operation.command_contract.as_ref() {
        lines.extend(render_catalog_contract_lines(command_contract));
    }

    lines
}

fn render_catalog_contract_lines(contract: &CatalogCommandContract) -> Vec<String> {
    let mut lines = vec![format!("  usage: {}", contract.invocation)];

    push_joined_catalog_line(&mut lines, "inputs", &contract.inputs, " | ");
    push_optional_catalog_line(
        &mut lines,
        "default match",
        contract.default_match.as_deref(),
    );
    push_joined_catalog_line(&mut lines, "match modes", &contract.selection_modes, ", ");
    push_optional_catalog_line(
        &mut lines,
        "default value",
        contract.default_value.as_deref(),
    );
    push_joined_catalog_line(&mut lines, "value modes", &contract.value_modes, ", ");
    push_optional_catalog_line(
        &mut lines,
        "default output",
        contract.default_output.as_deref(),
    );
    if !contract.default_output_overrides.is_empty() {
        lines.push("  default output overrides:".to_owned());
        lines.extend(
            contract
                .default_output_overrides
                .iter()
                .map(|override_spec| {
                    format!(
                        "  - when {} => {}",
                        render_catalog_condition(&override_spec.when),
                        override_spec.value
                    )
                }),
        );
    }
    push_joined_catalog_line(&mut lines, "output modes", &contract.output_modes, ", ");
    if !contract.constraints.is_empty() {
        lines.push("  constraints:".to_owned());
        lines.extend(
            contract
                .constraints
                .iter()
                .map(render_catalog_constraint_line),
        );
    }
    if !contract.notes.is_empty() {
        lines.push("  notes:".to_owned());
        lines.extend(contract.notes.iter().map(|note| format!("  - {note}")));
    }
    if !contract.examples.is_empty() {
        lines.push("  examples:".to_owned());
        lines.extend(
            contract
                .examples
                .iter()
                .map(|example| format!("  - {example}")),
        );
    }
    if !contract.parameters.is_empty() {
        lines.push("  parameters:".to_owned());
        for parameter in &contract.parameters {
            lines.push(format!(
                "  - {} | {} {} | {}",
                parameter.section,
                render_parameter_kind(&parameter.kind),
                render_parameter_name(parameter),
                render_parameter_requirement(parameter)
            ));
            lines.push(format!("    {}", parameter.summary));
            if let Some(default) = parameter.default.as_deref() {
                lines.push(format!("    default: {default}"));
            }
            if !parameter.allowed_values.is_empty() {
                lines.push(format!(
                    "    values: {}",
                    parameter.allowed_values.join(", ")
                ));
            }
        }
    }

    lines
}

fn push_joined_catalog_line(
    lines: &mut Vec<String>,
    label: &str,
    values: &[String],
    separator: &str,
) {
    if !values.is_empty() {
        lines.push(format!("  {label}: {}", values.join(separator)));
    }
}

fn push_optional_catalog_line(lines: &mut Vec<String>, label: &str, value: Option<&str>) {
    if let Some(value) = value {
        lines.push(format!("  {label}: {value}"));
    }
}

fn render_catalog_constraint_line(constraint: &CatalogConstraint) -> String {
    match constraint {
        CatalogConstraint::RequiresParameter { parameter, when } => {
            format!(
                "  - requires {parameter} when {}",
                render_catalog_condition(when)
            )
        }
        CatalogConstraint::AllowedOnlyWhen { parameter, when } => format!(
            "  - allows {parameter} only when {}",
            render_catalog_condition(when)
        ),
        CatalogConstraint::RestrictsParameterValues {
            parameter,
            allowed_values,
            when,
        } => format!(
            "  - restricts {parameter} to {} when {}",
            allowed_values.join(", "),
            render_catalog_condition(when)
        ),
    }
}

fn render_catalog_condition(condition: &CatalogCondition) -> String {
    if condition.values.is_empty() {
        return condition.parameter.clone();
    }

    format!(
        "{} is {}",
        condition.parameter,
        condition.values.join(" or ")
    )
}

fn render_catalog_contract_surface_lines(
    label: &str,
    contract: &CatalogContractSurface,
) -> Vec<String> {
    let mut lines = vec![format!("  {label}: {}", contract.rust_shape)];
    if !contract.schema_refs.is_empty() {
        lines.push(format!(
            "  {label} schemas: {}",
            contract
                .schema_refs
                .iter()
                .map(render_schema_ref)
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    lines
}

fn render_parameter_kind(kind: &CatalogParameterKind) -> &'static str {
    match kind {
        CatalogParameterKind::Positional => "positional",
        CatalogParameterKind::Option => "option",
        CatalogParameterKind::Flag => "flag",
    }
}

fn render_parameter_name(parameter: &crate::model::CatalogParameterSpec) -> String {
    match parameter.value_hint.as_deref() {
        Some(value_hint) if parameter.kind == CatalogParameterKind::Option => {
            format!("{} <{value_hint}>", parameter.name)
        }
        _ => parameter.name.clone(),
    }
}

fn render_parameter_requirement(parameter: &crate::model::CatalogParameterSpec) -> String {
    match parameter.requirement {
        CatalogParameterRequirement::Required => "required".to_owned(),
        CatalogParameterRequirement::Optional => "optional".to_owned(),
        CatalogParameterRequirement::Conditional => format!(
            "conditional ({})",
            parameter
                .requirement_note
                .as_deref()
                .unwrap_or("see command notes")
        ),
    }
}

pub(crate) fn render_catalog_surface(
    command: Option<&str>,
    availability: &CatalogAvailability,
) -> String {
    match (command, availability) {
        (Some(command), _) => command.to_owned(),
        (None, CatalogAvailability::CoreOnly) => "core only".to_owned(),
        (None, CatalogAvailability::Cli) => "cli".to_owned(),
    }
}
