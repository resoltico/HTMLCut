use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Frozen schema name for the machine-readable `htmlcut schema` report.
pub const SCHEMA_COMMAND_REPORT_SCHEMA_NAME: &str = "htmlcut.schema_report";
/// Schema version for the machine-readable `htmlcut schema` report.
pub const SCHEMA_COMMAND_REPORT_SCHEMA_VERSION: u32 = 1;

/// Stable schema reference in machine-readable CLI output.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct SchemaRefReport {
    /// Stable schema name.
    pub schema_name: String,
    /// Stable schema version.
    pub schema_version: u32,
}

/// One exported JSON-schema document.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct SchemaDocumentReport {
    /// Stable schema name.
    pub schema_name: String,
    /// Stable schema version.
    pub schema_version: u32,
    /// Surface that owns the contract.
    pub owner_surface: String,
    /// Rust type or type composition that maps to this schema.
    pub rust_shape: String,
    /// Stability class for this schema.
    pub stability: htmlcut_core::SchemaStability,
    /// Validator-grade JSON Schema document.
    pub json_schema: Value,
}

/// Top-level report emitted by `htmlcut schema`.
#[derive(Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct SchemaCommandReport {
    /// The user-facing tool name.
    pub tool: String,
    /// The CLI version string.
    pub version: String,
    /// The stable schema name for this report document.
    pub schema_name: String,
    /// The schema-report schema version.
    pub schema_version: u32,
    /// The exported JSON-schema registry profile.
    pub schema_profile: String,
    /// The manifest-backed one-line product description.
    pub description: String,
    /// The concrete command that produced this report.
    pub command: String,
    /// Exported schema documents known to this HTMLCut build.
    pub schemas: Vec<SchemaDocumentReport>,
}
