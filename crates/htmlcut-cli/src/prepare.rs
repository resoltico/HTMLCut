use std::num::NonZeroUsize;
use std::path::PathBuf;

mod build;
mod definition;
mod reports;

use htmlcut_core::{
    DEFAULT_REGEX_FLAGS, ExtractionDefinition, ExtractionRequest, ExtractionStrategy,
    InspectionOptions, RuntimeOptions, SourceRequest, ValueSpec,
};

use crate::args::{
    CliOutputMode, CliWhitespaceMode, InspectSelectArgs, InspectSliceArgs, InspectSourceArgs,
    SelectArgs, SliceArgs,
};
use crate::error::{CliError, usage_error};

pub(crate) use self::build::extract_prefers_json;
pub(crate) use self::build::{
    StrategyArgs, build_extraction_request, build_runtime, build_source_request,
    resolve_extract_output_mode_with_output_file, resolve_value_spec, validate_preview_chars,
};
#[cfg(test)]
pub(crate) use self::build::{
    default_output_for_value, parse_byte_size, resolve_extract_output_mode, resolve_regex_flags,
    resolve_selection_spec, validate_base_url,
};
use self::definition::{
    ensure_inline_inspect_select_request_is_default,
    ensure_inline_inspect_slice_request_is_default, ensure_inline_select_request_is_default,
    ensure_inline_slice_request_is_default, materialize_extraction_definition,
};
#[cfg(test)]
pub(crate) use self::definition::{
    format_json_error_path_for_tests, load_extraction_definition_for_tests,
};
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

impl PreparedExtraction {
    #[cfg(test)]
    pub(crate) fn from_select(args: SelectArgs) -> Result<Self, CliError> {
        Self::from_select_with_logging(args, 0, false)
    }

    pub(crate) fn from_select_with_logging(
        args: SelectArgs,
        verbose: u8,
        quiet: bool,
    ) -> Result<Self, CliError> {
        let display_command =
            htmlcut_core::cli_operation_display_command(htmlcut_core::OperationId::SelectExtract)
                .expect("select extract should stay CLI-visible");
        if args.definition.request_file.is_some() {
            ensure_inline_select_request_is_default(&args)?;
        }
        let materialized = materialize_extraction_definition(
            &args.definition,
            ExtractionStrategy::Selector,
            &display_command,
            htmlcut_core::OperationId::SelectExtract,
            || {
                let value = resolve_value_spec(args.output.value, args.output.attribute.clone())?;
                let preview_chars = validate_preview_chars(args.output.preview_chars)?;
                let strategy_args = StrategyArgs::Select {
                    css: required_cli_value(args.css, "--css")?,
                };
                let options = RequestBuildOptions {
                    value,
                    whitespace: args.output.whitespace,
                    rewrite_urls: args.output.rewrite_urls,
                    preview_chars,
                    include_source_text: args.output.include_source_text,
                };
                Ok((
                    build_extraction_request(
                        strategy_args,
                        &args.source,
                        &args.selection,
                        options,
                    )?,
                    build_runtime(&args.source)?,
                ))
            },
        )?;
        let request = materialized.request;
        let runtime = materialized.runtime;
        let value_type = request.extraction.value().value_type();
        let output = resolve_extract_output_mode_with_output_file(
            args.output.output,
            &value_type,
            args.output.bundle.as_deref(),
            args.output.output_file.as_deref(),
        )?;

        Ok(Self {
            command: htmlcut_core::cli_operation_report_command(
                htmlcut_core::OperationId::SelectExtract,
            )
            .expect("select extract should stay CLI-visible"),
            runtime,
            request,
            request_definition_output: materialized.request_definition_output,
            output,
            bundle: args.output.bundle,
            output_file: args.output.output_file,
            verbose,
            quiet,
        })
    }

    #[cfg(test)]
    pub(crate) fn from_slice(args: SliceArgs) -> Result<Self, CliError> {
        Self::from_slice_with_logging(args, 0, false)
    }

    pub(crate) fn from_slice_with_logging(
        args: SliceArgs,
        verbose: u8,
        quiet: bool,
    ) -> Result<Self, CliError> {
        let display_command =
            htmlcut_core::cli_operation_display_command(htmlcut_core::OperationId::SliceExtract)
                .expect("slice extract should stay CLI-visible");
        if args.definition.request_file.is_some() {
            ensure_inline_slice_request_is_default(&args)?;
        }
        let materialized = materialize_extraction_definition(
            &args.definition,
            ExtractionStrategy::Slice,
            &display_command,
            htmlcut_core::OperationId::SliceExtract,
            || {
                let value = resolve_value_spec(args.output.value, args.output.attribute.clone())?;
                let preview_chars = validate_preview_chars(args.output.preview_chars)?;
                let strategy_args = StrategyArgs::Slice {
                    from: required_cli_value(args.from, "--from")?,
                    to: required_cli_value(args.to, "--to")?,
                    pattern: args.pattern,
                    regex_flags: args.regex_flags,
                    include_start: args.include_start,
                    include_end: args.include_end,
                };
                let options = RequestBuildOptions {
                    value,
                    whitespace: args.output.whitespace,
                    rewrite_urls: args.output.rewrite_urls,
                    preview_chars,
                    include_source_text: args.output.include_source_text,
                };
                Ok((
                    build_extraction_request(
                        strategy_args,
                        &args.source,
                        &args.selection,
                        options,
                    )?,
                    build_runtime(&args.source)?,
                ))
            },
        )?;
        let request = materialized.request;
        let runtime = materialized.runtime;
        let value_type = request.extraction.value().value_type();
        let output = resolve_extract_output_mode_with_output_file(
            args.output.output,
            &value_type,
            args.output.bundle.as_deref(),
            args.output.output_file.as_deref(),
        )?;

        Ok(Self {
            command: htmlcut_core::cli_operation_report_command(
                htmlcut_core::OperationId::SliceExtract,
            )
            .expect("slice extract should stay CLI-visible"),
            runtime,
            request,
            request_definition_output: materialized.request_definition_output,
            output,
            bundle: args.output.bundle,
            output_file: args.output.output_file,
            verbose,
            quiet,
        })
    }
}

impl PreparedSourceInspection {
    #[cfg(test)]
    pub(crate) fn new(args: InspectSourceArgs) -> Result<Self, CliError> {
        Self::new_with_logging(args, 0, false)
    }

    pub(crate) fn new_with_logging(
        args: InspectSourceArgs,
        verbose: u8,
        quiet: bool,
    ) -> Result<Self, CliError> {
        let preview_chars = validate_preview_chars(args.preview_chars)?;
        let runtime = build_runtime(&args.source)?;
        let source = build_source_request(&args.source)?;
        Ok(Self {
            command: htmlcut_core::cli_operation_report_command(
                htmlcut_core::OperationId::SourceInspect,
            )
            .expect("source inspect should stay CLI-visible"),
            source,
            runtime,
            output: args.output,
            preview_chars: preview_chars.get(),
            output_file: args.output_file,
            verbose,
            quiet,
            options: InspectionOptions {
                include_source_text: args.include_source_text,
                sample_limit: args.sample_limit,
            },
        })
    }
}

impl PreparedPreview {
    #[cfg(test)]
    pub(crate) fn from_select(args: InspectSelectArgs) -> Result<Self, CliError> {
        Self::from_select_with_logging(args, 0, false)
    }

    pub(crate) fn from_select_with_logging(
        args: InspectSelectArgs,
        verbose: u8,
        quiet: bool,
    ) -> Result<Self, CliError> {
        let display_command =
            htmlcut_core::cli_operation_display_command(htmlcut_core::OperationId::SelectPreview)
                .expect("select preview should stay CLI-visible");
        if args.definition.request_file.is_some() {
            ensure_inline_inspect_select_request_is_default(&args)?;
        }
        let materialized = materialize_extraction_definition(
            &args.definition,
            ExtractionStrategy::Selector,
            &display_command,
            htmlcut_core::OperationId::SelectPreview,
            || {
                let preview_chars = validate_preview_chars(args.output.preview_chars)?;
                Ok((
                    build_extraction_request(
                        StrategyArgs::Select {
                            css: required_cli_value(args.css, "--css")?,
                        },
                        &args.source,
                        &args.selection,
                        RequestBuildOptions {
                            value: ValueSpec::Structured,
                            whitespace: args.whitespace,
                            rewrite_urls: args.rewrite_urls,
                            preview_chars,
                            include_source_text: args.output.include_source_text,
                        },
                    )?,
                    build_runtime(&args.source)?,
                ))
            },
        )?;
        let mut request = materialized.request;
        let runtime = materialized.runtime;
        request.extraction = request.extraction.clone().with_value(ValueSpec::Structured);
        Ok(Self {
            command: htmlcut_core::cli_operation_report_command(
                htmlcut_core::OperationId::SelectPreview,
            )
            .expect("select preview should stay CLI-visible"),
            runtime,
            request,
            request_definition_output: materialized.request_definition_output,
            output: args.output.output,
            output_file: args.output.output_file,
            verbose,
            quiet,
        })
    }

    #[cfg(test)]
    pub(crate) fn from_slice(args: InspectSliceArgs) -> Result<Self, CliError> {
        Self::from_slice_with_logging(args, 0, false)
    }

    pub(crate) fn from_slice_with_logging(
        args: InspectSliceArgs,
        verbose: u8,
        quiet: bool,
    ) -> Result<Self, CliError> {
        let display_command =
            htmlcut_core::cli_operation_display_command(htmlcut_core::OperationId::SlicePreview)
                .expect("slice preview should stay CLI-visible");
        if args.definition.request_file.is_some() {
            ensure_inline_inspect_slice_request_is_default(&args)?;
        }
        let materialized = materialize_extraction_definition(
            &args.definition,
            ExtractionStrategy::Slice,
            &display_command,
            htmlcut_core::OperationId::SlicePreview,
            || {
                let preview_chars = validate_preview_chars(args.output.preview_chars)?;
                Ok((
                    build_extraction_request(
                        StrategyArgs::Slice {
                            from: required_cli_value(args.from, "--from")?,
                            to: required_cli_value(args.to, "--to")?,
                            pattern: args.pattern,
                            regex_flags: args.regex_flags,
                            include_start: args.include_start,
                            include_end: args.include_end,
                        },
                        &args.source,
                        &args.selection,
                        RequestBuildOptions {
                            value: ValueSpec::Structured,
                            whitespace: args.whitespace,
                            rewrite_urls: args.rewrite_urls,
                            preview_chars,
                            include_source_text: args.output.include_source_text,
                        },
                    )?,
                    build_runtime(&args.source)?,
                ))
            },
        )?;
        let mut request = materialized.request;
        request.extraction = request.extraction.clone().with_value(ValueSpec::Structured);
        Ok(Self {
            command: htmlcut_core::cli_operation_report_command(
                htmlcut_core::OperationId::SlicePreview,
            )
            .expect("slice preview should stay CLI-visible"),
            runtime: materialized.runtime,
            request,
            request_definition_output: materialized.request_definition_output,
            output: args.output.output,
            output_file: args.output.output_file,
            verbose,
            quiet,
        })
    }
}

fn required_cli_value(value: Option<String>, parameter: &'static str) -> Result<String, CliError> {
    value.ok_or_else(|| {
        usage_error(
            "CLI_REQUIRED_PARAMETER_MISSING",
            format!("{parameter} is required unless --request-file is used."),
        )
    })
}

pub(crate) fn default_regex_flags() -> String {
    DEFAULT_REGEX_FLAGS.to_owned()
}
