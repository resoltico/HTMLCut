use std::fmt;
use std::sync::LazyLock;

use serde::Serialize;

use crate::catalog::OperationId;
#[cfg(test)]
use crate::catalog::{operation_catalog, operation_descriptor};
use crate::contracts::{
    DEFAULT_FETCH_TIMEOUT_MS, DEFAULT_INSPECTION_SAMPLE_LIMIT, DEFAULT_MAX_BYTES,
    DEFAULT_PREVIEW_CHARS, FetchPreflightMode, PatternMode, ValueType, WhitespaceMode,
};
#[cfg(test)]
use std::collections::BTreeSet;

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

static CLI_OPERATION_CATALOG: LazyLock<Vec<OperationCliContract>> = LazyLock::new(|| {
    vec![
        build_source_inspect_contract(),
        build_select_preview_contract(),
        build_slice_preview_contract(),
        build_select_extract_contract(),
        build_slice_extract_contract(),
    ]
});

/// Returns the canonical CLI operation-contract catalog.
pub fn cli_operation_catalog() -> &'static [OperationCliContract] {
    CLI_OPERATION_CATALOG.as_slice()
}

/// Returns the canonical CLI command contract for one operation, when the CLI exposes it.
pub fn cli_operation_contract(operation_id: OperationId) -> Option<&'static OperationCliContract> {
    cli_operation_catalog()
        .iter()
        .find(|contract| contract.operation_id == operation_id)
}

/// Returns the canonical display-form CLI command for one operation, when the CLI exposes it.
pub fn cli_operation_display_command(operation_id: OperationId) -> Option<String> {
    cli_operation_contract(operation_id).map(OperationCliContract::display_command)
}

/// Returns the canonical report-form CLI command for one operation, when the CLI exposes it.
pub fn cli_operation_report_command(operation_id: OperationId) -> Option<String> {
    cli_operation_contract(operation_id).map(OperationCliContract::report_command)
}

/// Finds the canonical CLI operation contract that matches one concrete command path.
pub fn find_cli_operation_by_command_path(
    command_path: &[&str],
) -> Option<&'static OperationCliContract> {
    cli_operation_catalog()
        .iter()
        .find(|contract| contract.command_path == command_path)
}

/// Renders one typed CLI contract value as the stable user-facing string form.
pub fn render_cli_value(value: CliValue) -> String {
    match value {
        CliValue::SelectionMode(mode) => render_enum_name(mode),
        CliValue::ValueType(value_type) => render_enum_name(value_type),
        CliValue::OutputMode(mode) => render_enum_name(mode),
        CliValue::WhitespaceMode(mode) => render_enum_name(mode),
        CliValue::PatternMode(mode) => render_enum_name(mode),
        CliValue::FetchPreflightMode(mode) => render_enum_name(mode),
        CliValue::Boolean(value) => value.to_string(),
        CliValue::Usize(value) => value.to_string(),
        CliValue::U64(value) => value.to_string(),
    }
}

#[cfg(test)]
pub(crate) fn cli_operation_catalog_validation_errors() -> Vec<String> {
    let mut errors = Vec::new();
    let mut seen_ids = BTreeSet::new();
    let cli_ids = cli_operation_catalog()
        .iter()
        .map(|contract| contract.operation_id)
        .collect::<BTreeSet<_>>();

    for descriptor in operation_catalog() {
        match (descriptor.cli_surface, cli_ids.contains(&descriptor.id)) {
            (Some(_), false) => errors.push(format!(
                "{} is marked CLI-visible in OPERATION_CATALOG but missing from cli_operation_catalog()",
                descriptor.id
            )),
            (None, true) => errors.push(format!(
                "{} appears in cli_operation_catalog() but is marked core-only in OPERATION_CATALOG",
                descriptor.id
            )),
            (Some(_), true) | (None, false) => {}
        }
    }

    for contract in cli_operation_catalog() {
        if !seen_ids.insert(contract.operation_id) {
            errors.push(format!(
                "{} appears more than once in cli_operation_catalog()",
                contract.operation_id
            ));
        }

        let descriptor = operation_descriptor(contract.operation_id);
        let display_command = contract.display_command();
        if descriptor.cli_surface != Some(display_command.as_str()) {
            errors.push(format!(
                "{} display command drifted: OPERATION_CATALOG={:?}, cli contract={display_command:?}",
                contract.operation_id, descriptor.cli_surface
            ));
        }

        validate_command_contract(contract, &mut errors);
    }

    errors
}

#[cfg(test)]
fn validate_command_contract(contract: &OperationCliContract, errors: &mut Vec<String>) {
    let scope = contract.operation_id;
    let display_command = contract.display_command();
    let parameter_ids = contract
        .parameters
        .iter()
        .map(|parameter| parameter.id)
        .collect::<BTreeSet<_>>();

    if contract.command_path.is_empty() {
        errors.push(format!("{scope} has an empty command path"));
    }

    if !contract
        .invocation
        .starts_with(&format!("htmlcut {display_command}"))
    {
        errors.push(format!(
            "{scope} invocation {:?} does not start with \"htmlcut {display_command}\"",
            contract.invocation
        ));
    }

    for example in &contract.examples {
        if !example.starts_with(&format!("htmlcut {display_command}")) {
            errors.push(format!(
                "{scope} example {example:?} does not start with \"htmlcut {display_command}\""
            ));
        }
    }

    if let Some(default_match) = contract.default_match
        && !contract.selection_modes.contains(&default_match)
    {
        errors.push(format!(
            "{scope} default_match {:?} is not present in selection_modes",
            render_cli_value(CliValue::SelectionMode(default_match))
        ));
    }

    if let Some(default_value) = contract.default_value
        && !contract.value_modes.contains(&default_value)
    {
        errors.push(format!(
            "{scope} default_value {:?} is not present in value_modes",
            render_cli_value(CliValue::ValueType(default_value))
        ));
    }

    if let Some(default_output) = contract.default_output
        && !contract.output_modes.contains(&default_output)
    {
        errors.push(format!(
            "{scope} default_output {:?} is not present in output_modes",
            render_cli_value(CliValue::OutputMode(default_output))
        ));
    }

    for override_default in &contract.default_output_overrides {
        if !contract
            .output_modes
            .contains(&match override_default.value {
                CliValue::OutputMode(mode) => mode,
                _ => {
                    errors.push(format!(
                        "{scope} default_output_override {:?} is not an output mode",
                        render_cli_value(override_default.value)
                    ));
                    continue;
                }
            })
        {
            errors.push(format!(
                "{scope} default_output_override {:?} is not present in output_modes",
                render_cli_value(override_default.value)
            ));
        }

        validate_condition(
            scope,
            "default_output_override",
            &override_default.when,
            &contract.parameters,
            errors,
        );
    }

    let mut seen_parameters = BTreeSet::new();
    for parameter in &contract.parameters {
        if !seen_parameters.insert(parameter.id) {
            errors.push(format!("{scope} lists {} more than once", parameter.id));
        }

        if matches!(parameter.kind, CliParameterKind::Flag) {
            if parameter.value_hint.is_some() {
                errors.push(format!(
                    "{scope} flag {} carries a value_hint",
                    parameter.id
                ));
            }
            if parameter.default != Some(CliValue::Boolean(false)) {
                errors.push(format!(
                    "{scope} flag {} should default to false in the catalog metadata",
                    parameter.id
                ));
            }
        }

        if !parameter.allowed_values.is_empty()
            && let Some(default) = parameter.default
            && !parameter.allowed_values.contains(&default)
        {
            errors.push(format!(
                "{scope} parameter {} default {:?} is not present in allowed_values",
                parameter.id,
                render_cli_value(default)
            ));
        }

        match &parameter.requirement {
            CliParameterRequirement::Required | CliParameterRequirement::Optional => {}
            CliParameterRequirement::RequiredUnless(other) => {
                if !parameter_ids.contains(other) {
                    errors.push(format!(
                        "{scope} parameter {} depends on missing parameter {}",
                        parameter.id, other
                    ));
                }
            }
            CliParameterRequirement::RequiredWhen(condition)
            | CliParameterRequirement::AllowedOnlyWhen(condition) => {
                validate_condition(
                    scope,
                    parameter.id.as_str(),
                    condition,
                    &contract.parameters,
                    errors,
                );
            }
        }
    }

    if let Some(parameter) = parameter_descriptor(contract, CliParameterId::Match)
        && parameter.allowed_values != selection_mode_values(&contract.selection_modes)
    {
        errors.push(format!(
            "{scope} parameter --match drifted from selection_modes"
        ));
    }

    if let Some(parameter) = parameter_descriptor(contract, CliParameterId::Value)
        && parameter.allowed_values != value_type_values(&contract.value_modes)
    {
        errors.push(format!(
            "{scope} parameter --value drifted from value_modes"
        ));
    }

    if let Some(parameter) = parameter_descriptor(contract, CliParameterId::Output)
        && parameter.allowed_values != output_mode_values(&contract.output_modes)
    {
        errors.push(format!(
            "{scope} parameter --output drifted from output_modes"
        ));
    }

    for constraint in &contract.constraints {
        match constraint {
            CliConstraint::RequiresParameter { parameter, when }
            | CliConstraint::AllowedOnlyWhen { parameter, when } => {
                if !parameter_ids.contains(parameter) {
                    errors.push(format!(
                        "{scope} constraint references missing parameter {parameter}"
                    ));
                }
                validate_condition(scope, "constraint", when, &contract.parameters, errors);
            }
            CliConstraint::RestrictsParameterValues {
                parameter,
                allowed_values,
                when,
            } => {
                if !parameter_ids.contains(parameter) {
                    errors.push(format!(
                        "{scope} value restriction references missing parameter {parameter}"
                    ));
                }
                validate_condition(scope, "constraint", when, &contract.parameters, errors);
                if let Some(target) = parameter_descriptor(contract, *parameter)
                    && !target.allowed_values.is_empty()
                    && !allowed_values
                        .iter()
                        .all(|value| target.allowed_values.contains(value))
                {
                    errors.push(format!(
                        "{scope} value restriction on {parameter} references values outside its allowed_values"
                    ));
                }
            }
        }
    }
}

#[cfg(test)]
fn validate_condition(
    operation_id: OperationId,
    scope: &str,
    condition: &CliCondition,
    parameters: &[CliParameterDescriptor],
    errors: &mut Vec<String>,
) {
    let Some(parameter) = parameters
        .iter()
        .find(|parameter| parameter.id == condition.parameter)
    else {
        errors.push(format!(
            "{operation_id} {scope} references missing condition parameter {}",
            condition.parameter
        ));
        return;
    };

    if parameter.allowed_values.is_empty() {
        errors.push(format!(
            "{operation_id} {scope} references condition parameter {} without allowed_values",
            condition.parameter
        ));
        return;
    }

    if !condition
        .values
        .iter()
        .all(|value| parameter.allowed_values.contains(value))
    {
        errors.push(format!(
            "{operation_id} {scope} references unsupported values for {}",
            condition.parameter
        ));
    }
}

#[cfg(test)]
fn parameter_descriptor(
    contract: &OperationCliContract,
    parameter_id: CliParameterId,
) -> Option<&CliParameterDescriptor> {
    contract
        .parameters
        .iter()
        .find(|parameter| parameter.id == parameter_id)
}

fn render_enum_name<T: Serialize>(value: T) -> String {
    serde_json::to_string(&value)
        .expect("catalog enum values should always serialize")
        .trim_matches('"')
        .to_owned()
}

fn common_input_forms() -> Vec<CliInputForm> {
    vec![
        CliInputForm::LocalFilePath,
        CliInputForm::Url,
        CliInputForm::Stdin,
    ]
}

fn common_selection_modes() -> Vec<CliSelectionMode> {
    vec![
        CliSelectionMode::Single,
        CliSelectionMode::First,
        CliSelectionMode::Nth,
        CliSelectionMode::All,
    ]
}

fn inspect_output_modes() -> Vec<CliOutputMode> {
    vec![CliOutputMode::Text, CliOutputMode::Json]
}

fn extract_output_modes() -> Vec<CliOutputMode> {
    vec![
        CliOutputMode::Text,
        CliOutputMode::Html,
        CliOutputMode::Json,
        CliOutputMode::None,
    ]
}

fn extract_value_modes() -> Vec<ValueType> {
    vec![
        ValueType::Text,
        ValueType::InnerHtml,
        ValueType::OuterHtml,
        ValueType::Attribute,
        ValueType::Structured,
    ]
}

fn selection_mode_values(modes: &[CliSelectionMode]) -> Vec<CliValue> {
    modes.iter().copied().map(CliValue::SelectionMode).collect()
}

fn output_mode_values(modes: &[CliOutputMode]) -> Vec<CliValue> {
    modes.iter().copied().map(CliValue::OutputMode).collect()
}

fn value_type_values(modes: &[ValueType]) -> Vec<CliValue> {
    modes.iter().copied().map(CliValue::ValueType).collect()
}

fn whitespace_values() -> Vec<CliValue> {
    vec![
        CliValue::WhitespaceMode(WhitespaceMode::Preserve),
        CliValue::WhitespaceMode(WhitespaceMode::Normalize),
    ]
}

fn fetch_preflight_values() -> Vec<CliValue> {
    vec![
        CliValue::FetchPreflightMode(FetchPreflightMode::HeadFirst),
        CliValue::FetchPreflightMode(FetchPreflightMode::GetOnly),
    ]
}

fn pattern_values() -> Vec<CliValue> {
    vec![
        CliValue::PatternMode(PatternMode::Literal),
        CliValue::PatternMode(PatternMode::Regex),
    ]
}

fn condition(parameter: CliParameterId, values: Vec<CliValue>) -> CliCondition {
    CliCondition { parameter, values }
}

fn conditional_default(value: CliValue, when: CliCondition) -> CliConditionalDefault {
    CliConditionalDefault { value, when }
}

fn restricts_parameter_values(
    parameter: CliParameterId,
    allowed_values: Vec<CliValue>,
    when: CliCondition,
) -> CliConstraint {
    CliConstraint::RestrictsParameterValues {
        parameter,
        allowed_values,
        when,
    }
}

fn requires_parameter(parameter: CliParameterId, when: CliCondition) -> CliConstraint {
    CliConstraint::RequiresParameter { parameter, when }
}

fn constraints_with_parameter_rules(
    parameters: &[CliParameterDescriptor],
    mut extra_constraints: Vec<CliConstraint>,
) -> Vec<CliConstraint> {
    let mut constraints = Vec::new();

    for parameter in parameters {
        match &parameter.requirement {
            CliParameterRequirement::RequiredWhen(when) => {
                constraints.push(CliConstraint::RequiresParameter {
                    parameter: parameter.id,
                    when: when.clone(),
                });
            }
            CliParameterRequirement::AllowedOnlyWhen(when) => {
                constraints.push(CliConstraint::AllowedOnlyWhen {
                    parameter: parameter.id,
                    when: when.clone(),
                });
            }
            CliParameterRequirement::Required | CliParameterRequirement::Optional => {}
            CliParameterRequirement::RequiredUnless(_) => {}
        }
    }

    constraints.append(&mut extra_constraints);
    constraints
}

fn param_positional(
    section: CliParameterSection,
    id: CliParameterId,
    requirement: CliParameterRequirement,
    summary: &'static str,
) -> CliParameterDescriptor {
    CliParameterDescriptor {
        section,
        id,
        kind: CliParameterKind::Positional,
        requirement,
        value_hint: None,
        default: None,
        allowed_values: Vec::new(),
        summary,
    }
}

fn param_option(
    section: CliParameterSection,
    id: CliParameterId,
    requirement: CliParameterRequirement,
    value_hint: &'static str,
    default: Option<CliValue>,
    allowed_values: Vec<CliValue>,
    summary: &'static str,
) -> CliParameterDescriptor {
    CliParameterDescriptor {
        section,
        id,
        kind: CliParameterKind::Option,
        requirement,
        value_hint: Some(value_hint),
        default,
        allowed_values,
        summary,
    }
}

fn param_flag(
    section: CliParameterSection,
    id: CliParameterId,
    summary: &'static str,
) -> CliParameterDescriptor {
    CliParameterDescriptor {
        section,
        id,
        kind: CliParameterKind::Flag,
        requirement: CliParameterRequirement::Optional,
        value_hint: None,
        default: Some(CliValue::Boolean(false)),
        allowed_values: Vec::new(),
        summary,
    }
}

fn common_source_parameters(
    input_requirement: CliParameterRequirement,
) -> Vec<CliParameterDescriptor> {
    vec![
        param_option(
            CliParameterSection::Source,
            CliParameterId::BaseUrl,
            CliParameterRequirement::Optional,
            "URL",
            None,
            Vec::new(),
            "Override the input base URL used for relative-link resolution.",
        ),
        param_option(
            CliParameterSection::Source,
            CliParameterId::MaxBytes,
            CliParameterRequirement::Optional,
            "SIZE",
            Some(CliValue::Usize(DEFAULT_MAX_BYTES)),
            Vec::new(),
            "Refuse sources larger than this limit. Accepts raw bytes or KB, MB, and GB.",
        ),
        param_option(
            CliParameterSection::Source,
            CliParameterId::FetchTimeoutMs,
            CliParameterRequirement::Optional,
            "MILLISECONDS",
            Some(CliValue::U64(DEFAULT_FETCH_TIMEOUT_MS)),
            Vec::new(),
            "HTTP fetch timeout in milliseconds for URL inputs.",
        ),
        param_option(
            CliParameterSection::Source,
            CliParameterId::FetchPreflight,
            CliParameterRequirement::Optional,
            "FETCH_PREFLIGHT",
            Some(CliValue::FetchPreflightMode(FetchPreflightMode::HeadFirst)),
            fetch_preflight_values(),
            "Probe remote URLs with HEAD before GET, automatically falling back when HEAD is rejected or broken, or skip the HEAD preflight entirely.",
        ),
        param_positional(
            CliParameterSection::Source,
            CliParameterId::Input,
            input_requirement,
            "HTML input source: a local file path, an http(s) URL, or - for stdin.",
        ),
    ]
}

fn common_definition_parameters() -> Vec<CliParameterDescriptor> {
    vec![
        param_option(
            CliParameterSection::Definition,
            CliParameterId::RequestFile,
            CliParameterRequirement::Optional,
            "PATH",
            None,
            Vec::new(),
            "Load a reusable extraction definition from a JSON file that matches HTMLCut's extraction-definition schema.",
        ),
        param_option(
            CliParameterSection::Definition,
            CliParameterId::EmitRequestFile,
            CliParameterRequirement::Optional,
            "PATH",
            None,
            Vec::new(),
            "Write the normalized extraction definition used for this run to a JSON file.",
        ),
    ]
}

fn request_file_aware_source_parameters() -> Vec<CliParameterDescriptor> {
    common_source_parameters(CliParameterRequirement::RequiredUnless(
        CliParameterId::RequestFile,
    ))
}

fn common_selection_parameters() -> Vec<CliParameterDescriptor> {
    let selection_modes = common_selection_modes();
    vec![
        param_option(
            CliParameterSection::Selection,
            CliParameterId::Match,
            CliParameterRequirement::Optional,
            "MATCH",
            Some(CliValue::SelectionMode(CliSelectionMode::First)),
            selection_mode_values(&selection_modes),
            "Require exactly one match, keep the first match, keep one 1-based match, or keep every match.",
        ),
        param_option(
            CliParameterSection::Selection,
            CliParameterId::Index,
            CliParameterRequirement::RequiredWhen(condition(
                CliParameterId::Match,
                vec![CliValue::SelectionMode(CliSelectionMode::Nth)],
            )),
            "INDEX",
            None,
            Vec::new(),
            "The 1-based match index when --match nth is used.",
        ),
    ]
}

fn common_extract_parameters() -> Vec<CliParameterDescriptor> {
    let value_modes = extract_value_modes();
    let output_modes = extract_output_modes();
    vec![
        param_option(
            CliParameterSection::Extraction,
            CliParameterId::Value,
            CliParameterRequirement::Optional,
            "VALUE",
            Some(CliValue::ValueType(ValueType::Text)),
            value_type_values(&value_modes),
            "What each selected match should produce before stdout formatting is applied.",
        ),
        param_option(
            CliParameterSection::Extraction,
            CliParameterId::Attribute,
            CliParameterRequirement::RequiredWhen(condition(
                CliParameterId::Value,
                vec![CliValue::ValueType(ValueType::Attribute)],
            )),
            "ATTRIBUTE",
            None,
            Vec::new(),
            "Attribute name to extract when --value attribute is used.",
        ),
        param_option(
            CliParameterSection::Extraction,
            CliParameterId::Whitespace,
            CliParameterRequirement::Optional,
            "WHITESPACE",
            Some(CliValue::WhitespaceMode(WhitespaceMode::Preserve)),
            whitespace_values(),
            "Preserve source whitespace or normalize it for text-like values.",
        ),
        param_flag(
            CliParameterSection::Extraction,
            CliParameterId::RewriteUrls,
            "Rewrite relative URLs in extracted HTML and attributes with the effective base URL.",
        ),
        param_option(
            CliParameterSection::Extraction,
            CliParameterId::Output,
            CliParameterRequirement::Optional,
            "OUTPUT",
            None,
            output_mode_values(&output_modes),
            "How stdout should be rendered after extraction.",
        ),
        param_option(
            CliParameterSection::Extraction,
            CliParameterId::Bundle,
            CliParameterRequirement::Optional,
            "BUNDLE",
            None,
            Vec::new(),
            "Write report.json, selection.html, and selection.txt to this directory.",
        ),
        param_option(
            CliParameterSection::Extraction,
            CliParameterId::OutputFile,
            CliParameterRequirement::Optional,
            "PATH",
            None,
            Vec::new(),
            "Write the stdout payload to exactly one file instead of stdout.",
        ),
        param_option(
            CliParameterSection::Extraction,
            CliParameterId::PreviewChars,
            CliParameterRequirement::Optional,
            "PREVIEW_CHARS",
            Some(CliValue::Usize(DEFAULT_PREVIEW_CHARS)),
            Vec::new(),
            "Maximum preview length stored in structured reports.",
        ),
        param_flag(
            CliParameterSection::Extraction,
            CliParameterId::IncludeSourceText,
            "Include the full source text inside structured reports and bundles.",
        ),
    ]
}

fn common_inspect_output_parameters() -> Vec<CliParameterDescriptor> {
    let output_modes = inspect_output_modes();
    vec![
        param_option(
            CliParameterSection::InspectionOutput,
            CliParameterId::Output,
            CliParameterRequirement::Optional,
            "OUTPUT",
            Some(CliValue::OutputMode(CliOutputMode::Json)),
            output_mode_values(&output_modes),
            "Render the inspection as compact text or structured JSON.",
        ),
        param_option(
            CliParameterSection::InspectionOutput,
            CliParameterId::PreviewChars,
            CliParameterRequirement::Optional,
            "PREVIEW_CHARS",
            Some(CliValue::Usize(DEFAULT_PREVIEW_CHARS)),
            Vec::new(),
            "Maximum preview length stored in structured preview reports.",
        ),
        param_flag(
            CliParameterSection::InspectionOutput,
            CliParameterId::IncludeSourceText,
            "Include the full source text inside structured inspection reports.",
        ),
        param_option(
            CliParameterSection::InspectionOutput,
            CliParameterId::OutputFile,
            CliParameterRequirement::Optional,
            "PATH",
            None,
            Vec::new(),
            "Write the stdout payload to exactly one file instead of stdout.",
        ),
    ]
}

fn inspect_source_parameters() -> Vec<CliParameterDescriptor> {
    let output_modes = inspect_output_modes();
    let mut parameters = common_source_parameters(CliParameterRequirement::Required);
    parameters.push(param_option(
        CliParameterSection::Source,
        CliParameterId::SampleLimit,
        CliParameterRequirement::Optional,
        "SAMPLE_LIMIT",
        Some(CliValue::Usize(DEFAULT_INSPECTION_SAMPLE_LIMIT)),
        Vec::new(),
        "Maximum number of headings, links, tags, and classes to sample in the summary.",
    ));
    parameters.push(param_option(
        CliParameterSection::Source,
        CliParameterId::Output,
        CliParameterRequirement::Optional,
        "OUTPUT",
        Some(CliValue::OutputMode(CliOutputMode::Json)),
        output_mode_values(&output_modes),
        "Render the inspection as compact text or structured JSON.",
    ));
    parameters.push(param_flag(
        CliParameterSection::Source,
        CliParameterId::IncludeSourceText,
        "Include the full source text in JSON output and a bounded preview in text output.",
    ));
    parameters.push(param_option(
        CliParameterSection::Source,
        CliParameterId::PreviewChars,
        CliParameterRequirement::Optional,
        "PREVIEW_CHARS",
        Some(CliValue::Usize(DEFAULT_PREVIEW_CHARS)),
        Vec::new(),
        "Maximum length of the source preview shown in text mode when --include-source-text is used.",
    ));
    parameters.push(param_option(
        CliParameterSection::Source,
        CliParameterId::OutputFile,
        CliParameterRequirement::Optional,
        "PATH",
        None,
        Vec::new(),
        "Write the stdout payload to exactly one file instead of stdout.",
    ));
    parameters
}

fn inspect_select_parameters() -> Vec<CliParameterDescriptor> {
    let mut parameters = common_definition_parameters();
    parameters.extend(request_file_aware_source_parameters());
    parameters.push(param_option(
        CliParameterSection::Source,
        CliParameterId::Css,
        CliParameterRequirement::RequiredUnless(CliParameterId::RequestFile),
        "CSS",
        None,
        Vec::new(),
        "CSS selector that chooses the candidate nodes to preview.",
    ));
    parameters.extend(common_selection_parameters());
    parameters.push(param_option(
        CliParameterSection::Selection,
        CliParameterId::Whitespace,
        CliParameterRequirement::Optional,
        "WHITESPACE",
        Some(CliValue::WhitespaceMode(WhitespaceMode::Preserve)),
        whitespace_values(),
        "Preserve source whitespace or normalize preview text.",
    ));
    parameters.push(param_flag(
        CliParameterSection::Selection,
        CliParameterId::RewriteUrls,
        "Rewrite relative URLs in preview HTML and attribute data with the effective base URL.",
    ));
    parameters.extend(common_inspect_output_parameters());
    parameters
}

fn inspect_slice_parameters() -> Vec<CliParameterDescriptor> {
    let mut parameters = common_definition_parameters();
    parameters.extend(request_file_aware_source_parameters());
    parameters.extend(slice_strategy_parameters(CliParameterSection::Source));
    parameters.extend(common_selection_parameters());
    parameters.push(param_option(
        CliParameterSection::Selection,
        CliParameterId::Whitespace,
        CliParameterRequirement::Optional,
        "WHITESPACE",
        Some(CliValue::WhitespaceMode(WhitespaceMode::Preserve)),
        whitespace_values(),
        "Preserve source whitespace or normalize preview text.",
    ));
    parameters.push(param_flag(
        CliParameterSection::Selection,
        CliParameterId::RewriteUrls,
        "Rewrite relative URLs in preview HTML and attribute data with the effective base URL.",
    ));
    parameters.extend(common_inspect_output_parameters());
    parameters
}

fn select_extract_parameters() -> Vec<CliParameterDescriptor> {
    let mut parameters = common_definition_parameters();
    parameters.extend(request_file_aware_source_parameters());
    parameters.push(param_option(
        CliParameterSection::Source,
        CliParameterId::Css,
        CliParameterRequirement::RequiredUnless(CliParameterId::RequestFile),
        "CSS",
        None,
        Vec::new(),
        "CSS selector that chooses the candidate nodes to extract.",
    ));
    parameters.extend(common_selection_parameters());
    parameters.extend(common_extract_parameters());
    parameters
}

fn slice_extract_parameters() -> Vec<CliParameterDescriptor> {
    let mut parameters = common_definition_parameters();
    parameters.extend(request_file_aware_source_parameters());
    parameters.extend(slice_strategy_parameters(CliParameterSection::Source));
    parameters.extend(common_selection_parameters());
    parameters.extend(common_extract_parameters());
    parameters
}

fn slice_strategy_parameters(section: CliParameterSection) -> Vec<CliParameterDescriptor> {
    vec![
        param_option(
            section,
            CliParameterId::From,
            CliParameterRequirement::RequiredUnless(CliParameterId::RequestFile),
            "FROM",
            None,
            Vec::new(),
            "Start boundary used to locate each candidate slice.",
        ),
        param_option(
            section,
            CliParameterId::To,
            CliParameterRequirement::RequiredUnless(CliParameterId::RequestFile),
            "TO",
            None,
            Vec::new(),
            "End boundary used to locate each candidate slice.",
        ),
        param_option(
            section,
            CliParameterId::Pattern,
            CliParameterRequirement::Optional,
            "PATTERN",
            Some(CliValue::PatternMode(PatternMode::Literal)),
            pattern_values(),
            "Interpret --from and --to as literal text or regex patterns.",
        ),
        param_option(
            section,
            CliParameterId::RegexFlags,
            CliParameterRequirement::AllowedOnlyWhen(condition(
                CliParameterId::Pattern,
                vec![CliValue::PatternMode(PatternMode::Regex)],
            )),
            "REGEX_FLAGS",
            None,
            Vec::new(),
            "Regex flags for --pattern regex. Accepts i, m, s, u, and x.",
        ),
        param_flag(
            section,
            CliParameterId::IncludeStart,
            "Include the matched --from boundary in the selected fragment.",
        ),
        param_flag(
            section,
            CliParameterId::IncludeEnd,
            "Include the matched --to boundary in the selected fragment.",
        ),
    ]
}

fn build_source_inspect_contract() -> OperationCliContract {
    let parameters = inspect_source_parameters();
    let output_modes = inspect_output_modes();
    OperationCliContract {
        operation_id: OperationId::SourceInspect,
        command_path: &["inspect", "source"],
        invocation: "htmlcut inspect source [OPTIONS] [INPUT]",
        inputs: common_input_forms(),
        default_match: None,
        selection_modes: Vec::new(),
        default_value: None,
        value_modes: Vec::new(),
        default_output: Some(CliOutputMode::Json),
        default_output_overrides: Vec::new(),
        output_modes: output_modes.clone(),
        constraints: constraints_with_parameter_rules(&parameters, Vec::new()),
        notes: vec![
            "Use this command to inspect document shape, headings, links, classes, and effective base-URL behavior before choosing selectors or slice boundaries.",
            "--include-source-text stores the full source in JSON output and prints a bounded source preview in text mode.",
            "--sample-limit bounds the sampled headings, links, tags, and classes in the summary.",
        ],
        examples: vec![
            "htmlcut inspect source ./page.html",
            "htmlcut inspect source ./page.html --output text --include-source-text --preview-chars 200",
        ],
        parameters,
    }
}

fn build_select_preview_contract() -> OperationCliContract {
    let parameters = inspect_select_parameters();
    let selection_modes = common_selection_modes();
    let output_modes = inspect_output_modes();
    OperationCliContract {
        operation_id: OperationId::SelectPreview,
        command_path: &["inspect", "select"],
        invocation: "htmlcut inspect select [OPTIONS] --css <CSS> [INPUT]",
        inputs: common_input_forms(),
        default_match: Some(CliSelectionMode::First),
        selection_modes: selection_modes.clone(),
        default_value: Some(ValueType::Structured),
        value_modes: vec![ValueType::Structured],
        default_output: Some(CliOutputMode::Json),
        default_output_overrides: Vec::new(),
        output_modes: output_modes.clone(),
        constraints: constraints_with_parameter_rules(&parameters, Vec::new()),
        notes: vec![
            "inspect select always previews matches in structured form; it is a preview workflow, not a final extraction surface.",
            "When --rewrite-urls is requested but no effective base URL can be resolved, relative URLs stay unchanged and a warning is emitted.",
            "Use --emit-request-file to capture the canonical preview definition while you iterate on inline flags.",
        ],
        examples: vec![
            "htmlcut inspect select ./page.html --css article --match single",
            "htmlcut inspect select ./page.html --css '.card' --match all --output text",
            "htmlcut inspect select ./page.html --css article --emit-request-file ./article-preview.json",
            "htmlcut inspect select --request-file ./article-preview.json",
        ],
        parameters,
    }
}

fn build_slice_preview_contract() -> OperationCliContract {
    let parameters = inspect_slice_parameters();
    let selection_modes = common_selection_modes();
    let output_modes = inspect_output_modes();
    OperationCliContract {
        operation_id: OperationId::SlicePreview,
        command_path: &["inspect", "slice"],
        invocation: "htmlcut inspect slice [OPTIONS] --from <FROM> --to <TO> [INPUT]",
        inputs: common_input_forms(),
        default_match: Some(CliSelectionMode::First),
        selection_modes: selection_modes.clone(),
        default_value: Some(ValueType::Structured),
        value_modes: vec![ValueType::Structured],
        default_output: Some(CliOutputMode::Json),
        default_output_overrides: Vec::new(),
        output_modes: output_modes.clone(),
        constraints: constraints_with_parameter_rules(&parameters, Vec::new()),
        notes: vec![
            "Literal boundaries are raw substring matching, not tag-aware; `<a` also matches `<article>`.",
            "Previews exclude both matched boundaries by default unless --include-start and/or --include-end are supplied.",
            "Text output shows fragment context when it adds signal so boundary-consumption mistakes are easier to spot.",
            "Use --emit-request-file to capture the canonical preview definition while you iterate on inline flags.",
        ],
        examples: vec![
            "htmlcut inspect slice ./page.html --from '<article>' --to '</article>'",
            "htmlcut inspect slice ./page.html --from 'START::' --to '::END' --pattern regex --output text",
            "htmlcut inspect slice ./page.html --from '<article>' --to '</article>' --emit-request-file ./article-slice-preview.json",
            "htmlcut inspect slice --request-file ./article-slice-preview.json",
        ],
        parameters,
    }
}

fn build_select_extract_contract() -> OperationCliContract {
    let parameters = select_extract_parameters();
    let selection_modes = common_selection_modes();
    let value_modes = extract_value_modes();
    let output_modes = extract_output_modes();
    OperationCliContract {
        operation_id: OperationId::SelectExtract,
        command_path: &["select"],
        invocation: "htmlcut select [OPTIONS] --css <CSS> [INPUT]",
        inputs: common_input_forms(),
        default_match: Some(CliSelectionMode::First),
        selection_modes: selection_modes.clone(),
        default_value: Some(ValueType::Text),
        value_modes: value_modes.clone(),
        default_output: Some(CliOutputMode::Text),
        default_output_overrides: vec![conditional_default(
            CliValue::OutputMode(CliOutputMode::Json),
            condition(
                CliParameterId::Value,
                vec![CliValue::ValueType(ValueType::Structured)],
            ),
        )],
        output_modes: output_modes.clone(),
        constraints: constraints_with_parameter_rules(
            &parameters,
            vec![
                requires_parameter(
                    CliParameterId::Bundle,
                    condition(
                        CliParameterId::Output,
                        vec![CliValue::OutputMode(CliOutputMode::None)],
                    ),
                ),
                restricts_parameter_values(
                    CliParameterId::Output,
                    vec![
                        CliValue::OutputMode(CliOutputMode::Json),
                        CliValue::OutputMode(CliOutputMode::None),
                    ],
                    condition(
                        CliParameterId::Value,
                        vec![CliValue::ValueType(ValueType::Structured)],
                    ),
                ),
                restricts_parameter_values(
                    CliParameterId::Value,
                    vec![
                        CliValue::ValueType(ValueType::InnerHtml),
                        CliValue::ValueType(ValueType::OuterHtml),
                    ],
                    condition(
                        CliParameterId::Output,
                        vec![CliValue::OutputMode(CliOutputMode::Html)],
                    ),
                ),
            ],
        ),
        notes: vec![
            "Structured extraction only supports --output json or --output none.",
            "--output html is only valid with --value inner-html or --value outer-html.",
            "When --rewrite-urls is requested but no effective base URL can be resolved, relative URLs stay unchanged and a warning is emitted.",
            "Use --emit-request-file to capture the canonical extraction definition while you iterate on inline flags.",
        ],
        examples: vec![
            "htmlcut select ./page.html --css article --match single",
            "htmlcut select ./page.html --css '.card' --match all --value outer-html",
            "htmlcut select ./page.html --css 'article a.more' --value attribute --attribute href --rewrite-urls",
            "htmlcut select ./page.html --css 'article a.more' --value attribute --attribute href --emit-request-file ./article-links.json",
            "htmlcut select --request-file ./article-links.json --output-file ./links.json",
        ],
        parameters,
    }
}

fn build_slice_extract_contract() -> OperationCliContract {
    let parameters = slice_extract_parameters();
    let selection_modes = common_selection_modes();
    let value_modes = extract_value_modes();
    let output_modes = extract_output_modes();
    OperationCliContract {
        operation_id: OperationId::SliceExtract,
        command_path: &["slice"],
        invocation: "htmlcut slice [OPTIONS] --from <FROM> --to <TO> [INPUT]",
        inputs: common_input_forms(),
        default_match: Some(CliSelectionMode::First),
        selection_modes: selection_modes.clone(),
        default_value: Some(ValueType::Text),
        value_modes: value_modes.clone(),
        default_output: Some(CliOutputMode::Text),
        default_output_overrides: vec![conditional_default(
            CliValue::OutputMode(CliOutputMode::Json),
            condition(
                CliParameterId::Value,
                vec![CliValue::ValueType(ValueType::Structured)],
            ),
        )],
        output_modes: output_modes.clone(),
        constraints: constraints_with_parameter_rules(
            &parameters,
            vec![
                requires_parameter(
                    CliParameterId::Bundle,
                    condition(
                        CliParameterId::Output,
                        vec![CliValue::OutputMode(CliOutputMode::None)],
                    ),
                ),
                restricts_parameter_values(
                    CliParameterId::Output,
                    vec![
                        CliValue::OutputMode(CliOutputMode::Json),
                        CliValue::OutputMode(CliOutputMode::None),
                    ],
                    condition(
                        CliParameterId::Value,
                        vec![CliValue::ValueType(ValueType::Structured)],
                    ),
                ),
                restricts_parameter_values(
                    CliParameterId::Value,
                    vec![
                        CliValue::ValueType(ValueType::InnerHtml),
                        CliValue::ValueType(ValueType::OuterHtml),
                    ],
                    condition(
                        CliParameterId::Output,
                        vec![CliValue::OutputMode(CliOutputMode::Html)],
                    ),
                ),
            ],
        ),
        notes: vec![
            "Literal boundaries are raw substring matching, not tag-aware; `<a` also matches `<article>`.",
            "The selected fragment excludes both matched boundaries by default; --include-start and --include-end control that selected fragment precisely.",
            "For --value inner-html, HTMLCut returns the selected fragment as HTML. For --value outer-html, HTMLCut returns the full outer matched range including both boundaries.",
            "When extracting --value attribute from sliced HTML, use --include-start when the opening tag lives in the start boundary.",
            "Structured extraction only supports --output json or --output none.",
            "Use --emit-request-file to capture the canonical extraction definition while you iterate on inline flags.",
        ],
        examples: vec![
            "htmlcut slice ./page.html --from '<article>' --to '</article>'",
            "htmlcut slice ./page.html --from 'START::' --to '::END' --pattern regex --match all --output json",
            "htmlcut slice ./page.html --from '<a ' --to '</a>' --include-start --include-end --value attribute --attribute href",
            "htmlcut slice ./page.html --from '<article>' --to '</article>' --emit-request-file ./article-slice.json",
            "htmlcut slice --request-file ./article-slice.json --output-file ./fragment.html",
        ],
        parameters,
    }
}
