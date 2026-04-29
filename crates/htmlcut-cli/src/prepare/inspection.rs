use htmlcut_core::InspectionOptions;

use super::PreparedSourceInspection;
use super::build::{build_runtime, build_source_request, validate_preview_chars};
use crate::args::InspectSourceArgs;
use crate::error::{CliError, internal_error};
use crate::model::CliErrorCode;

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
        let report_command = htmlcut_core::cli_contract::cli_operation_report_command(
            htmlcut_core::OperationId::SourceInspect,
        );
        let command = source_inspection_report_command(report_command.as_deref())?;
        Ok(Self {
            command,
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

fn source_inspection_report_command(command: Option<&str>) -> Result<String, CliError> {
    command.map(str::to_owned).ok_or_else(|| {
        internal_error(
            CliErrorCode::ContractMissing,
            "The core-owned CLI contract is missing the report command for inspect source.",
        )
    })
}

#[cfg(test)]
pub(crate) fn source_inspection_report_command_for_tests(
    command: Option<&str>,
) -> Result<String, CliError> {
    source_inspection_report_command(command)
}
