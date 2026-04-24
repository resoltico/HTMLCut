use htmlcut_core::cli_contract::{
    CliHelpDocument, CliHelpSection, CliHelpSectionStyle, OperationCliContract,
};

use crate::error::CliError;
use crate::lookup;

pub(super) fn build_operation_long_about(
    operation_id: htmlcut_core::OperationId,
) -> Result<String, CliError> {
    build_operation_long_about_from_sources(
        operation_contract(operation_id),
        lookup::operation_help_document(operation_id),
    )
}

fn build_operation_long_about_from_sources(
    contract: Result<&'static OperationCliContract, CliError>,
    document: Result<CliHelpDocument, CliError>,
) -> Result<String, CliError> {
    let contract = contract?;
    let sections = document?.sections;
    Ok(build_operation_long_about_from_parts(sections, contract))
}

pub(super) fn build_operation_long_about_from_parts(
    mut sections: Vec<CliHelpSection>,
    contract: &OperationCliContract,
) -> String {
    let mode_summary = render_contract_mode_summary(contract);
    if !mode_summary.is_empty() {
        sections.push(CliHelpSection {
            title: "Modes".to_owned(),
            style: CliHelpSectionStyle::Plain,
            lines: mode_summary.lines().map(str::to_owned).collect(),
        });
    }
    if !contract.notes.is_empty() {
        sections.push(CliHelpSection {
            title: "Notes".to_owned(),
            style: CliHelpSectionStyle::Bullets,
            lines: contract
                .notes
                .iter()
                .map(|note| (*note).to_owned())
                .collect(),
        });
    }

    render_help_sections(&sections)
}

pub(super) fn operation_examples_after_help(
    operation_id: htmlcut_core::OperationId,
) -> Result<String, CliError> {
    operation_examples_after_help_from_document(lookup::operation_help_document(operation_id))
}

fn operation_examples_after_help_from_document(
    document: Result<CliHelpDocument, CliError>,
) -> Result<String, CliError> {
    Ok(render_help_examples(&document?))
}

pub(super) fn render_help_examples(document: &CliHelpDocument) -> String {
    format!("Examples:\n  {}", document.examples.join("\n  "))
}

pub(super) fn render_help_sections(sections: &[CliHelpSection]) -> String {
    sections
        .iter()
        .filter(|section| !section.lines.is_empty())
        .map(render_help_section)
        .collect::<Vec<_>>()
        .join("\n\n")
}

pub(super) fn render_help_section(section: &CliHelpSection) -> String {
    let body = match section.style {
        CliHelpSectionStyle::Plain => section.lines.join("\n"),
        CliHelpSectionStyle::Bullets => section
            .lines
            .iter()
            .map(|line| format!("- {line}"))
            .collect::<Vec<_>>()
            .join("\n"),
        CliHelpSectionStyle::Numbered => section
            .lines
            .iter()
            .enumerate()
            .map(|(index, line)| format!("{}. {line}", index + 1))
            .collect::<Vec<_>>()
            .join("\n"),
    };

    format!("{}:\n{}", section.title, body)
}

pub(super) fn render_contract_mode_summary(contract: &OperationCliContract) -> String {
    let mut lines = Vec::new();

    if let Some(default_match) = contract.default_match {
        lines.push(format!(
            "Default match mode: {}.",
            htmlcut_core::cli_contract::render_cli_value(
                htmlcut_core::cli_contract::CliValue::SelectionMode(default_match)
            )
        ));
    }
    if !contract.selection_modes.is_empty() {
        lines.push(format!(
            "Supported match modes: {}.",
            join_cli_values(
                contract
                    .selection_modes
                    .iter()
                    .copied()
                    .map(htmlcut_core::cli_contract::CliValue::SelectionMode)
            )
        ));
    }
    if let Some(default_value) = contract.default_value {
        lines.push(format!(
            "Default value mode: {}.",
            htmlcut_core::cli_contract::render_cli_value(
                htmlcut_core::cli_contract::CliValue::ValueType(default_value)
            )
        ));
    }
    if !contract.value_modes.is_empty() {
        lines.push(format!(
            "Supported value modes: {}.",
            join_cli_values(
                contract
                    .value_modes
                    .iter()
                    .copied()
                    .map(htmlcut_core::cli_contract::CliValue::ValueType)
            )
        ));
    }
    if let Some(default_output) = contract.default_output {
        lines.push(format!(
            "Default output mode: {}.",
            htmlcut_core::cli_contract::render_cli_value(
                htmlcut_core::cli_contract::CliValue::OutputMode(default_output)
            )
        ));
    }
    for output_override in &contract.default_output_overrides {
        let condition_values = join_cli_values(output_override.when.values.iter().copied());
        let verb = if output_override.when.values.len() == 1 {
            "is"
        } else {
            "is one of"
        };
        lines.push(format!(
            "Output default override: {} when {} {} {}.",
            htmlcut_core::cli_contract::render_cli_value(output_override.value),
            output_override.when.parameter,
            verb,
            condition_values
        ));
    }
    if !contract.output_modes.is_empty() {
        lines.push(format!(
            "Supported output modes: {}.",
            join_cli_values(
                contract
                    .output_modes
                    .iter()
                    .copied()
                    .map(htmlcut_core::cli_contract::CliValue::OutputMode)
            )
        ));
    }

    lines.join("\n")
}

fn join_cli_values(
    values: impl IntoIterator<Item = htmlcut_core::cli_contract::CliValue>,
) -> String {
    values
        .into_iter()
        .map(htmlcut_core::cli_contract::render_cli_value)
        .collect::<Vec<_>>()
        .join(", ")
}

fn operation_contract(
    operation_id: htmlcut_core::OperationId,
) -> Result<&'static htmlcut_core::cli_contract::OperationCliContract, CliError> {
    lookup::operation_contract(operation_id)
}

#[cfg(test)]
pub(crate) fn build_operation_long_about_from_sources_for_tests(
    contract: Result<&'static OperationCliContract, CliError>,
    document: Result<CliHelpDocument, CliError>,
) -> Result<String, CliError> {
    build_operation_long_about_from_sources(contract, document)
}

#[cfg(test)]
pub(crate) fn operation_examples_after_help_from_document_for_tests(
    document: Result<CliHelpDocument, CliError>,
) -> Result<String, CliError> {
    operation_examples_after_help_from_document(document)
}
