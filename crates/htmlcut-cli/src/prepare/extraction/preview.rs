use htmlcut_core::{ExtractionStrategy, ValueSpec};

use crate::args::{InspectSelectArgs, InspectSliceArgs};
use crate::error::CliError;
use crate::prepare::build::{
    StrategyArgs, build_extraction_request, build_runtime, validate_preview_chars,
};
use crate::prepare::definition::{
    ensure_inline_inspect_select_request_is_default, ensure_inline_inspect_slice_request_is_default,
};
use crate::prepare::{PreparedPreview, RequestBuildOptions, required_cli_value};

use super::materialize_operation_definition;

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
        if args.definition.request_file.is_some() {
            ensure_inline_inspect_select_request_is_default(&args)?;
        }
        let (commands, materialized) = materialize_operation_definition(
            &args.definition,
            ExtractionStrategy::Selector,
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
        request.extraction = request.extraction.clone().with_value(ValueSpec::Structured);
        Ok(Self {
            command: commands.report,
            runtime: materialized.runtime,
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
        if args.definition.request_file.is_some() {
            ensure_inline_inspect_slice_request_is_default(&args)?;
        }
        let (commands, materialized) = materialize_operation_definition(
            &args.definition,
            ExtractionStrategy::Slice,
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
            command: commands.report,
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
