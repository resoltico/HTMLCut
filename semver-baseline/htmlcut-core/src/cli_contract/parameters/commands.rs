use crate::contracts::{
    DEFAULT_INSPECTION_SAMPLE_LIMIT, DEFAULT_PREVIEW_CHARS, PatternMode, WhitespaceMode,
};

use super::common::{
    common_definition_parameters, common_extract_parameters, common_inspect_output_parameters,
    common_selection_parameters, common_source_parameters, request_file_aware_source_parameters,
};
use super::descriptors::{param_flag, param_option};
use super::{
    CliParameterDescriptor, CliParameterId, CliParameterRequirement, CliParameterSection, CliValue,
    pattern_values, whitespace_values,
};

pub(super) fn inspect_source_parameters() -> Vec<CliParameterDescriptor> {
    let output_modes = super::common::inspect_output_modes();
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
        Some(CliValue::OutputMode(super::CliOutputMode::Json)),
        super::output_mode_values(&output_modes),
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

pub(super) fn inspect_select_parameters() -> Vec<CliParameterDescriptor> {
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

pub(super) fn inspect_slice_parameters() -> Vec<CliParameterDescriptor> {
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

pub(super) fn select_extract_parameters() -> Vec<CliParameterDescriptor> {
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

pub(super) fn slice_extract_parameters() -> Vec<CliParameterDescriptor> {
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
            CliParameterRequirement::AllowedOnlyWhen(super::condition(
                CliParameterId::Pattern,
                vec![CliValue::PatternMode(PatternMode::Regex)],
            )),
            "REGEX_FLAGS",
            None,
            Vec::new(),
            "Regex flags for --pattern regex. Accepts i, m, s, U, u, and x; g is accepted for compatibility and ignored.",
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
