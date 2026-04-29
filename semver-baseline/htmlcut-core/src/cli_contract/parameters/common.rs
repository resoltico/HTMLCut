use crate::contracts::{
    DEFAULT_FETCH_CONNECT_TIMEOUT_MS, DEFAULT_FETCH_TIMEOUT_MS, DEFAULT_MAX_BYTES,
    DEFAULT_PREVIEW_CHARS, FetchPreflightMode, ValueType, WhitespaceMode,
};

use super::descriptors::{param_flag, param_option, param_positional};
use super::{
    CliInputForm, CliOutputMode, CliParameterDescriptor, CliParameterId, CliParameterRequirement,
    CliParameterSection, CliSelectionMode, CliValue, condition, fetch_preflight_values,
    output_mode_values, selection_mode_values, value_type_values, whitespace_values,
};

pub(super) fn common_input_forms() -> Vec<CliInputForm> {
    vec![
        CliInputForm::LocalFilePath,
        CliInputForm::Url,
        CliInputForm::Stdin,
    ]
}

pub(super) fn common_selection_modes() -> Vec<CliSelectionMode> {
    vec![
        CliSelectionMode::Single,
        CliSelectionMode::First,
        CliSelectionMode::Nth,
        CliSelectionMode::All,
    ]
}

pub(super) fn inspect_output_modes() -> Vec<CliOutputMode> {
    vec![CliOutputMode::Text, CliOutputMode::Json]
}

pub(super) fn extract_output_modes() -> Vec<CliOutputMode> {
    vec![
        CliOutputMode::Text,
        CliOutputMode::Html,
        CliOutputMode::Json,
        CliOutputMode::None,
    ]
}

pub(super) fn extract_value_modes() -> Vec<ValueType> {
    vec![
        ValueType::Text,
        ValueType::InnerHtml,
        ValueType::OuterHtml,
        ValueType::Attribute,
        ValueType::Structured,
    ]
}

pub(super) fn common_source_parameters(
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
            "Refuse sources larger than this limit. Accepts raw bytes or KiB, MiB, and GiB when the final byte count is a whole positive number.",
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
            CliParameterId::FetchConnectTimeoutMs,
            CliParameterRequirement::Optional,
            "MILLISECONDS",
            Some(CliValue::U64(DEFAULT_FETCH_CONNECT_TIMEOUT_MS)),
            Vec::new(),
            "HTTP connect timeout in milliseconds for URL inputs.",
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

pub(super) fn common_definition_parameters() -> Vec<CliParameterDescriptor> {
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

pub(super) fn request_file_aware_source_parameters() -> Vec<CliParameterDescriptor> {
    common_source_parameters(CliParameterRequirement::RequiredUnless(
        CliParameterId::RequestFile,
    ))
}

pub(super) fn common_selection_parameters() -> Vec<CliParameterDescriptor> {
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

pub(super) fn common_extract_parameters() -> Vec<CliParameterDescriptor> {
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

pub(super) fn common_inspect_output_parameters() -> Vec<CliParameterDescriptor> {
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
