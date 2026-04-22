use htmlcut_core::result::{DocumentInspection, ExtractionMatch, ExtractionStats};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Frozen schema name for extraction and preview CLI reports.
pub const EXTRACTION_COMMAND_REPORT_SCHEMA_NAME: &str = "htmlcut.extraction_report";
/// Schema version for extraction and preview CLI reports.
pub const EXTRACTION_COMMAND_REPORT_SCHEMA_VERSION: u32 = 5;
/// Frozen schema name for `htmlcut inspect source` reports.
pub const SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_NAME: &str = "htmlcut.source_inspection_report";
/// Schema version for `htmlcut inspect source` reports.
pub const SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_VERSION: u32 = 3;

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
