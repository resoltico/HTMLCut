use htmlcut_core::InspectionOptions;

use super::PreparedSourceInspection;
use super::build::{build_runtime, build_source_request, validate_preview_chars};
use crate::args::InspectSourceArgs;
use crate::error::CliError;

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
