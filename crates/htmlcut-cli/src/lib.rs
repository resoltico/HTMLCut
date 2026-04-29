//! Command-line workflows for HTMLCut's extraction and inspection engine.
#![deny(missing_docs)]

/// Exit code for internal CLI failures.
pub const EXIT_CODE_INTERNAL: i32 = 1;
/// Exit code for invalid CLI usage.
pub const EXIT_CODE_USAGE: i32 = 2;
/// Exit code for source loading failures.
pub const EXIT_CODE_SOURCE: i32 = 3;
/// Exit code for extraction or inspection failures.
pub const EXIT_CODE_EXTRACTION: i32 = 4;
/// Exit code for output rendering or writing failures.
pub const EXIT_CODE_OUTPUT: i32 = 5;

mod args;
mod error;
mod execute;
mod help;
mod lookup;
mod metadata;
mod model;
mod prepare;
mod render;
#[cfg(test)]
mod tests;

use clap::Command;

pub use execute::run;
pub use model::{
    BundlePaths, CATALOG_REPORT_SCHEMA_NAME, CATALOG_SCHEMA_VERSION, CatalogAvailability,
    CatalogCommandContract, CatalogCommandReport, CatalogCondition, CatalogConditionalDefault,
    CatalogConstraint, CatalogContractSurface, CatalogOperationReport, CatalogParameterKind,
    CatalogParameterRequirement, CatalogParameterSpec, CliErrorCode,
    ERROR_COMMAND_REPORT_SCHEMA_NAME, ERROR_COMMAND_REPORT_SCHEMA_VERSION,
    EXTRACTION_COMMAND_REPORT_SCHEMA_NAME, EXTRACTION_COMMAND_REPORT_SCHEMA_VERSION,
    ErrorCommandReport, ErrorReportBody, ErrorReportCategory, ErrorReportCode,
    ErrorReportDiagnostic, ExtractionCommandReport, SCHEMA_COMMAND_REPORT_SCHEMA_NAME,
    SCHEMA_COMMAND_REPORT_SCHEMA_VERSION, SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_NAME,
    SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_VERSION, SchemaCommandReport, SchemaDocumentReport,
    SchemaRefReport, SourceInspectionCommandReport,
};

/// Builds the canonical HTMLCut clap command tree for tooling and docs-contract validation.
pub fn command() -> Command {
    use clap::CommandFactory;

    args::Cli::command()
}
