use htmlcut_core::{
    CORE_RESULT_SCHEMA_NAME, OperationId,
    interop::v1::{RESULT_SCHEMA_NAME, RESULT_SCHEMA_VERSION},
    operation_descriptor,
};

use super::super::cli_operation_contract;
use super::super::operation_specs::operation_surface_spec;
use super::ensure_cli_help_catalog_validated;
use super::model::{
    CliAuxCommandId, CliHelpDocument, CliHelpSection, CliHelpSectionStyle,
    cli_aux_command_display_command,
};

/// Builds the canonical root help document for the HTMLCut CLI.
pub fn cli_root_help_document() -> CliHelpDocument {
    ensure_cli_help_catalog_validated();
    build_cli_root_help_document()
}

pub(super) fn build_cli_root_help_document() -> CliHelpDocument {
    let start_here_lines = vec![
        format!(
            "{} learns document shape, headings, links, classes, and effective base URL.",
            cli_operation_display_command(OperationId::SourceInspect)
        ),
        format!(
            "{} or {} previews the same extraction value modes plus match metadata before final output.",
            cli_operation_display_command(OperationId::SelectPreview),
            cli_operation_display_command(OperationId::SlicePreview)
        ),
        format!(
            "{} or {} emits the final payload once you trust the request.",
            cli_operation_display_command(OperationId::SelectExtract),
            cli_operation_display_command(OperationId::SliceExtract)
        ),
        format!(
            "{} and {} expose the stable contracts behind those workflows.",
            cli_aux_command_display_command(CliAuxCommandId::Catalog),
            cli_aux_command_display_command(CliAuxCommandId::Schema)
        ),
    ];

    CliHelpDocument {
        sections: vec![
            CliHelpSection {
                title: "Workflow".to_owned(),
                style: CliHelpSectionStyle::Numbered,
                lines: start_here_lines,
            },
            CliHelpSection {
                title: "Request files".to_owned(),
                style: CliHelpSectionStyle::Bullets,
                lines: vec![
                    "--emit-request-file writes the normalized extraction definition for the current run."
                        .to_owned(),
                    "--request-file reruns a saved definition instead of spelling the source and strategy inline."
                        .to_owned(),
                    "--overwrite is required before HTMLCut will replace an existing request file, output file, or bundle path."
                        .to_owned(),
                ],
            },
            CliHelpSection {
                title: "Inputs".to_owned(),
                style: CliHelpSectionStyle::Bullets,
                lines: vec![
                    "<INPUT> may be a local file path, an http:// or https:// URL, or - for stdin."
                        .to_owned(),
                    "select, slice, inspect select, and inspect slice can load a saved definition with --request-file."
                        .to_owned(),
                    "Parent directories are created automatically for --emit-request-file, --output-file, and --bundle."
                        .to_owned(),
                ],
            },
            CliHelpSection {
                title: "Output".to_owned(),
                style: CliHelpSectionStyle::Bullets,
                lines: vec![
                    "--value chooses what each extracted match should produce before stdout formatting."
                        .to_owned(),
                    "--output chooses how stdout is emitted.".to_owned(),
                    "When the extracted value is HTML, --output text renders that fragment into readable plain text with heading, list, table, and link structure preserved."
                        .to_owned(),
                    "HTML output preserves the selected fragment apart from optional URL rewriting, so compare alternate selectors when you want a cleaner saved fragment."
                        .to_owned(),
                    "--output none suppresses stdout and therefore requires --bundle."
                        .to_owned(),
                    "inspect defaults to JSON so agents and scripts can reason about the source or preview report."
                        .to_owned(),
                    "Use --output text for a compact human summary.".to_owned(),
                ],
            },
            CliHelpSection {
                title: "URL resolution".to_owned(),
                style: CliHelpSectionStyle::Bullets,
                lines: vec![
                    "--rewrite-urls rewrites supported relative URLs with the effective base URL, including standard HTML URL-bearing attributes plus CSS url(...) and quoted @import references."
                        .to_owned(),
                    "When rendered plain text includes link destinations and an effective base is known, HTMLCut resolves those displayed destinations to absolute URLs even if --rewrite-urls is off."
                        .to_owned(),
                    "The effective base comes from --base-url when supplied, the input URL for URL sources, and any document <base href> when one is present."
                        .to_owned(),
                    "When no effective base can be resolved, HTMLCut leaves HTML fragments unchanged, plain-text link destinations remain relative, and --rewrite-urls reports a warning."
                        .to_owned(),
                ],
            },
            CliHelpSection {
                title: "Errors".to_owned(),
                style: CliHelpSectionStyle::Bullets,
                lines: vec![
                    "Human output modes print the primary failure to stderr.".to_owned(),
                    "JSON output modes emit structured JSON to stdout and still exit non-zero."
                        .to_owned(),
                ],
            },
        ],
        examples: [
            first_operation_example(OperationId::SourceInspect).map(ToOwned::to_owned),
            first_operation_example(OperationId::SelectPreview).map(ToOwned::to_owned),
            operation_example_containing(OperationId::SelectExtract, "--emit-request-file")
                .map(ToOwned::to_owned),
            operation_example_containing(OperationId::SelectExtract, "--request-file")
                .map(ToOwned::to_owned),
            first_operation_example(OperationId::SliceExtract).map(ToOwned::to_owned),
            Some(format!(
                "htmlcut {} --output json",
                cli_aux_command_display_command(CliAuxCommandId::Catalog)
            )),
            Some(format!(
                "htmlcut {} --output json",
                cli_aux_command_display_command(CliAuxCommandId::Schema)
            )),
        ]
        .into_iter()
        .flatten()
        .collect(),
    }
}

/// Builds the canonical help document for one non-operation CLI command.
pub fn cli_aux_command_help_document(id: CliAuxCommandId) -> CliHelpDocument {
    ensure_cli_help_catalog_validated();
    build_cli_aux_command_help_document(id)
}

pub(super) fn build_cli_aux_command_help_document(id: CliAuxCommandId) -> CliHelpDocument {
    match id {
        CliAuxCommandId::Catalog => CliHelpDocument {
            sections: vec![CliHelpSection {
                title: "Overview".to_owned(),
                style: CliHelpSectionStyle::Bullets,
                lines: vec![
                    "Print HTMLCut's capability catalog.".to_owned(),
                    "Use this command to discover stable operation IDs, the command and core surfaces that expose each operation, the public request/result contracts tied to that operation, and the machine-readable CLI command contract when one exists, including parameter inventory, typed defaults, command constraints, and schema references.".to_owned(),
                    "Use --output json when an agent or script wants machine-readable capability introspection.".to_owned(),
                    "When --output-file points at an existing path, pass --overwrite or choose a fresh file instead."
                        .to_owned(),
                ],
            }],
            examples: vec![
                format!(
                    "htmlcut {}",
                    cli_aux_command_display_command(CliAuxCommandId::Catalog)
                ),
                format!(
                    "htmlcut {} --output json",
                    cli_aux_command_display_command(CliAuxCommandId::Catalog)
                ),
                format!(
                    "htmlcut {} --operation {}",
                    cli_aux_command_display_command(CliAuxCommandId::Catalog),
                    OperationId::SourceInspect.as_str()
                ),
                format!(
                    "htmlcut {} --operation {} --output json",
                    cli_aux_command_display_command(CliAuxCommandId::Catalog),
                    OperationId::SliceExtract.as_str()
                ),
            ],
        },
        CliAuxCommandId::Schema => CliHelpDocument {
            sections: vec![
                CliHelpSection {
                    title: "Overview".to_owned(),
                    style: CliHelpSectionStyle::Bullets,
                    lines: vec![
                        "Export HTMLCut's validator-grade JSON schema registry.".to_owned(),
                        "Use this command when a downstream tool needs the actual JSON Schema documents for HTMLCut's public JSON contracts instead of descriptive capability text."
                            .to_owned(),
                        "When --output-file points at an existing path, pass --overwrite or choose a fresh file instead."
                            .to_owned(),
                    ],
                },
                CliHelpSection {
                    title: "Registry includes".to_owned(),
                    style: CliHelpSectionStyle::Bullets,
                    lines: vec![
                        "htmlcut-core request/result schemas".to_owned(),
                        "htmlcut-cli report schemas".to_owned(),
                        "the versioned interop schemas shipped by htmlcut_core::interop::v1"
                            .to_owned(),
                    ],
                },
                CliHelpSection {
                    title: "Filtering".to_owned(),
                    style: CliHelpSectionStyle::Bullets,
                    lines: vec![
                        "Use --name to select one schema family.".to_owned(),
                        "Use --schema-version to pin one exact version.".to_owned(),
                    ],
                },
            ],
            examples: vec![
                format!(
                    "htmlcut {}",
                    cli_aux_command_display_command(CliAuxCommandId::Schema)
                ),
                format!(
                    "htmlcut {} --output json",
                    cli_aux_command_display_command(CliAuxCommandId::Schema)
                ),
                format!(
                    "htmlcut {} --name {} --output json",
                    cli_aux_command_display_command(CliAuxCommandId::Schema),
                    CORE_RESULT_SCHEMA_NAME
                ),
                format!(
                    "htmlcut {} --name {} --schema-version {} --output json",
                    cli_aux_command_display_command(CliAuxCommandId::Schema),
                    RESULT_SCHEMA_NAME,
                    RESULT_SCHEMA_VERSION
                ),
            ],
        },
        CliAuxCommandId::Inspect => CliHelpDocument {
            sections: vec![CliHelpSection {
                title: "Commands".to_owned(),
                style: CliHelpSectionStyle::Plain,
                lines: [
                    OperationId::SourceInspect,
                    OperationId::SelectPreview,
                    OperationId::SlicePreview,
                ]
                .into_iter()
                .filter_map(|operation_id| {
                    let contract = cli_operation_contract(operation_id)?;
                    let descriptor = operation_descriptor(operation_id)?;
                    Some(format!(
                        "{}    {}",
                        contract.display_command(),
                        descriptor.description
                    ))
                })
                .collect(),
            }],
            examples: Vec::new(),
        },
    }
}

/// Builds the canonical help document for one CLI-visible operation.
pub fn cli_operation_help_document(operation_id: OperationId) -> Option<CliHelpDocument> {
    ensure_cli_help_catalog_validated();
    build_cli_operation_help_document(operation_id)
}

pub(super) fn build_cli_operation_help_document(
    operation_id: OperationId,
) -> Option<CliHelpDocument> {
    let surface = operation_surface_spec(operation_id)?;
    let cli_spec = surface.cli.as_ref()?;
    let mut overview_lines = vec![operation_descriptor(operation_id)?.description.to_owned()];
    overview_lines.extend(cli_spec.help_overview.iter().map(|line| (*line).to_owned()));
    cli_operation_help_document_with_overview(operation_id, overview_lines)
}

fn cli_operation_help_document_with_overview(
    operation_id: OperationId,
    overview_lines: Vec<String>,
) -> Option<CliHelpDocument> {
    let cli_contract = cli_operation_contract(operation_id)?;

    Some(CliHelpDocument {
        sections: vec![CliHelpSection {
            title: "Overview".to_owned(),
            style: CliHelpSectionStyle::Bullets,
            lines: overview_lines,
        }],
        examples: cli_contract
            .examples
            .iter()
            .map(|example| (*example).to_owned())
            .collect(),
    })
}

fn first_operation_example(operation_id: OperationId) -> Option<&'static str> {
    cli_operation_contract(operation_id)
        .and_then(|contract| contract.examples.first())
        .copied()
}

fn operation_example_containing(operation_id: OperationId, needle: &str) -> Option<&'static str> {
    cli_operation_contract(operation_id)
        .and_then(|contract| {
            contract
                .examples
                .iter()
                .copied()
                .find(|example| example.contains(needle))
        })
        .or_else(|| first_operation_example(operation_id))
}

fn cli_operation_display_command(operation_id: OperationId) -> String {
    operation_descriptor(operation_id)
        .and_then(|descriptor| descriptor.cli_surface)
        .unwrap_or(operation_id.as_str())
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::CliAuxCommandId;

    #[test]
    fn inspect_aux_help_document_lists_the_live_inspection_commands() {
        let document = cli_aux_command_help_document(CliAuxCommandId::Inspect);
        assert!(document.examples.is_empty());
        assert_eq!(document.sections.len(), 1);
        assert_eq!(document.sections[0].title, "Commands");
        assert!(
            document.sections[0]
                .lines
                .iter()
                .any(|line| line.starts_with("inspect source"))
        );
        assert!(
            document.sections[0]
                .lines
                .iter()
                .any(|line| line.starts_with("inspect select"))
        );
        assert!(
            document.sections[0]
                .lines
                .iter()
                .any(|line| line.starts_with("inspect slice"))
        );
    }
}
