use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::SchemaRefReport;

/// Frozen schema name for the machine-readable `htmlcut catalog` report.
pub const CATALOG_REPORT_SCHEMA_NAME: &str = "htmlcut.catalog_report";
/// Schema version for the machine-readable `htmlcut catalog` report.
pub const CATALOG_SCHEMA_VERSION: u32 = 4;

/// Rust and JSON-schema contract surface for one operation input or output.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct CatalogContractSurface {
    /// Rust type or type composition used in-process.
    pub rust_shape: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Exported JSON schema references for this contract.
    pub schema_refs: Vec<SchemaRefReport>,
}

/// States whether a canonical operation is exposed by the CLI or only by the core crate.
#[derive(Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum CatalogAvailability {
    /// The operation is available through a first-class CLI command.
    Cli,
    /// The operation exists only for embeddable `htmlcut-core` callers.
    CoreOnly,
}

/// Parameter location within one CLI command contract.
#[derive(Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum CatalogParameterKind {
    /// Positional parameter supplied without a flag.
    Positional,
    /// Option that carries a value.
    Option,
    /// Boolean flag without an explicit value payload.
    Flag,
}

/// Requiredness state for one CLI parameter.
#[derive(Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum CatalogParameterRequirement {
    /// The parameter is always required.
    Required,
    /// The parameter is always optional.
    Optional,
    /// The parameter is required only when another parameter selects its mode.
    Conditional,
}

/// Machine-readable description of one CLI parameter.
#[derive(Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct CatalogParameterSpec {
    /// Help-section grouping for this parameter.
    pub section: String,
    /// Parameter spelling exactly as exposed by the CLI.
    pub name: String,
    /// Parameter transport kind.
    pub kind: CatalogParameterKind,
    /// Requiredness state for this parameter.
    pub requirement: CatalogParameterRequirement,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Human-readable condition when `requirement = conditional`.
    pub requirement_note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Placeholder or value label when the parameter carries a value.
    pub value_hint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Default value when the CLI applies one automatically.
    pub default: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Allowed enum-like values when the parameter is constrained.
    pub allowed_values: Vec<String>,
    /// Stable user-facing summary for this parameter.
    pub summary: String,
}

/// One condition over another CLI parameter inside the catalog contract.
#[derive(Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct CatalogCondition {
    /// Parameter spelling exactly as exposed by the CLI.
    pub parameter: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Values that activate this condition.
    pub values: Vec<String>,
}

/// One conditional default exposed by the machine-readable catalog.
#[derive(Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct CatalogConditionalDefault {
    /// Default value applied when the condition is satisfied.
    pub value: String,
    /// Activating condition for this default.
    pub when: CatalogCondition,
}

/// Machine-readable cross-parameter rule for one CLI command.
#[derive(Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum CatalogConstraint {
    /// One parameter becomes required when another parameter selects its mode.
    RequiresParameter {
        /// Parameter spelling exactly as exposed by the CLI.
        parameter: String,
        /// Activating condition for the requirement.
        when: CatalogCondition,
    },
    /// One parameter is only valid when another parameter selects its mode.
    AllowedOnlyWhen {
        /// Parameter spelling exactly as exposed by the CLI.
        parameter: String,
        /// Activating condition for the allowed presence.
        when: CatalogCondition,
    },
    /// One parameter's accepted values narrow when another parameter selects a mode.
    RestrictsParameterValues {
        /// Parameter spelling exactly as exposed by the CLI.
        parameter: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        /// Values allowed while the condition is active.
        allowed_values: Vec<String>,
        /// Activating condition for the restriction.
        when: CatalogCondition,
    },
}

/// Machine-readable CLI command contract for one cataloged operation.
#[derive(Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct CatalogCommandContract {
    /// Canonical CLI invocation for this operation.
    pub invocation: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Accepted input forms when the command consumes HTML input.
    pub inputs: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Default match-retention mode when the command supports selection.
    pub default_match: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Supported match-retention modes when the command supports selection.
    pub selection_modes: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Default extracted value kind when the command supports value selection.
    pub default_value: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Supported extracted value kinds when the command supports value selection.
    pub value_modes: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Unconditional default stdout rendering mode for this command.
    pub default_output: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Conditional stdout default overrides for this command.
    pub default_output_overrides: Vec<CatalogConditionalDefault>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Supported stdout rendering modes for this command.
    pub output_modes: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Machine-readable cross-parameter command rules.
    pub constraints: Vec<CatalogConstraint>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Stable operator-facing notes that define important behavior or constraints.
    pub notes: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Stable example invocations for this command.
    pub examples: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    /// Parameter inventory for this command.
    pub parameters: Vec<CatalogParameterSpec>,
}

/// Structured catalog entry for one canonical HTMLCut operation.
#[derive(Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct CatalogOperationReport {
    /// The stable canonical operation identifier.
    pub operation_id: htmlcut_core::OperationId,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// The CLI command surface for this operation, when one exists.
    pub command: Option<String>,
    /// Whether this operation is exposed by the CLI or is core-only.
    pub availability: CatalogAvailability,
    /// A concise summary of what the operation does.
    pub summary: String,
    /// The embeddable core surface that exposes this operation.
    pub core_surface: String,
    /// The public request contract for this operation.
    pub request_contract: CatalogContractSurface,
    /// The public result contract for this operation.
    pub result_contract: CatalogContractSurface,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Machine-readable CLI command contract when the operation has a first-class CLI surface.
    pub command_contract: Option<CatalogCommandContract>,
}

/// Top-level report emitted by `htmlcut catalog`.
#[derive(Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct CatalogCommandReport {
    /// The user-facing tool name.
    pub tool: String,
    /// The CLI version string.
    pub version: String,
    /// The stable schema name for this report document.
    pub schema_name: String,
    /// The catalog report schema version.
    pub schema_version: u32,
    /// The exported JSON-schema registry profile.
    pub schema_profile: String,
    /// The manifest-backed one-line product description.
    pub description: String,
    /// The concrete command that produced this report.
    pub command: String,
    /// The operations known to this HTMLCut build.
    pub operations: Vec<CatalogOperationReport>,
}
