use htmlcut_core::result::{DocumentInspection, ExtractionMatch, ExtractionStats};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::ErrorReportCode;

/// Frozen schema name for extraction and preview CLI reports.
pub const EXTRACTION_COMMAND_REPORT_SCHEMA_NAME: &str = "htmlcut.extraction_report";
/// Schema version for extraction and preview CLI reports.
pub const EXTRACTION_COMMAND_REPORT_SCHEMA_VERSION: u32 = 5;
/// Frozen schema name for `htmlcut inspect source` reports.
pub const SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_NAME: &str = "htmlcut.source_inspection_report";
/// Schema version for `htmlcut inspect source` reports.
pub const SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_VERSION: u32 = 3;
/// Frozen schema name for structured CLI error reports.
pub const ERROR_COMMAND_REPORT_SCHEMA_NAME: &str = "htmlcut.error_report";
/// Schema version for structured CLI error reports.
pub const ERROR_COMMAND_REPORT_SCHEMA_VERSION: u32 = 2;

/// The filesystem paths emitted when a bundle is requested.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct BundlePaths {
    /// The bundle directory that contains every emitted artifact.
    pub dir: String,
    /// The HTML artifact path inside the bundle directory.
    pub html: String,
    /// The plain-text artifact path inside the bundle directory.
    pub text: String,
    /// The structured JSON report path inside the bundle directory.
    pub report: String,
}

/// Structured report emitted by extraction and preview CLI commands.
#[derive(Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ExtractionCommandReport {
    /// The user-facing tool name.
    pub tool: String,
    /// The engine that produced the extraction result.
    pub engine: String,
    /// The CLI version string.
    pub version: String,
    /// The stable schema name for this report document.
    pub schema_name: String,
    /// The extraction report schema version.
    pub schema_version: u32,
    /// The concrete command that produced this report.
    pub command: String,
    /// The canonical cross-surface operation ID for this execution.
    pub operation_id: htmlcut_core::OperationId,
    /// Whether the command completed without error diagnostics.
    pub ok: bool,
    /// Source metadata copied from `htmlcut-core`.
    pub source: htmlcut_core::SourceMetadata,
    /// The extraction request contract that produced the result.
    pub extraction: htmlcut_core::ExtractionSpec,
    /// Match counts and timing information for the execution.
    pub stats: ExtractionStats,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// The parsed document title when one was available.
    pub document_title: Option<String>,
    /// Extracted matches in CLI report order.
    pub matches: Vec<ExtractionMatch>,
    /// Diagnostics emitted by the core engine.
    pub diagnostics: Vec<htmlcut_core::Diagnostic>,
    /// Bundle artifact paths when `--bundle` was requested.
    pub bundle: Option<BundlePaths>,
}

/// Structured report emitted by `htmlcut inspect source`.
#[derive(Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct SourceInspectionCommandReport {
    /// The user-facing tool name.
    pub tool: String,
    /// The engine that produced the inspection result.
    pub engine: String,
    /// The CLI version string.
    pub version: String,
    /// The stable schema name for this report document.
    pub schema_name: String,
    /// The source inspection schema version.
    pub schema_version: u32,
    /// The concrete command that produced this report.
    pub command: String,
    /// The canonical cross-surface operation ID for this execution.
    pub operation_id: htmlcut_core::OperationId,
    /// Whether the command completed without error diagnostics.
    pub ok: bool,
    /// Source metadata copied from `htmlcut-core`.
    pub source: htmlcut_core::SourceMetadata,
    /// Document-level inspection details when the source parsed successfully.
    pub document: Option<DocumentInspection>,
    /// Diagnostics emitted by the core engine.
    pub diagnostics: Vec<htmlcut_core::Diagnostic>,
}

/// Failure category for one structured CLI error report.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ErrorReportCategory {
    /// Invalid invocation or request contract.
    Usage,
    /// Source loading failed before extraction.
    Source,
    /// Extraction or preview failed after loading.
    Extraction,
    /// Output rendering or writeback failed.
    Output,
    /// Internal CLI failure.
    Internal,
}

/// One machine-readable diagnostic embedded in a CLI error report.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ErrorReportDiagnostic {
    /// Severity level for the diagnostic.
    pub level: htmlcut_core::DiagnosticLevel,
    /// Stable diagnostic or CLI-specific error code.
    pub code: ErrorReportCode,
    /// Human-readable diagnostic message.
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Optional structured detail payload.
    pub details: Option<Value>,
}

/// Primary failure body carried by `htmlcut.error_report`.
#[derive(Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ErrorReportBody {
    /// Stable failure category.
    pub category: ErrorReportCategory,
    /// Stable diagnostic or CLI-specific error code.
    pub code: ErrorReportCode,
    /// Human-readable primary error message.
    pub message: String,
}

/// Structured report emitted when the CLI fails before it can return a command-specific report.
#[derive(Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ErrorCommandReport {
    /// The user-facing tool name.
    pub tool: String,
    /// The engine backing the CLI.
    pub engine: String,
    /// The CLI version string.
    pub version: String,
    /// The stable schema name for this error report document.
    pub schema_name: String,
    /// The error report schema version.
    pub schema_version: u32,
    /// The concrete command that was being prepared or executed.
    pub command: String,
    /// Whether the command completed successfully.
    pub ok: bool,
    /// Process exit code mapped from the failure category.
    pub exit_code: i32,
    /// Primary error payload.
    pub error: ErrorReportBody,
    /// Structured diagnostics emitted while building the error.
    pub diagnostics: Vec<ErrorReportDiagnostic>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Structured source-load trace captured before the command failed, when available.
    pub source_load_steps: Vec<htmlcut_core::SourceLoadStep>,
}
