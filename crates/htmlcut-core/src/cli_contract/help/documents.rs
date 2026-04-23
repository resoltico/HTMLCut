use crate::catalog::{OperationId, operation_descriptor};
use crate::{CORE_RESULT_SCHEMA_NAME, cli_operation_contract, interop::v1::RESULT_SCHEMA_NAME};

use super::model::{
    CliAuxCommandId, CliHelpDocument, CliHelpSection, CliHelpSectionStyle, cli_aux_command_catalog,
    cli_aux_command_display_command,
};

/// Builds the canonical root help document for the HTMLCut CLI.
pub fn cli_root_help_document() -> CliHelpDocument {
    let mut command_summaries = cli_aux_command_catalog()
        .iter()
        .map(|descriptor| {
            (
                descriptor.command_path.join(" "),
                descriptor.about.to_owned(),
            )
        })
        .collect::<Vec<_>>();
    command_summaries.extend(
        [OperationId::SelectExtract, OperationId::SliceExtract]
            .into_iter()
            .filter_map(|operation_id| {
                cli_operation_contract(operation_id).map(|contract| {
                    (
                        contract.display_command(),
                        operation_descriptor(operation_id).description.to_owned(),
                    )
                })
            }),
    );

    let discovery_lines = vec![
        format!(
            "{} lists stable operation IDs plus the CLI/core surfaces, request/result contract refs, typed defaults, command constraints, modes, parameters, notes, and examples for each operation.",
            cli_aux_command_display_command(CliAuxCommandId::Catalog)
        ),
        format!(
            "{} exports the validator-grade JSON Schema documents behind those contract refs.",
            cli_aux_command_display_command(CliAuxCommandId::Schema)
        ),
        format!(
            "{} learns document shape, headings, links, classes, and effective base URL.",
            cli_operation_display_command(OperationId::SourceInspect)
        ),
        format!(
            "{} or {} previews matches in structured JSON before extraction.",
            cli_operation_display_command(OperationId::SelectPreview),
            cli_operation_display_command(OperationId::SlicePreview)
        ),
        format!(
            "{} or {} emits the final payload once you trust the request.",
            cli_operation_display_command(OperationId::SelectExtract),
            cli_operation_display_command(OperationId::SliceExtract)
        ),
        "--emit-request-file saves the normalized extraction definition you can reuse with --request-file.".to_owned(),
    ];

    CliHelpDocument {
        sections: vec![
            CliHelpSection {
                title: format!(
                    "HTMLCut has {} operator-facing entry points",
                    command_summaries.len()
                ),
                style: CliHelpSectionStyle::Plain,
                lines: command_summaries
                    .into_iter()
                    .map(|(command, about)| format!("  {command:<8} {about}"))
                    .collect(),
            },
            CliHelpSection {
                title: "Discovery flow".to_owned(),
                style: CliHelpSectionStyle::Numbered,
                lines: discovery_lines,
            },
            CliHelpSection {
                title: "Inputs".to_owned(),
                style: CliHelpSectionStyle::Bullets,
                lines: vec![
                    "<INPUT> may be a local file path, an http:// or https:// URL, or - for stdin."
                        .to_owned(),
                    "Bundle directories are created automatically when you use --bundle."
                        .to_owned(),
                ],
            },
            CliHelpSection {
                title: "Output model".to_owned(),
                style: CliHelpSectionStyle::Bullets,
                lines: vec![
                    "select and slice separate the value you extract from how stdout is rendered."
                        .to_owned(),
                    "--value chooses what each match produces before stdout formatting.".to_owned(),
                    "--output chooses how stdout is emitted.".to_owned(),
                    "inspect defaults to JSON so agents can reason about the source and preview report."
                        .to_owned(),
                    "Use --output text for a compact human summary.".to_owned(),
                ],
            },
            CliHelpSection {
                title: "URL resolution".to_owned(),
                style: CliHelpSectionStyle::Bullets,
                lines: vec![
                    "--rewrite-urls resolves relative links with the effective base URL."
                        .to_owned(),
                    "The effective base comes from --base-url when supplied, the input URL for URL sources, and any document <base href> when one is present."
                        .to_owned(),
                    "When no effective base can be resolved, HTMLCut leaves relative URLs unchanged and reports a warning."
                        .to_owned(),
                ],
            },
            CliHelpSection {
                title: "Failure model".to_owned(),
                style: CliHelpSectionStyle::Bullets,
                lines: vec![
                    "Human output modes print the primary failure to stderr.".to_owned(),
                    "JSON output modes emit structured JSON to stdout and still exit non-zero."
                        .to_owned(),
                ],
            },
        ],
        examples: [
            Some(format!(
                "htmlcut {} --output json",
                cli_aux_command_display_command(CliAuxCommandId::Catalog)
            )),
            Some(format!(
                "htmlcut {} --output json",
                cli_aux_command_display_command(CliAuxCommandId::Schema)
            )),
            first_operation_example(OperationId::SelectExtract).map(ToOwned::to_owned),
            operation_example_containing(OperationId::SelectExtract, "--rewrite-urls")
                .map(ToOwned::to_owned),
            first_operation_example(OperationId::SliceExtract).map(ToOwned::to_owned),
            first_operation_example(OperationId::SourceInspect).map(ToOwned::to_owned),
            first_operation_example(OperationId::SelectPreview).map(ToOwned::to_owned),
        ]
        .into_iter()
        .flatten()
        .collect(),
    }
}

/// Builds the canonical help document for one non-operation CLI command.
pub fn cli_aux_command_help_document(id: CliAuxCommandId) -> CliHelpDocument {
    match id {
        CliAuxCommandId::Catalog => CliHelpDocument {
            sections: vec![CliHelpSection {
                title: "Overview".to_owned(),
                style: CliHelpSectionStyle::Bullets,
                lines: vec![
                    "Print HTMLCut's capability catalog.".to_owned(),
                    "Use this command to discover stable operation IDs, the command and core surfaces that expose each operation, the public request/result contracts tied to that operation, and the machine-readable CLI command contract when one exists, including parameter inventory, typed defaults, command constraints, and schema references.".to_owned(),
                    "Use --output json when an agent or script wants machine-readable capability introspection.".to_owned(),
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
                    ],
                },
                CliHelpSection {
                    title: "Registry includes".to_owned(),
                    style: CliHelpSectionStyle::Bullets,
                    lines: vec![
                        "htmlcut-core request/result schemas".to_owned(),
                        "htmlcut-cli report schemas".to_owned(),
                        "the frozen interop schemas shipped by htmlcut_core::interop::v1"
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
                    "htmlcut {} --name {} --schema-version 1 --output json",
                    cli_aux_command_display_command(CliAuxCommandId::Schema),
                    RESULT_SCHEMA_NAME
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
                    cli_operation_contract(operation_id).map(|contract| {
                        format!(
                            "{}    {}",
                            contract.display_command(),
                            operation_descriptor(operation_id).description
                        )
                    })
                })
                .collect(),
            }],
            examples: Vec::new(),
        },
    }
}

/// Builds the canonical help document for one CLI-visible operation.
pub fn cli_operation_help_document(operation_id: OperationId) -> Option<CliHelpDocument> {
    match operation_id {
        OperationId::DocumentParse => None,
        OperationId::SourceInspect => cli_operation_help_document_with_overview(
            operation_id,
            vec![
                operation_descriptor(operation_id).description.to_owned(),
                "This command summarizes title, counts, headings, link previews, top tags, top classes, document base behavior, and optional source text. It is designed to help you choose selectors or confirm how URL rewriting will behave before extracting data."
                    .to_owned(),
            ],
        ),
        OperationId::SelectExtract => cli_operation_help_document_with_overview(
            operation_id,
            vec![
                operation_descriptor(operation_id).description.to_owned(),
                "Use inspect source first when you need to learn the document shape, then inspect select to preview matches before emitting the final payload."
                    .to_owned(),
            ],
        ),
        OperationId::SliceExtract => cli_operation_help_document_with_overview(
            operation_id,
            vec![
                operation_descriptor(operation_id).description.to_owned(),
                "Use --pattern literal for plain substring boundaries or --pattern regex for regex boundaries. Boundary matches are consumed exactly as matched."
                    .to_owned(),
            ],
        ),
        OperationId::SelectPreview => cli_operation_help_document_with_overview(
            operation_id,
            vec![
                operation_descriptor(operation_id).description.to_owned(),
                "Use this preview workflow to inspect structured per-match metadata before final extraction."
                    .to_owned(),
            ],
        ),
        OperationId::SlicePreview => cli_operation_help_document_with_overview(
            operation_id,
            vec![
                operation_descriptor(operation_id).description.to_owned(),
                "Use this preview workflow to inspect literal or regex slice ranges before final extraction."
                    .to_owned(),
            ],
        ),
    }
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
        .cli_surface
        .unwrap_or(operation_id.as_str())
        .to_owned()
}
