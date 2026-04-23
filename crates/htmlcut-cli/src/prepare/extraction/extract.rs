use htmlcut_core::ExtractionStrategy;

use crate::args::{SelectArgs, SliceArgs};
use crate::error::CliError;
use crate::prepare::build::{
    StrategyArgs, build_extraction_request, build_runtime,
    resolve_extract_output_mode_with_output_file, resolve_value_spec, validate_preview_chars,
};
use crate::prepare::definition::{
    ensure_inline_select_request_is_default, ensure_inline_slice_request_is_default,
};
use crate::prepare::{PreparedExtraction, RequestBuildOptions, required_cli_value};

use super::shared::materialize_operation_definition;

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
        if args.definition.request_file.is_some() {
            ensure_inline_select_request_is_default(&args)?;
        }
        let (commands, materialized) = materialize_operation_definition(
            &args.definition,
            ExtractionStrategy::Selector,
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
            command: commands.report,
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
        if args.definition.request_file.is_some() {
            ensure_inline_slice_request_is_default(&args)?;
        }
        let (commands, materialized) = materialize_operation_definition(
            &args.definition,
            ExtractionStrategy::Slice,
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
            command: commands.report,
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
