use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Frozen schema name for the machine-readable `htmlcut schema` report.
pub const SCHEMA_COMMAND_REPORT_SCHEMA_NAME: &str = "htmlcut.schema_report";
/// Schema version for the machine-readable `htmlcut schema` report.
pub const SCHEMA_COMMAND_REPORT_SCHEMA_VERSION: u32 = 2;
/// Frozen schema name for the lightweight machine-readable `htmlcut schema` inventory report.
pub const SCHEMA_INVENTORY_REPORT_SCHEMA_NAME: &str = "htmlcut.schema_inventory_report";
/// Schema version for the lightweight machine-readable `htmlcut schema` inventory report.
pub const SCHEMA_INVENTORY_REPORT_SCHEMA_VERSION: u32 = 1;

/// Stable schema reference in machine-readable CLI output.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct SchemaRefReport {
    /// Stable schema name.
    pub schema_name: String,
    /// Stable schema version.
    pub schema_version: u32,
}

/// One exported JSON-schema document plus its public contract identity.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct SchemaDocumentReport {
    /// Stable schema name.
    pub schema_name: String,
    /// Stable schema version.
    pub schema_version: u32,
    /// Public surface label for this contract family.
    pub surface: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Public profile label when the contract belongs to one named integration profile.
    pub profile: Option<String>,
    /// Public artifact name exposed to operators and embedders.
    pub artifact: String,
    /// Stability class for this schema.
    pub stability: htmlcut_core::SchemaStability,
    /// Validator-grade JSON Schema document.
    pub json_schema: Value,
}

/// One exported schema identity plus its public metadata, without the embedded JSON Schema document.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct SchemaInventoryEntryReport {
    /// Stable schema name.
    pub schema_name: String,
    /// Stable schema version.
    pub schema_version: u32,
    /// Public surface label for this contract family.
    pub surface: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Public profile label when the contract belongs to one named integration profile.
    pub profile: Option<String>,
    /// Public artifact name exposed to operators and embedders.
    pub artifact: String,
    /// Stability class for this schema.
    pub stability: htmlcut_core::SchemaStability,
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

/// Top-level report emitted by `htmlcut schema --output index-json`.
#[derive(Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct SchemaInventoryCommandReport {
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
    /// Exported schema identities known to this HTMLCut build.
    pub schemas: Vec<SchemaInventoryEntryReport>,
}
