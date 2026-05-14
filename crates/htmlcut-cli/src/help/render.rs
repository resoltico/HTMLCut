use crate::contract::CliHelpDocument;
#[cfg(test)]
use crate::contract::{CliHelpSection, CliHelpSectionStyle, OperationCliContract};
use crate::error::CliError;
use crate::lookup;

#[cfg(test)]
fn build_operation_long_about_from_sources(
    contract: Result<&'static OperationCliContract, CliError>,
    document: Result<CliHelpDocument, CliError>,
) -> Result<String, CliError> {
    let contract = contract?;
    let sections = document?.sections;
    Ok(build_operation_long_about_from_parts(sections, contract))
}

#[cfg(test)]
pub(super) fn build_operation_long_about_from_parts(
    mut sections: Vec<CliHelpSection>,
    contract: &OperationCliContract,
) -> String {
    let mode_summary = render_contract_mode_summary(contract);
    if !mode_summary.is_empty() {
        sections.push(CliHelpSection {
            title: "Behavior".to_owned(),
            style: CliHelpSectionStyle::Plain,
            lines: mode_summary.lines().map(str::to_owned).collect(),
        });
    }
    if !contract.notes.is_empty() {
        sections.push(CliHelpSection {
            title: "Key Rules".to_owned(),
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
    Ok(render_examples_after_help(&document?))
}

pub(super) fn render_examples_after_help(document: &CliHelpDocument) -> String {
    if document.examples.is_empty() {
        String::new()
    } else {
        format!("Examples:\n  {}", document.examples.join("\n  "))
    }
}

#[cfg(test)]
pub(super) fn render_help_sections(sections: &[CliHelpSection]) -> String {
    sections
        .iter()
        .filter(|section| !section.lines.is_empty())
        .map(render_help_section)
        .collect::<Vec<_>>()
        .join("\n\n")
}

#[cfg(test)]
pub(super) fn render_guidance_sections(sections: &[CliHelpSection]) -> String {
    let rendered_sections = sections
        .iter()
        .filter(|section| !section.lines.is_empty())
        .map(render_guidance_section)
        .collect::<Vec<_>>()
        .join("\n\n");

    if rendered_sections.is_empty() {
        String::new()
    } else {
        format!("Guidance:\n\n{rendered_sections}")
    }
}

#[cfg(test)]
pub(super) fn render_help_section(section: &CliHelpSection) -> String {
    let body = render_section_body(section, "");

    format!("{}:\n{}", section.title, body)
}

#[cfg(test)]
fn render_guidance_section(section: &CliHelpSection) -> String {
    let body = render_section_body(section, "    ");
    format!("  {}:\n{}", section.title, body)
}

#[cfg(test)]
fn render_section_body(section: &CliHelpSection, indent: &str) -> String {
    match section.style {
        CliHelpSectionStyle::Plain => section
            .lines
            .iter()
            .map(|line| format!("{indent}{line}"))
            .collect::<Vec<_>>()
            .join("\n"),
        CliHelpSectionStyle::Bullets => section
            .lines
            .iter()
            .map(|line| format!("{indent}- {line}"))
            .collect::<Vec<_>>()
            .join("\n"),
        CliHelpSectionStyle::Numbered => section
            .lines
            .iter()
            .enumerate()
            .map(|(index, line)| format!("{indent}{}. {line}", index + 1))
            .collect::<Vec<_>>()
            .join("\n"),
    }
}

#[cfg(test)]
pub(super) fn render_contract_mode_summary(contract: &OperationCliContract) -> String {
    let mut lines = Vec::new();

    if let Some(default_match) = contract.default_match {
        lines.push(format!(
            "Default match mode: {}.",
            crate::contract::render_cli_value(crate::contract::CliValue::SelectionMode(
                default_match
            ))
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
                    .map(crate::contract::CliValue::SelectionMode)
            )
        ));
    }
    if let Some(default_value) = contract.default_value {
        lines.push(format!(
            "Default value mode: {}.",
            crate::contract::render_cli_value(crate::contract::CliValue::ValueType(default_value))
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
                    .map(crate::contract::CliValue::ValueType)
            )
        ));
    }
    if let Some(default_output) = contract.default_output {
        lines.push(format!(
            "Default output mode: {}.",
            crate::contract::render_cli_value(crate::contract::CliValue::OutputMode(
                default_output
            ))
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
            crate::contract::render_cli_value(output_override.value),
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
                    .map(crate::contract::CliValue::OutputMode)
            )
        ));
    }

    lines.join("\n")
}

#[cfg(test)]
fn join_cli_values(values: impl IntoIterator<Item = crate::contract::CliValue>) -> String {
    values
        .into_iter()
        .map(crate::contract::render_cli_value)
        .collect::<Vec<_>>()
        .join(", ")
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::{
        CliCondition, CliConditionalDefault, CliInputForm, CliOutputMode, CliParameterId, CliValue,
        OperationCliContract, cli_operation_contract,
    };
    use crate::error::usage_error;
    use crate::model::CliErrorCode;
    use htmlcut_core::{OperationId, ValueType};

    #[test]
    fn render_examples_after_help_handles_empty_sections_and_examples() {
        let empty = CliHelpDocument {
            sections: Vec::new(),
            examples: Vec::new(),
        };
        assert!(render_examples_after_help(&empty).is_empty());
        assert!(render_guidance_sections(&[]).is_empty());

        let examples_only = CliHelpDocument {
            sections: Vec::new(),
            examples: vec!["htmlcut select README.md --css article".to_owned()],
        };
        assert_eq!(
            render_examples_after_help(&examples_only),
            "Examples:\n  htmlcut select README.md --css article"
        );

        let guide_only = CliHelpDocument {
            sections: vec![CliHelpSection {
                title: "Start Here".to_owned(),
                style: CliHelpSectionStyle::Plain,
                lines: vec!["Inspect first.".to_owned()],
            }],
            examples: Vec::new(),
        };
        assert_eq!(
            render_guidance_sections(&guide_only.sections),
            "Guidance:\n\n  Start Here:\n    Inspect first."
        );
    }

    #[test]
    fn long_about_helpers_render_behavior_rules_and_section_styles() {
        let contract = OperationCliContract {
            operation_id: OperationId::SelectExtract,
            command_path: &["select"],
            invocation: "htmlcut select [OPTIONS] [INPUT]",
            inputs: vec![CliInputForm::LocalFilePath],
            default_match: Some(crate::contract::CliSelectionMode::First),
            selection_modes: vec![
                crate::contract::CliSelectionMode::First,
                crate::contract::CliSelectionMode::All,
            ],
            default_value: Some(ValueType::Text),
            value_modes: vec![ValueType::Text, ValueType::OuterHtml],
            default_output: Some(CliOutputMode::Text),
            default_output_overrides: vec![
                CliConditionalDefault {
                    value: CliValue::OutputMode(CliOutputMode::Html),
                    when: CliCondition {
                        parameter: CliParameterId::Value,
                        values: vec![CliValue::ValueType(ValueType::OuterHtml)],
                    },
                },
                CliConditionalDefault {
                    value: CliValue::OutputMode(CliOutputMode::Json),
                    when: CliCondition {
                        parameter: CliParameterId::Value,
                        values: vec![
                            CliValue::ValueType(ValueType::Text),
                            CliValue::ValueType(ValueType::Structured),
                        ],
                    },
                },
            ],
            output_modes: vec![
                CliOutputMode::Text,
                CliOutputMode::Json,
                CliOutputMode::Html,
            ],
            constraints: Vec::new(),
            notes: vec![
                "Pick one stable selector.",
                "Bundle output writes report files.",
            ],
            examples: vec!["htmlcut select ./page.html --css article"],
            parameters: Vec::new(),
        };
        let document = CliHelpDocument {
            sections: vec![
                CliHelpSection {
                    title: "Modes".to_owned(),
                    style: CliHelpSectionStyle::Numbered,
                    lines: vec![
                        "Inspect the source.".to_owned(),
                        "Extract one match.".to_owned(),
                    ],
                },
                CliHelpSection {
                    title: "Flags".to_owned(),
                    style: CliHelpSectionStyle::Bullets,
                    lines: vec!["Use --rewrite-urls when needed.".to_owned()],
                },
            ],
            examples: vec!["htmlcut select ./page.html --css article".to_owned()],
        };

        let rendered = build_operation_long_about_from_parts(document.sections.clone(), &contract);
        assert!(rendered.contains("Modes:\n1. Inspect the source.\n2. Extract one match."));
        assert!(rendered.contains("Flags:\n- Use --rewrite-urls when needed."));
        assert!(rendered.contains("Behavior:\nDefault match mode: first."));
        assert!(rendered.contains("Key Rules:\n- Pick one stable selector."));
        assert!(rendered.contains("Output default override: html when --value is outer-html."));
        assert!(
            rendered
                .contains("Output default override: json when --value is one of text, structured.")
        );

        let rendered_from_sources = build_operation_long_about_from_sources_for_tests(
            Ok(cli_operation_contract(OperationId::SelectExtract).expect("select contract")),
            Ok(CliHelpDocument {
                sections: vec![CliHelpSection {
                    title: "Modes".to_owned(),
                    style: CliHelpSectionStyle::Numbered,
                    lines: vec!["Inspect the source.".to_owned()],
                }],
                examples: Vec::new(),
            }),
        )
        .expect("rendered long about");
        assert!(rendered_from_sources.contains("Behavior:"));
        assert!(rendered_from_sources.contains("Key Rules:"));

        let missing_contract = build_operation_long_about_from_sources_for_tests(
            None.ok_or_else(|| {
                usage_error(CliErrorCode::ContractMissing, "select contract missing")
            }),
            Ok(CliHelpDocument {
                sections: Vec::new(),
                examples: Vec::new(),
            }),
        )
        .expect_err("missing contract should fail");
        assert_eq!(
            missing_contract.code.as_str(),
            CliErrorCode::ContractMissing.as_str()
        );
        assert_eq!(missing_contract.message, "select contract missing");
    }
}
