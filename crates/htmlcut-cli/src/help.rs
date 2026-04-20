use std::sync::LazyLock;

use htmlcut_core::{CliAuxCommandId, CliHelpDocument, CliHelpSection, CliHelpSectionStyle};

static ROOT_LONG_ABOUT: LazyLock<String> =
    LazyLock::new(|| render_help_sections(&htmlcut_core::cli_root_help_document().sections));
static ROOT_AFTER_HELP: LazyLock<String> =
    LazyLock::new(|| render_help_examples(&htmlcut_core::cli_root_help_document()));

static CATALOG_LONG_ABOUT: LazyLock<String> = LazyLock::new(|| {
    render_help_sections(
        &htmlcut_core::cli_aux_command_help_document(CliAuxCommandId::Catalog).sections,
    )
});
static CATALOG_AFTER_HELP: LazyLock<String> = LazyLock::new(|| {
    render_help_examples(&htmlcut_core::cli_aux_command_help_document(
        CliAuxCommandId::Catalog,
    ))
});

static SCHEMA_LONG_ABOUT: LazyLock<String> = LazyLock::new(|| {
    render_help_sections(
        &htmlcut_core::cli_aux_command_help_document(CliAuxCommandId::Schema).sections,
    )
});
static SCHEMA_AFTER_HELP: LazyLock<String> = LazyLock::new(|| {
    render_help_examples(&htmlcut_core::cli_aux_command_help_document(
        CliAuxCommandId::Schema,
    ))
});

static INSPECT_LONG_ABOUT: LazyLock<String> = LazyLock::new(|| {
    render_help_sections(
        &htmlcut_core::cli_aux_command_help_document(CliAuxCommandId::Inspect).sections,
    )
});

static SELECT_LONG_ABOUT: LazyLock<String> =
    LazyLock::new(|| build_operation_long_about(htmlcut_core::OperationId::SelectExtract));
static SLICE_LONG_ABOUT: LazyLock<String> =
    LazyLock::new(|| build_operation_long_about(htmlcut_core::OperationId::SliceExtract));
static INSPECT_SOURCE_LONG_ABOUT: LazyLock<String> =
    LazyLock::new(|| build_operation_long_about(htmlcut_core::OperationId::SourceInspect));
static INSPECT_SELECT_LONG_ABOUT: LazyLock<String> =
    LazyLock::new(|| build_operation_long_about(htmlcut_core::OperationId::SelectPreview));
static INSPECT_SLICE_LONG_ABOUT: LazyLock<String> =
    LazyLock::new(|| build_operation_long_about(htmlcut_core::OperationId::SlicePreview));

static SELECT_AFTER_HELP: LazyLock<String> =
    LazyLock::new(|| operation_examples_after_help(htmlcut_core::OperationId::SelectExtract));
static SLICE_AFTER_HELP: LazyLock<String> =
    LazyLock::new(|| operation_examples_after_help(htmlcut_core::OperationId::SliceExtract));
static INSPECT_SOURCE_AFTER_HELP: LazyLock<String> =
    LazyLock::new(|| operation_examples_after_help(htmlcut_core::OperationId::SourceInspect));
static INSPECT_SELECT_AFTER_HELP: LazyLock<String> =
    LazyLock::new(|| operation_examples_after_help(htmlcut_core::OperationId::SelectPreview));
static INSPECT_SLICE_AFTER_HELP: LazyLock<String> =
    LazyLock::new(|| operation_examples_after_help(htmlcut_core::OperationId::SlicePreview));

pub(crate) fn catalog_about() -> &'static str {
    htmlcut_core::cli_aux_command_descriptor(CliAuxCommandId::Catalog).about
}

pub(crate) fn schema_about() -> &'static str {
    htmlcut_core::cli_aux_command_descriptor(CliAuxCommandId::Schema).about
}

pub(crate) fn inspect_about() -> &'static str {
    htmlcut_core::cli_aux_command_descriptor(CliAuxCommandId::Inspect).about
}

pub(crate) fn select_about() -> &'static str {
    htmlcut_core::operation_descriptor(htmlcut_core::OperationId::SelectExtract).description
}

pub(crate) fn slice_about() -> &'static str {
    htmlcut_core::operation_descriptor(htmlcut_core::OperationId::SliceExtract).description
}

pub(crate) fn inspect_source_about() -> &'static str {
    htmlcut_core::operation_descriptor(htmlcut_core::OperationId::SourceInspect).description
}

pub(crate) fn inspect_select_about() -> &'static str {
    htmlcut_core::operation_descriptor(htmlcut_core::OperationId::SelectPreview).description
}

pub(crate) fn inspect_slice_about() -> &'static str {
    htmlcut_core::operation_descriptor(htmlcut_core::OperationId::SlicePreview).description
}

pub(crate) fn root_long_about() -> &'static str {
    ROOT_LONG_ABOUT.as_str()
}

pub(crate) fn root_after_help() -> &'static str {
    ROOT_AFTER_HELP.as_str()
}

pub(crate) fn catalog_long_about() -> &'static str {
    CATALOG_LONG_ABOUT.as_str()
}

pub(crate) fn catalog_after_help() -> &'static str {
    CATALOG_AFTER_HELP.as_str()
}

pub(crate) fn schema_long_about() -> &'static str {
    SCHEMA_LONG_ABOUT.as_str()
}

pub(crate) fn schema_after_help() -> &'static str {
    SCHEMA_AFTER_HELP.as_str()
}

pub(crate) fn inspect_long_about() -> &'static str {
    INSPECT_LONG_ABOUT.as_str()
}

pub(crate) fn select_long_about() -> &'static str {
    SELECT_LONG_ABOUT.as_str()
}

pub(crate) fn slice_long_about() -> &'static str {
    SLICE_LONG_ABOUT.as_str()
}

pub(crate) fn inspect_source_long_about() -> &'static str {
    INSPECT_SOURCE_LONG_ABOUT.as_str()
}

pub(crate) fn inspect_select_long_about() -> &'static str {
    INSPECT_SELECT_LONG_ABOUT.as_str()
}

pub(crate) fn inspect_slice_long_about() -> &'static str {
    INSPECT_SLICE_LONG_ABOUT.as_str()
}

pub(crate) fn select_after_help() -> &'static str {
    SELECT_AFTER_HELP.as_str()
}

pub(crate) fn slice_after_help() -> &'static str {
    SLICE_AFTER_HELP.as_str()
}

pub(crate) fn inspect_source_after_help() -> &'static str {
    INSPECT_SOURCE_AFTER_HELP.as_str()
}

pub(crate) fn inspect_select_after_help() -> &'static str {
    INSPECT_SELECT_AFTER_HELP.as_str()
}

pub(crate) fn inspect_slice_after_help() -> &'static str {
    INSPECT_SLICE_AFTER_HELP.as_str()
}

fn build_operation_long_about(operation_id: htmlcut_core::OperationId) -> String {
    let contract = operation_contract(operation_id);
    let sections = htmlcut_core::cli_operation_help_document(operation_id)
        .expect("CLI-visible operation")
        .sections;

    build_operation_long_about_from_parts(sections, contract)
}

fn build_operation_long_about_from_parts(
    mut sections: Vec<CliHelpSection>,
    contract: &htmlcut_core::OperationCliContract,
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

fn operation_examples_after_help(operation_id: htmlcut_core::OperationId) -> String {
    let document =
        htmlcut_core::cli_operation_help_document(operation_id).expect("CLI-visible operation");
    render_help_examples(&document)
}

fn render_help_examples(document: &CliHelpDocument) -> String {
    format!("Examples:\n  {}", document.examples.join("\n  "))
}

fn render_help_sections(sections: &[CliHelpSection]) -> String {
    sections
        .iter()
        .filter(|section| !section.lines.is_empty())
        .map(render_help_section)
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn render_help_section(section: &CliHelpSection) -> String {
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

fn render_contract_mode_summary(contract: &htmlcut_core::OperationCliContract) -> String {
    let mut lines = Vec::new();

    if let Some(default_match) = contract.default_match {
        lines.push(format!(
            "Default match mode: {}.",
            htmlcut_core::render_cli_value(htmlcut_core::CliValue::SelectionMode(default_match))
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
                    .map(htmlcut_core::CliValue::SelectionMode)
            )
        ));
    }
    if let Some(default_value) = contract.default_value {
        lines.push(format!(
            "Default value mode: {}.",
            htmlcut_core::render_cli_value(htmlcut_core::CliValue::ValueType(default_value))
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
                    .map(htmlcut_core::CliValue::ValueType)
            )
        ));
    }
    if let Some(default_output) = contract.default_output {
        lines.push(format!(
            "Default output mode: {}.",
            htmlcut_core::render_cli_value(htmlcut_core::CliValue::OutputMode(default_output))
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
            htmlcut_core::render_cli_value(output_override.value),
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
                    .map(htmlcut_core::CliValue::OutputMode)
            )
        ));
    }

    lines.join("\n")
}

fn join_cli_values(values: impl IntoIterator<Item = htmlcut_core::CliValue>) -> String {
    values
        .into_iter()
        .map(htmlcut_core::render_cli_value)
        .collect::<Vec<_>>()
        .join(", ")
}

fn operation_contract(
    operation_id: htmlcut_core::OperationId,
) -> &'static htmlcut_core::OperationCliContract {
    htmlcut_core::cli_operation_contract(operation_id).expect("CLI-visible operation")
}

#[cfg(test)]
pub(crate) fn render_help_section_for_tests(section: &CliHelpSection) -> String {
    render_help_section(section)
}

#[cfg(test)]
pub(crate) fn render_contract_mode_summary_for_tests(
    contract: &htmlcut_core::OperationCliContract,
) -> String {
    render_contract_mode_summary(contract)
}

#[cfg(test)]
pub(crate) fn build_operation_long_about_from_parts_for_tests(
    sections: Vec<CliHelpSection>,
    contract: &htmlcut_core::OperationCliContract,
) -> String {
    build_operation_long_about_from_parts(sections, contract)
}
