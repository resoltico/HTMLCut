use htmlcut_core::{
    DEFAULT_FETCH_CONNECT_TIMEOUT_MS, DEFAULT_FETCH_TIMEOUT_MS, DEFAULT_MAX_BYTES,
    DEFAULT_PREVIEW_CHARS, FetchPreflightMode, ValueType, WhitespaceMode,
};

use super::descriptors::{param_flag, param_option, param_positional};
use super::{
    CliInputForm, CliOutputMode, CliParameterDescriptor, CliParameterId, CliParameterRequirement,
    CliParameterSection, CliSelectionMode, CliValue, condition, fetch_preflight_values,
    output_mode_values, selection_mode_values, tls_trust_values, value_type_values,
    whitespace_values,
};

pub(in crate::contract) fn common_input_forms() -> Vec<CliInputForm> {
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

pub(super) fn select_extract_value_modes() -> Vec<ValueType> {
    vec![
        ValueType::Text,
        ValueType::InnerHtml,
        ValueType::OuterHtml,
        ValueType::Attribute,
        ValueType::Structured,
    ]
}

pub(super) fn slice_extract_value_modes() -> Vec<ValueType> {
    vec![
        ValueType::Text,
        ValueType::SelectedHtml,
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
            "Use successful HEAD responses as advisory validation before GET. HTMLCut falls back only when HEAD rejects the method or fails in a way that indicates HEAD intolerance; get-only skips the probe.",
        ),
        param_option(
            CliParameterSection::Source,
            CliParameterId::TlsTrust,
            CliParameterRequirement::Optional,
            "TLS_TRUST",
            Some(CliValue::TlsTrustMode(super::CliTlsTrustMode::WebPki)),
            tls_trust_values(),
            "Choose the trust-root policy for built-in HTTP fetching: bundled WebPKI roots, the host platform verifier, or one explicit PEM CA bundle.",
        ),
        param_option(
            CliParameterSection::Source,
            CliParameterId::TlsCaBundle,
            CliParameterRequirement::Optional,
            "PATH",
            None,
            Vec::new(),
            "PEM CA bundle path required only with --tls-trust custom-ca-bundle.",
        ),
        param_positional(
            CliParameterSection::Source,
            CliParameterId::Input,
            input_requirement,
            "HTML input source: a local file path, an http(s) URL, or - for explicit stdin. Omitted INPUT never implicitly consumes piped stdin.",
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

pub(super) fn output_file_filesystem_output_parameters() -> Vec<CliParameterDescriptor> {
    vec![param_flag(
        CliParameterSection::FilesystemOutput,
        CliParameterId::Overwrite,
        "Allow HTMLCut to replace an existing --output-file.",
    )]
}

pub(super) fn preview_filesystem_output_parameters() -> Vec<CliParameterDescriptor> {
    vec![param_flag(
        CliParameterSection::FilesystemOutput,
        CliParameterId::Overwrite,
        "Allow HTMLCut to replace existing --output-file and --emit-request-file paths.",
    )]
}

pub(super) fn extraction_filesystem_output_parameters() -> Vec<CliParameterDescriptor> {
    vec![param_flag(
        CliParameterSection::FilesystemOutput,
        CliParameterId::Overwrite,
        "Allow HTMLCut to replace existing --output-file, --emit-request-file, and --bundle paths.",
    )]
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

pub(super) fn common_extract_parameters(value_modes: &[ValueType]) -> Vec<CliParameterDescriptor> {
    let output_modes = extract_output_modes();
    vec![
        param_option(
            CliParameterSection::Extraction,
            CliParameterId::Value,
            CliParameterRequirement::Optional,
            "VALUE",
            Some(CliValue::ValueType(ValueType::Text)),
            value_type_values(value_modes),
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
            Some(CliValue::WhitespaceMode(WhitespaceMode::Rendered)),
            whitespace_values(),
            "Preserve semantic rendered layout, or normalize whitespace within that HTML-aware rendering without flattening headings, lists, tables, or link annotations.",
        ),
        param_flag(
            CliParameterSection::Extraction,
            CliParameterId::RewriteUrls,
            "Rewrite supported relative URLs in extracted HTML with the effective base URL, including standard HTML URL-bearing attributes plus CSS url(...) and quoted @import references. Plain-text rendering resolves displayed link destinations against the effective base whenever one is known, and --rewrite-urls controls the saved HTML fragment itself.",
        ),
        param_option(
            CliParameterSection::Extraction,
            CliParameterId::Output,
            CliParameterRequirement::Optional,
            "OUTPUT",
            None,
            output_mode_values(&output_modes),
            "How stdout should be rendered after extraction. HTML output emits fragments, not a standalone document; multiple fragments are separated by blank lines. JSON mode writes either a success or failure document to stdout, and the process exit status remains authoritative. When OUTPUT is `none`, --bundle is required because no stdout payload is produced.",
        ),
        param_option(
            CliParameterSection::Extraction,
            CliParameterId::Bundle,
            CliParameterRequirement::Optional,
            "BUNDLE",
            None,
            Vec::new(),
            "Write selection.json, selection.html, selection.txt, and report.json to this directory. selection.json keeps the canonical extracted values and metadata, selection.txt always contains rendered plain text, even when the extracted value is HTML, and report.json is the lightweight execution summary for the same bundle run.",
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
            "Render the inspection as compact text or structured JSON. JSON mode writes either a success or failure document to stdout, and the process exit status remains authoritative.",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filesystem_output_surfaces_require_the_shared_overwrite_flag() {
        for parameters in [
            output_file_filesystem_output_parameters(),
            preview_filesystem_output_parameters(),
            extraction_filesystem_output_parameters(),
        ] {
            assert_eq!(parameters.len(), 1);
            assert_eq!(parameters[0].id, CliParameterId::Overwrite);
            assert_eq!(parameters[0].section, CliParameterSection::FilesystemOutput);
        }
    }
}
