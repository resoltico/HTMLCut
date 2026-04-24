use std::num::NonZeroUsize;
use std::path::PathBuf;

mod build;
mod definition;
mod extraction;
mod inspection;
mod reports;

use htmlcut_core::{
    ExtractionDefinition, ExtractionRequest, InspectionOptions, RuntimeOptions, SourceRequest,
    ValueSpec,
};

use crate::args::{CliOutputMode, CliWhitespaceMode};
use crate::error::{CliError, usage_error};

pub(crate) use self::build::extract_prefers_json;
#[cfg(test)]
pub(crate) use self::build::{
    build_runtime, build_source_request, default_output_for_value, parse_byte_size,
    resolve_extract_output_mode, resolve_extract_output_mode_with_output_file, resolve_regex_flags,
    resolve_selection_spec, resolve_value_spec, validate_base_url, validate_preview_chars,
};
#[cfg(test)]
pub(crate) use self::definition::{
    format_json_error_path_for_tests, load_extraction_definition_for_tests,
};
pub(crate) use self::extraction::default_regex_flags;
#[cfg(test)]
pub(crate) use self::inspection::source_inspection_report_command_for_tests;
#[cfg(test)]
pub(crate) use self::reports::render_condition_expression_for_tests;
pub(crate) use self::reports::{
    build_catalog_report, build_extraction_report, build_schema_report,
    build_source_inspection_report,
};

pub(crate) struct PreparedExtraction {
    pub(crate) command: String,
    pub(crate) request: ExtractionRequest,
    pub(crate) runtime: RuntimeOptions,
    pub(crate) request_definition_output: Option<PendingExtractionDefinitionWrite>,
    pub(crate) output: CliOutputMode,
    pub(crate) bundle: Option<PathBuf>,
    pub(crate) output_file: Option<PathBuf>,
    pub(crate) verbose: u8,
    pub(crate) quiet: bool,
}

pub(crate) struct PreparedSourceInspection {
    pub(crate) command: String,
    pub(crate) source: SourceRequest,
    pub(crate) runtime: RuntimeOptions,
    pub(crate) options: InspectionOptions,
    pub(crate) output: crate::args::CliInspectOutputMode,
    pub(crate) preview_chars: usize,
    pub(crate) output_file: Option<PathBuf>,
    pub(crate) verbose: u8,
    pub(crate) quiet: bool,
}

pub(crate) struct PreparedPreview {
    pub(crate) command: String,
    pub(crate) request: ExtractionRequest,
    pub(crate) runtime: RuntimeOptions,
    pub(crate) request_definition_output: Option<PendingExtractionDefinitionWrite>,
    pub(crate) output: crate::args::CliInspectOutputMode,
    pub(crate) output_file: Option<PathBuf>,
    pub(crate) verbose: u8,
    pub(crate) quiet: bool,
}

pub(crate) struct RequestBuildOptions {
    pub(crate) value: ValueSpec,
    pub(crate) whitespace: CliWhitespaceMode,
    pub(crate) rewrite_urls: bool,
    pub(crate) preview_chars: NonZeroUsize,
    pub(crate) include_source_text: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct PendingExtractionDefinitionWrite {
    pub(crate) path: PathBuf,
    pub(crate) definition: ExtractionDefinition,
}

pub(super) struct MaterializedDefinition {
    request: ExtractionRequest,
    runtime: RuntimeOptions,
    request_definition_output: Option<PendingExtractionDefinitionWrite>,
}

fn required_cli_value(value: Option<String>, parameter: &'static str) -> Result<String, CliError> {
    value.ok_or_else(|| {
        usage_error(
            "CLI_REQUIRED_PARAMETER_MISSING",
            format!("{parameter} is required unless --request-file is used."),
        )
    })
}
