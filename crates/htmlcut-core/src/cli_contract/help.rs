#[cfg(test)]
use crate::catalog::{OperationDescriptor, operation_catalog};
use crate::catalog::{OperationId, operation_descriptor};
use crate::{CORE_RESULT_SCHEMA_NAME, cli_operation_contract, interop::v1::RESULT_SCHEMA_NAME};

/// Canonical non-operation CLI commands whose help surface is owned by `htmlcut-core`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CliAuxCommandId {
    /// `htmlcut catalog`
    Catalog,
    /// `htmlcut schema`
    Schema,
    /// `htmlcut inspect`
    Inspect,
}

impl CliAuxCommandId {
    /// Returns the stable display-form command path for this command.
    pub const fn command_path(self) -> &'static [&'static str] {
        match self {
            Self::Catalog => &["catalog"],
            Self::Schema => &["schema"],
            Self::Inspect => &["inspect"],
        }
    }
}

/// Stable summary for one non-operation CLI command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CliAuxCommandDescriptor {
    /// Stable command identifier.
    pub id: CliAuxCommandId,
    /// Command path tokens exactly as the user types them.
    pub command_path: &'static [&'static str],
    /// Concise user-facing command summary.
    pub about: &'static str,
}

/// Structured formatting style for one help section.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CliHelpSectionStyle {
    /// Render lines exactly as-is.
    Plain,
    /// Render each line as a bulleted item.
    Bullets,
    /// Render each line as a numbered step.
    Numbered,
}

/// One structured help section owned by the canonical CLI contract.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CliHelpSection {
    /// Section title.
    pub title: String,
    /// Rendering style for the section lines.
    pub style: CliHelpSectionStyle,
    /// Section body lines.
    pub lines: Vec<String>,
}

/// Structured help document owned by the canonical CLI contract.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CliHelpDocument {
    /// Ordered help sections.
    pub sections: Vec<CliHelpSection>,
    /// Example invocations that belong to the surface.
    pub examples: Vec<String>,
}

const CLI_AUX_COMMAND_CATALOG: &[CliAuxCommandDescriptor] = &[
    CliAuxCommandDescriptor {
        id: CliAuxCommandId::Catalog,
        command_path: &["catalog"],
        about: "Print the capability catalog with stable operation IDs.",
    },
    CliAuxCommandDescriptor {
        id: CliAuxCommandId::Schema,
        command_path: &["schema"],
        about: "Export validator-grade JSON schemas for HTMLCut's public JSON contracts.",
    },
    CliAuxCommandDescriptor {
        id: CliAuxCommandId::Inspect,
        command_path: &["inspect"],
        about: "Explore a source or preview a request before committing to a final extraction.",
    },
];

/// Returns the canonical catalog of non-operation CLI commands.
pub const fn cli_aux_command_catalog() -> &'static [CliAuxCommandDescriptor] {
    CLI_AUX_COMMAND_CATALOG
}

/// Returns the canonical non-operation CLI descriptor for one command.
pub fn cli_aux_command_descriptor(id: CliAuxCommandId) -> &'static CliAuxCommandDescriptor {
    cli_aux_command_catalog()
        .iter()
        .find(|descriptor| descriptor.id == id)
        .expect("every CliAuxCommandId should appear in CLI_AUX_COMMAND_CATALOG")
}

/// Returns the display-form command label for one canonical non-operation CLI command.
pub fn cli_aux_command_display_command(id: CliAuxCommandId) -> String {
    cli_aux_command_descriptor(id).command_path.join(" ")
}

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
        examples: vec![
            format!(
                "htmlcut {} --output json",
                cli_aux_command_display_command(CliAuxCommandId::Catalog)
            ),
            format!(
                "htmlcut {} --output json",
                cli_aux_command_display_command(CliAuxCommandId::Schema)
            ),
            first_operation_example(OperationId::SelectExtract).to_owned(),
            operation_example_containing(OperationId::SelectExtract, "--rewrite-urls").to_owned(),
            first_operation_example(OperationId::SliceExtract).to_owned(),
            first_operation_example(OperationId::SourceInspect).to_owned(),
            first_operation_example(OperationId::SelectPreview).to_owned(),
        ],
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

#[cfg(test)]
pub(crate) fn cli_help_catalog_validation_errors() -> Vec<String> {
    cli_help_catalog_validation_errors_with(
        cli_aux_command_catalog(),
        operation_catalog(),
        |operation_id| cli_operation_help_document(operation_id).is_some(),
        cli_root_help_document().examples.is_empty(),
    )
}

#[cfg(test)]
fn cli_help_catalog_validation_errors_with(
    aux_descriptors: &[CliAuxCommandDescriptor],
    operation_descriptors: &[OperationDescriptor],
    has_help: impl Fn(OperationId) -> bool,
    root_examples_empty: bool,
) -> Vec<String> {
    let mut errors = Vec::new();

    if aux_descriptors.is_empty() {
        errors.push("cli_aux_command_catalog() is empty".to_owned());
    }

    for descriptor in aux_descriptors {
        if descriptor.command_path.is_empty() {
            errors.push(format!("{:?} has an empty command path", descriptor.id));
        }
        if descriptor.about.trim().is_empty() {
            errors.push(format!("{:?} has an empty about string", descriptor.id));
        }
    }

    for descriptor in operation_descriptors {
        match (descriptor.cli_surface.is_some(), has_help(descriptor.id)) {
            (true, false) => errors.push(format!(
                "{} is CLI-visible in OPERATION_CATALOG but missing CLI help documentation",
                descriptor.id
            )),
            (false, true) => errors.push(format!(
                "{} is core-only in OPERATION_CATALOG but has CLI help documentation",
                descriptor.id
            )),
            (true, true) | (false, false) => {}
        }
    }

    if root_examples_empty {
        errors.push("root help examples are empty".to_owned());
    }

    errors
}

fn first_operation_example(operation_id: OperationId) -> &'static str {
    cli_operation_contract(operation_id)
        .expect("CLI-visible operation")
        .examples
        .first()
        .copied()
        .expect("CLI-visible operation should keep examples")
}

fn operation_example_containing(operation_id: OperationId, needle: &str) -> &'static str {
    cli_operation_contract(operation_id)
        .expect("CLI-visible operation")
        .examples
        .iter()
        .copied()
        .find(|example| example.contains(needle))
        .unwrap_or_else(|| first_operation_example(operation_id))
}

fn cli_operation_display_command(operation_id: OperationId) -> String {
    cli_operation_contract(operation_id)
        .expect("CLI-visible operation")
        .display_command()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aux_command_ids_keep_stable_command_paths() {
        assert_eq!(CliAuxCommandId::Catalog.command_path(), &["catalog"]);
        assert_eq!(CliAuxCommandId::Schema.command_path(), &["schema"]);
        assert_eq!(CliAuxCommandId::Inspect.command_path(), &["inspect"]);
    }

    #[test]
    fn document_parse_remains_core_only_in_operation_help() {
        assert!(cli_operation_help_document(OperationId::DocumentParse).is_none());
    }

    #[test]
    fn validation_helper_reports_missing_help_and_empty_catalog_fields() {
        let malformed_aux = [CliAuxCommandDescriptor {
            id: CliAuxCommandId::Catalog,
            command_path: &[],
            about: "   ",
        }];
        let select_extract = *operation_descriptor(OperationId::SelectExtract);

        let errors = cli_help_catalog_validation_errors_with(
            &malformed_aux,
            &[select_extract],
            |_| false,
            true,
        );

        assert!(
            errors
                .iter()
                .any(|error| error.contains("empty command path"))
        );
        assert!(
            errors
                .iter()
                .any(|error| error.contains("empty about string"))
        );
        assert!(
            errors
                .iter()
                .any(|error| error.contains("missing CLI help documentation"))
        );
        assert!(
            errors
                .iter()
                .any(|error| error.contains("root help examples are empty"))
        );
    }

    #[test]
    fn validation_helper_reports_empty_auxiliary_catalogs() {
        let errors = cli_help_catalog_validation_errors_with(&[], &[], |_| false, false);

        assert!(
            errors
                .iter()
                .any(|error| error.contains("cli_aux_command_catalog() is empty"))
        );
    }

    #[test]
    fn validation_helper_reports_help_for_core_only_operations() {
        let document_parse = *operation_descriptor(OperationId::DocumentParse);

        let errors = cli_help_catalog_validation_errors_with(
            cli_aux_command_catalog(),
            &[document_parse],
            |_| true,
            false,
        );

        assert!(
            errors
                .iter()
                .any(|error| error.contains("core-only in OPERATION_CATALOG"))
        );
    }
}
