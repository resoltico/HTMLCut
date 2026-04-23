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
