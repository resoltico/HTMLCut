use std::path::PathBuf;

use clap::Args;

use super::{CliCatalogOutputMode, CliSchemaOutputMode, cli_choice_parser};

#[derive(Debug, Args)]
pub(crate) struct CatalogArgs {
    /// Render the catalog as detailed text or structured JSON.
    #[arg(long, value_parser = cli_choice_parser::<CliCatalogOutputMode>(), default_value_t = CliCatalogOutputMode::Text)]
    pub(crate) output: CliCatalogOutputMode,

    /// Write the stdout payload to exactly one file instead of stdout.
    #[arg(long, value_name = "PATH")]
    pub(crate) output_file: Option<PathBuf>,

    /// Filter the catalog to one stable operation ID.
    #[arg(long, value_name = "OPERATION_ID")]
    pub(crate) operation: Option<String>,
}

#[derive(Debug, Args)]
pub(crate) struct SchemaArgs {
    /// Render the schema registry as compact text or structured JSON.
    #[arg(long, value_parser = cli_choice_parser::<CliSchemaOutputMode>(), default_value_t = CliSchemaOutputMode::Text)]
    pub(crate) output: CliSchemaOutputMode,

    /// Write the stdout payload to exactly one file instead of stdout.
    #[arg(long, value_name = "PATH")]
    pub(crate) output_file: Option<PathBuf>,

    /// Filter the registry to one stable schema name.
    #[arg(long, value_name = "SCHEMA_NAME")]
    pub(crate) name: Option<String>,

    /// Filter the registry to one schema version. Requires `--name`.
    #[arg(long = "schema-version", value_name = "SCHEMA_VERSION")]
    pub(crate) schema_version: Option<u32>,
}
