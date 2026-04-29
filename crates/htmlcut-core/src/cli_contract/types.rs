use std::fmt;

use serde::Serialize;

use super::render_cli_value;
use crate::catalog::OperationId;
use crate::contracts::{FetchPreflightMode, PatternMode, ValueType, WhitespaceMode};

/// Canonical input forms accepted by HTMLCut's CLI extraction and inspection commands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CliInputForm {
    /// A local filesystem path.
    LocalFilePath,
    /// An `http://` or `https://` URL.
    Url,
    /// Standard input selected with `-`.
    Stdin,
}

impl CliInputForm {
    /// Returns the stable catalog label for this input form.
    pub const fn description(self) -> &'static str {
        match self {
            Self::LocalFilePath => "local file path",
            Self::Url => "http:// or https:// URL",
            Self::Stdin => "- for stdin",
        }
    }
}

/// Canonical CLI match-retention modes exposed by HTMLCut.
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum CliSelectionMode {
    /// Require exactly one match.
    Single,
    /// Keep the first match.
    First,
    /// Keep one explicit 1-based match.
    Nth,
    /// Keep every match.
    All,
}

crate::cli_choice::impl_cli_choice!(CliSelectionMode {
    CliSelectionMode::Single => "single",
    CliSelectionMode::First => "first",
    CliSelectionMode::Nth => "nth",
    CliSelectionMode::All => "all",
});

/// Canonical stdout rendering modes exposed by HTMLCut CLI commands.
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "lowercase")]
pub enum CliOutputMode {
    /// Render compact human-readable text.
    Text,
    /// Render HTML output.
    Html,
    /// Render machine-readable JSON.
    Json,
    /// Suppress the stdout payload.
    None,
}

crate::cli_choice::impl_cli_choice!(CliOutputMode {
    CliOutputMode::Text => "text",
    CliOutputMode::Html => "html",
    CliOutputMode::Json => "json",
    CliOutputMode::None => "none",
});

/// Canonical stdout rendering modes for CLI commands that only expose text or JSON.
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "lowercase")]
pub enum CliTextJsonOutputMode {
    /// Render compact human-readable text.
    Text,
    /// Render machine-readable JSON.
    Json,
}

crate::cli_choice::impl_cli_choice!(CliTextJsonOutputMode {
    CliTextJsonOutputMode::Text => "text",
    CliTextJsonOutputMode::Json => "json",
});

impl CliTextJsonOutputMode {
    /// Returns the corresponding general CLI output mode.
    pub const fn as_output_mode(self) -> CliOutputMode {
        match self {
            Self::Text => CliOutputMode::Text,
            Self::Json => CliOutputMode::Json,
        }
    }
}

impl From<CliTextJsonOutputMode> for CliOutputMode {
    fn from(value: CliTextJsonOutputMode) -> Self {
        value.as_output_mode()
    }
}

/// Help-section grouping for one CLI parameter.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CliParameterSection {
    /// Parameters that identify and load the HTML source.
    Source,
    /// Parameters that select a reusable request definition.
    Definition,
    /// Parameters that choose which matches survive.
    Selection,
    /// Parameters that shape the final extracted payload.
    Extraction,
    /// Parameters that shape inspection output.
    InspectionOutput,
}

impl CliParameterSection {
    /// Returns the stable catalog label for this parameter section.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Source => "Source",
            Self::Definition => "Definition",
            Self::Selection => "Selection",
            Self::Extraction => "Extraction",
            Self::InspectionOutput => "Inspection Output",
        }
    }
}

impl fmt::Display for CliParameterSection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.label())
    }
}

/// Canonical identifier for one CLI parameter.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CliParameterId {
    /// Positional source input parameter.
    Input,
    /// Request-definition file path.
    RequestFile,
    /// Output path for the normalized request-definition JSON.
    EmitRequestFile,
    /// Base URL override.
    BaseUrl,
    /// Maximum allowed source size.
    MaxBytes,
    /// HTTP fetch timeout in milliseconds.
    FetchTimeoutMs,
    /// HTTP connect timeout in milliseconds.
    FetchConnectTimeoutMs,
    /// URL preflight policy.
    FetchPreflight,
    /// Inspection sample-limit option.
    SampleLimit,
    /// CSS selector option.
    Css,
    /// Match-retention mode.
    Match,
    /// Explicit 1-based match index.
    Index,
    /// Extracted value kind.
    Value,
    /// Attribute name when attribute extraction is requested.
    Attribute,
    /// Whitespace normalization policy.
    Whitespace,
    /// Relative-URL rewriting flag.
    RewriteUrls,
    /// Stdout rendering mode.
    Output,
    /// Bundle directory path.
    Bundle,
    /// Exact stdout output-file path.
    OutputFile,
    /// Preview-character limit.
    PreviewChars,
    /// Include-source-text flag.
    IncludeSourceText,
    /// Slice start boundary.
    From,
    /// Slice end boundary.
    To,
    /// Slice literal-vs-regex mode.
    Pattern,
    /// Regex flags for slice mode.
    RegexFlags,
    /// Include-start boundary flag.
    IncludeStart,
    /// Include-end boundary flag.
    IncludeEnd,
}

impl CliParameterId {
    /// Returns the stable CLI spelling for this parameter.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Input => "<INPUT>",
            Self::RequestFile => "--request-file",
            Self::EmitRequestFile => "--emit-request-file",
            Self::BaseUrl => "--base-url",
            Self::MaxBytes => "--max-bytes",
            Self::FetchTimeoutMs => "--fetch-timeout-ms",
            Self::FetchConnectTimeoutMs => "--fetch-connect-timeout-ms",
            Self::FetchPreflight => "--fetch-preflight",
            Self::SampleLimit => "--sample-limit",
            Self::Css => "--css",
            Self::Match => "--match",
            Self::Index => "--index",
            Self::Value => "--value",
            Self::Attribute => "--attribute",
            Self::Whitespace => "--whitespace",
            Self::RewriteUrls => "--rewrite-urls",
            Self::Output => "--output",
            Self::Bundle => "--bundle",
            Self::OutputFile => "--output-file",
            Self::PreviewChars => "--preview-chars",
            Self::IncludeSourceText => "--include-source-text",
            Self::From => "--from",
            Self::To => "--to",
            Self::Pattern => "--pattern",
            Self::RegexFlags => "--regex-flags",
            Self::IncludeStart => "--include-start",
            Self::IncludeEnd => "--include-end",
        }
    }
}

impl fmt::Display for CliParameterId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Transport kind for one CLI parameter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CliParameterKind {
    /// Positional parameter supplied without a flag.
    Positional,
    /// Option that carries a value.
    Option,
    /// Boolean flag without an explicit value.
    Flag,
}

/// Typed literal carried in the canonical CLI contract metadata.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CliValue {
    /// One selection-mode value.
    SelectionMode(CliSelectionMode),
    /// One extraction value kind.
    ValueType(ValueType),
    /// One stdout rendering mode.
    OutputMode(CliOutputMode),
    /// One whitespace policy.
    WhitespaceMode(WhitespaceMode),
    /// One slice pattern mode.
    PatternMode(PatternMode),
    /// One fetch preflight policy.
    FetchPreflightMode(FetchPreflightMode),
    /// One boolean literal.
    Boolean(bool),
    /// One usize literal.
    Usize(usize),
    /// One u64 literal.
    U64(u64),
}

impl fmt::Display for CliValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&render_cli_value(*self))
    }
}

/// Condition over another CLI parameter inside the canonical contract metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CliCondition {
    /// Parameter that activates the condition.
    pub parameter: CliParameterId,
    /// Accepted activating values for the condition.
    pub values: Vec<CliValue>,
}

/// One conditional default value exposed by the canonical command contract.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CliConditionalDefault {
    /// Default value applied when the condition is satisfied.
    pub value: CliValue,
    /// Activating condition for the default.
    pub when: CliCondition,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_value_display_matches_rendered_cli_value() {
        let selection = CliValue::SelectionMode(CliSelectionMode::All);
        let boolean = CliValue::Boolean(true);

        assert_eq!(selection.to_string(), render_cli_value(selection));
        assert_eq!(boolean.to_string(), render_cli_value(boolean));
    }
}

/// One cross-parameter CLI contract rule.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CliConstraint {
    /// One parameter becomes required when another parameter selects its mode.
    RequiresParameter {
        /// Parameter that becomes required.
        parameter: CliParameterId,
        /// Activating condition for the requirement.
        when: CliCondition,
    },
    /// One parameter is only valid when another parameter selects its mode.
    AllowedOnlyWhen {
        /// Parameter whose presence is restricted.
        parameter: CliParameterId,
        /// Activating condition for allowed presence.
        when: CliCondition,
    },
    /// One parameter's accepted values narrow when another parameter selects a mode.
    RestrictsParameterValues {
        /// Parameter whose values narrow.
        parameter: CliParameterId,
        /// Values allowed while the condition is active.
        allowed_values: Vec<CliValue>,
        /// Activating condition for the restriction.
        when: CliCondition,
    },
}

/// Requiredness state for one CLI parameter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CliParameterRequirement {
    /// The parameter is always required.
    Required,
    /// The parameter is always optional.
    Optional,
    /// The parameter is required unless another parameter is present.
    RequiredUnless(CliParameterId),
    /// The parameter is required when another parameter selects specific values.
    RequiredWhen(CliCondition),
    /// The parameter is allowed only when another parameter selects specific values.
    AllowedOnlyWhen(CliCondition),
}

/// Canonical metadata for one CLI parameter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CliParameterDescriptor {
    /// Help-section grouping for this parameter.
    pub section: CliParameterSection,
    /// Stable parameter identifier.
    pub id: CliParameterId,
    /// Parameter transport kind.
    pub kind: CliParameterKind,
    /// Requiredness state for this parameter.
    pub requirement: CliParameterRequirement,
    /// Placeholder or value label when the parameter carries a value.
    pub value_hint: Option<&'static str>,
    /// Default value when the CLI applies one automatically.
    pub default: Option<CliValue>,
    /// Allowed enum-like values when the parameter is constrained.
    pub allowed_values: Vec<CliValue>,
    /// Stable user-facing summary for this parameter.
    pub summary: &'static str,
}

/// Canonical CLI contract facts for one stable HTMLCut operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OperationCliContract {
    /// Stable operation identifier that owns this CLI contract.
    pub operation_id: OperationId,
    /// Command path tokens exactly as the user types them.
    pub command_path: &'static [&'static str],
    /// Canonical invocation synopsis for the operation.
    pub invocation: &'static str,
    /// Accepted source input forms for the operation.
    pub inputs: Vec<CliInputForm>,
    /// Default match-retention mode when selection is supported.
    pub default_match: Option<CliSelectionMode>,
    /// Supported match-retention modes when selection is supported.
    pub selection_modes: Vec<CliSelectionMode>,
    /// Default extracted value kind when value selection is supported.
    pub default_value: Option<ValueType>,
    /// Supported extracted value kinds when value selection is supported.
    pub value_modes: Vec<ValueType>,
    /// Unconditional default stdout rendering mode.
    pub default_output: Option<CliOutputMode>,
    /// Conditional stdout default overrides.
    pub default_output_overrides: Vec<CliConditionalDefault>,
    /// Supported stdout rendering modes for the command.
    pub output_modes: Vec<CliOutputMode>,
    /// Machine-readable cross-parameter command rules.
    pub constraints: Vec<CliConstraint>,
    /// Stable operator-facing notes for the command.
    pub notes: Vec<&'static str>,
    /// Stable example invocations for the command.
    pub examples: Vec<&'static str>,
    /// Parameter inventory for the command.
    pub parameters: Vec<CliParameterDescriptor>,
}

impl OperationCliContract {
    /// Returns the display-form command label used in help and catalog text.
    pub fn display_command(&self) -> String {
        self.command_path.join(" ")
    }

    /// Returns the normalized report command label used in CLI reports.
    pub fn report_command(&self) -> String {
        self.command_path.join("-")
    }
}
