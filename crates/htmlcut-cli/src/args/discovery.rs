use std::path::PathBuf;

use clap::Args;

use super::{CliCatalogOutputMode, CliSchemaOutputMode, OutputFileWriteArgs, cli_choice_parser};

#[derive(Debug, Default, Args)]
#[command(next_help_heading = "Catalog Filter")]
pub(crate) struct CatalogFilterArgs {
    /// Filter the catalog to one stable operation ID.
    #[arg(long, value_name = "OPERATION_ID")]
    pub(crate) operation: Option<String>,
}

#[derive(Debug, Default, Args)]
#[command(next_help_heading = "Schema Filter")]
pub(crate) struct SchemaFilterArgs {
    /// Filter the registry to one stable schema name.
    #[arg(long, value_name = "SCHEMA_NAME")]
    pub(crate) name: Option<String>,

    /// Filter the registry to one schema version. Requires `--name`.
    #[arg(long = "schema-version", value_name = "SCHEMA_VERSION")]
    pub(crate) schema_version: Option<u32>,
}

#[derive(Debug, Args)]
pub(crate) struct CatalogArgs {
    /// Render the catalog as detailed text or structured JSON.
    #[arg(long, value_parser = cli_choice_parser::<CliCatalogOutputMode>(), default_value_t = CliCatalogOutputMode::Text)]
    pub(crate) output: CliCatalogOutputMode,

    /// Write the stdout payload to exactly one file instead of stdout.
    ///
    /// Parent directories are created automatically. Existing files require `--overwrite`.
    #[arg(long, value_name = "PATH")]
    pub(crate) output_file: Option<PathBuf>,

    #[command(flatten)]
    pub(crate) filter: CatalogFilterArgs,

    #[command(flatten)]
    pub(crate) file_write: OutputFileWriteArgs,
}

#[derive(Debug, Args)]
pub(crate) struct SchemaArgs {
    /// Render the schema registry as compact text, full JSON Schema documents, or a lightweight machine-readable inventory.
    #[arg(long, value_parser = cli_choice_parser::<CliSchemaOutputMode>(), default_value_t = CliSchemaOutputMode::Text)]
    pub(crate) output: CliSchemaOutputMode,

    /// Write the stdout payload to exactly one file instead of stdout.
    ///
    /// Parent directories are created automatically. Existing files require `--overwrite`.
    #[arg(long, value_name = "PATH")]
    pub(crate) output_file: Option<PathBuf>,

    #[command(flatten)]
    pub(crate) filter: SchemaFilterArgs,

    #[command(flatten)]
    pub(crate) file_write: OutputFileWriteArgs,
}
