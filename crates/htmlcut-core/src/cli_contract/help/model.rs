#[cfg(test)]
use std::collections::BTreeSet;

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
    /// Every canonical non-operation CLI command in declaration order.
    pub const ALL: &'static [Self] = &[Self::Catalog, Self::Schema, Self::Inspect];

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

const CATALOG_COMMAND_DESCRIPTOR: CliAuxCommandDescriptor = CliAuxCommandDescriptor {
    id: CliAuxCommandId::Catalog,
    command_path: &["catalog"],
    about: "Print the capability catalog with stable operation IDs.",
};
const SCHEMA_COMMAND_DESCRIPTOR: CliAuxCommandDescriptor = CliAuxCommandDescriptor {
    id: CliAuxCommandId::Schema,
    command_path: &["schema"],
    about: "Export validator-grade JSON schemas for HTMLCut's public JSON contracts.",
};
const INSPECT_COMMAND_DESCRIPTOR: CliAuxCommandDescriptor = CliAuxCommandDescriptor {
    id: CliAuxCommandId::Inspect,
    command_path: &["inspect"],
    about: "Explore a source or preview a request before committing to a final extraction.",
};

const CLI_AUX_COMMAND_CATALOG: &[CliAuxCommandDescriptor] = &[
    CATALOG_COMMAND_DESCRIPTOR,
    SCHEMA_COMMAND_DESCRIPTOR,
    INSPECT_COMMAND_DESCRIPTOR,
];

/// Returns the canonical catalog of non-operation CLI commands.
pub fn cli_aux_command_catalog() -> &'static [CliAuxCommandDescriptor] {
    CLI_AUX_COMMAND_CATALOG
}

/// Returns the canonical non-operation CLI descriptor for one command.
pub fn cli_aux_command_descriptor(id: CliAuxCommandId) -> &'static CliAuxCommandDescriptor {
    match id {
        CliAuxCommandId::Catalog => &CATALOG_COMMAND_DESCRIPTOR,
        CliAuxCommandId::Schema => &SCHEMA_COMMAND_DESCRIPTOR,
        CliAuxCommandId::Inspect => &INSPECT_COMMAND_DESCRIPTOR,
    }
}

/// Returns the display-form command label for one canonical non-operation CLI command.
pub fn cli_aux_command_display_command(id: CliAuxCommandId) -> String {
    cli_aux_command_descriptor(id).command_path.join(" ")
}

#[cfg(test)]
pub(crate) fn cli_aux_command_catalog_validation_errors_for_tests(
    descriptors: &[CliAuxCommandDescriptor],
) -> Vec<String> {
    cli_aux_command_catalog_validation_errors(descriptors)
}

#[cfg(test)]
pub(crate) fn assert_cli_aux_command_catalog_for_tests(descriptors: &[CliAuxCommandDescriptor]) {
    let errors = cli_aux_command_catalog_validation_errors(descriptors);
    assert!(
        errors.is_empty(),
        "cli_aux_command_catalog drifted:\n- {}",
        errors.join("\n- ")
    );
}

#[cfg(test)]
fn cli_aux_command_catalog_validation_errors(
    descriptors: &[CliAuxCommandDescriptor],
) -> Vec<String> {
    let mut errors = Vec::new();
    let mut seen_ids = BTreeSet::new();

    if descriptors.is_empty() {
        errors.push("cli_aux_command_catalog() is empty".to_owned());
    }

    for descriptor in descriptors {
        if !seen_ids.insert(descriptor.id) {
            errors.push(format!(
                "{:?} appears more than once in cli_aux_command_catalog()",
                descriptor.id
            ));
        }
        if descriptor.command_path.is_empty() {
            errors.push(format!("{:?} has an empty command path", descriptor.id));
        }
        if descriptor.about.trim().is_empty() {
            errors.push(format!("{:?} has an empty about string", descriptor.id));
        }
        if descriptor.command_path != descriptor.id.command_path() {
            errors.push(format!(
                "{:?} command path drifted: expected {:?}, found {:?}",
                descriptor.id,
                descriptor.id.command_path(),
                descriptor.command_path
            ));
        }
    }

    for id in CliAuxCommandId::ALL {
        if !seen_ids.contains(id) {
            errors.push(format!("{id:?} is missing from cli_aux_command_catalog()"));
        }
    }

    errors
}
