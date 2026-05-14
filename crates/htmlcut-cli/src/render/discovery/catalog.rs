use crate::metadata::DISPLAY_NAME;
use crate::model::{CatalogAvailability, CatalogCommandReport, CatalogContractSurface};

use super::shared::render_schema_ref;

pub(crate) fn render_catalog_text(report: &CatalogCommandReport) -> String {
    let catalog_command =
        crate::contract::cli_aux_command_display_command(crate::contract::CliAuxCommandId::Catalog);
    let mut lines = vec![
        format!("{DISPLAY_NAME} {}", report.version),
        report.description.clone(),
    ];

    let operation_count = report.operations.len();
    lines.push(format!(
        "Catalog: {operation_count} operation{}.",
        if operation_count == 1 { "" } else { "s" }
    ));
    lines.push(
        format!(
            "Use `htmlcut {catalog_command} --operation <OPERATION_ID>` for one compact contract or `--output json` for the full machine-readable surface."
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
    let mut lines = vec![format!(
        "- {} | {}",
        operation.operation_id,
        render_catalog_surface(operation.command.as_deref(), &operation.availability)
    )];
    lines.push(format!("  {}", operation.summary));
    lines.push(format!(
        "  engine capability: {}",
        operation.engine_capability
    ));
    lines.extend(render_catalog_contract_surface_lines(
        "request",
        &operation.request_contract,
    ));
    lines.extend(render_catalog_contract_surface_lines(
        "result",
        &operation.result_contract,
    ));
    if operation.command_contract.is_some() && operation.command.is_some() {
        lines.push(
            "  Use `--output json` for parameters, defaults, constraints, and examples.".to_owned(),
        );
    }

    lines
}
fn render_catalog_contract_surface_lines(
    label: &str,
    contract: &CatalogContractSurface,
) -> Vec<String> {
    let mut lines = vec![format!("  {label}: {}", contract.artifact)];
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

pub(crate) fn render_catalog_surface(
    command: Option<&str>,
    availability: &CatalogAvailability,
) -> String {
    match (command, availability) {
        (Some(command), _) => command.to_owned(),
        (None, CatalogAvailability::EngineOnly) => "engine only".to_owned(),
        (None, CatalogAvailability::Cli) => "cli".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{CatalogContractSurface, CatalogOperationReport};

    #[test]
    fn catalog_surface_rendering_covers_cli_without_explicit_command() {
        assert_eq!(
            render_catalog_surface(None, &CatalogAvailability::Cli),
            "cli"
        );
        assert_eq!(
            render_catalog_surface(None, &CatalogAvailability::EngineOnly),
            "engine only"
        );
        assert_eq!(
            render_catalog_surface(Some("inspect source"), &CatalogAvailability::Cli),
            "inspect source"
        );
    }

    #[test]
    fn catalog_operation_rendering_skips_optional_sections_when_contracts_are_sparse() {
        let lines = render_catalog_operation_lines(&CatalogOperationReport {
            operation_id: htmlcut_core::OperationId::DocumentParse,
            command: None,
            availability: CatalogAvailability::EngineOnly,
            summary: "Parse a document".to_owned(),
            engine_capability: "parse_document(SourceRequest, RuntimeOptions)".to_owned(),
            request_contract: CatalogContractSurface {
                artifact: "SourceRequest + RuntimeOptions".to_owned(),
                schema_refs: Vec::new(),
            },
            result_contract: CatalogContractSurface {
                artifact: "ParseDocumentResult".to_owned(),
                schema_refs: Vec::new(),
            },
            command_contract: Some(crate::model::CatalogCommandContract {
                invocation: "htmlcut parse".to_owned(),
                inputs: Vec::new(),
                default_match: None,
                selection_modes: Vec::new(),
                default_value: None,
                value_modes: Vec::new(),
                default_output: None,
                default_output_overrides: Vec::new(),
                output_modes: Vec::new(),
                constraints: Vec::new(),
                notes: Vec::new(),
                examples: Vec::new(),
                parameters: Vec::new(),
            }),
        });

        assert!(
            lines
                .iter()
                .any(|line| line == "  request: SourceRequest + RuntimeOptions")
        );
        assert!(lines.iter().all(|line| !line.contains("schemas:")));
        assert!(
            lines
                .iter()
                .all(|line| !line.contains("Use `--output json` for parameters"))
        );
    }
}
