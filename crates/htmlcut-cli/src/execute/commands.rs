use htmlcut_core::inspect_source;

use super::{
    ExecutionOutcome, error_outcome, execute_extraction, execute_preview, output_file_notice,
};
use crate::args::{
    CatalogArgs, CliCatalogOutputMode, CliInspectOutputMode, CliSchemaOutputMode,
    InspectSelectArgs, InspectSliceArgs, InspectSourceArgs, SchemaArgs, SelectArgs, SliceArgs,
};
use crate::error::{exit_code_for_error, primary_source_inspection_error, with_source_load_steps};
use crate::lookup;
use crate::prepare::{
    PreparedExtraction, PreparedPreview, PreparedSourceInspection, build_catalog_report,
    build_schema_report, build_source_inspection_report, extract_prefers_json,
};
use crate::render::{
    build_source_inspection_verbose_lines, render_catalog_text, render_schema_text,
    render_source_inspection_text, to_pretty_json,
};

pub(crate) fn run_catalog(args: CatalogArgs, verbose: u8, quiet: bool) -> ExecutionOutcome {
    let report = match build_catalog_report(args.operation.as_deref()) {
        Ok(report) => report,
        Err(error) => {
            return error_outcome(
                "catalog".to_owned(),
                args.output == CliCatalogOutputMode::Json,
                args.output_file,
                error,
            );
        }
    };

    let post_write_stderr = output_file_notice(args.output_file.as_deref(), verbose, quiet);
    ExecutionOutcome {
        stdout: Some(match args.output {
            CliCatalogOutputMode::Json => to_pretty_json(&report),
            CliCatalogOutputMode::Text => render_catalog_text(&report),
        }),
        output_file: args.output_file,
        post_write_stderr,
        stderr: Vec::new(),
        exit_code: 0,
    }
}

pub(crate) fn run_schema(args: SchemaArgs, verbose: u8, quiet: bool) -> ExecutionOutcome {
    let report = match build_schema_report(args.name.as_deref(), args.schema_version) {
        Ok(report) => report,
        Err(error) => {
            return error_outcome(
                "schema".to_owned(),
                args.output == CliSchemaOutputMode::Json,
                args.output_file,
                error,
            );
        }
    };

    let post_write_stderr = output_file_notice(args.output_file.as_deref(), verbose, quiet);
    ExecutionOutcome {
        stdout: Some(match args.output {
            CliSchemaOutputMode::Json => to_pretty_json(&report),
            CliSchemaOutputMode::Text => render_schema_text(&report),
        }),
        output_file: args.output_file,
        post_write_stderr,
        stderr: Vec::new(),
        exit_code: 0,
    }
}

pub(crate) fn run_select(args: SelectArgs, verbose: u8, quiet: bool) -> ExecutionOutcome {
    let prefers_json = extract_prefers_json(&args.output);
    let output_file = args.output.output_file.clone();
    let prepared = match PreparedExtraction::from_select_with_logging(args, verbose, quiet) {
        Ok(prepared) => prepared,
        Err(error) => {
            return operation_error_outcome(
                htmlcut_core::OperationId::SelectExtract,
                prefers_json,
                output_file,
                error,
            );
        }
    };
    execute_extraction(prepared)
}

pub(crate) fn run_slice(args: SliceArgs, verbose: u8, quiet: bool) -> ExecutionOutcome {
    let prefers_json = extract_prefers_json(&args.output);
    let output_file = args.output.output_file.clone();
    let prepared = match PreparedExtraction::from_slice_with_logging(args, verbose, quiet) {
        Ok(prepared) => prepared,
        Err(error) => {
            return operation_error_outcome(
                htmlcut_core::OperationId::SliceExtract,
                prefers_json,
                output_file,
                error,
            );
        }
    };
    execute_extraction(prepared)
}

pub(crate) fn run_inspect_source(
    args: InspectSourceArgs,
    verbose: u8,
    quiet: bool,
) -> ExecutionOutcome {
    let prefers_json = args.output == CliInspectOutputMode::Json;
    let output_file = args.output_file.clone();
    let prepared = match PreparedSourceInspection::new_with_logging(args, verbose, quiet) {
        Ok(prepared) => prepared,
        Err(error) => {
            return operation_error_outcome(
                htmlcut_core::OperationId::SourceInspect,
                prefers_json,
                output_file,
                error,
            );
        }
    };
    let result = inspect_source(&prepared.source, &prepared.runtime, &prepared.options);
    let report = build_source_inspection_report(prepared.command.clone(), result);

    if !report.ok {
        let error = with_source_load_steps(
            primary_source_inspection_error(&report.diagnostics),
            &report.source,
        );
        if prepared.output == CliInspectOutputMode::Json {
            return ExecutionOutcome {
                stdout: Some(to_pretty_json(&report)),
                output_file: prepared.output_file,
                post_write_stderr: Vec::new(),
                stderr: Vec::new(),
                exit_code: exit_code_for_error(&error),
            };
        }

        return error_outcome(prepared.command.clone(), false, None, error);
    }

    let post_write_stderr = output_file_notice(
        prepared.output_file.as_deref(),
        prepared.verbose,
        prepared.quiet,
    );
    ExecutionOutcome {
        stdout: Some(match prepared.output {
            CliInspectOutputMode::Json => to_pretty_json(&report),
            CliInspectOutputMode::Text => {
                render_source_inspection_text(&report, prepared.preview_chars)
            }
        }),
        output_file: prepared.output_file,
        post_write_stderr,
        stderr: if prepared.quiet {
            Vec::new()
        } else {
            build_source_inspection_verbose_lines(&report, prepared.verbose)
        },
        exit_code: 0,
    }
}

pub(crate) fn run_inspect_select(
    args: InspectSelectArgs,
    verbose: u8,
    quiet: bool,
) -> ExecutionOutcome {
    let prefers_json = args.output.output == CliInspectOutputMode::Json;
    let output_file = args.output.output_file.clone();
    let prepared = match PreparedPreview::from_select_with_logging(args, verbose, quiet) {
        Ok(prepared) => prepared,
        Err(error) => {
            return operation_error_outcome(
                htmlcut_core::OperationId::SelectPreview,
                prefers_json,
                output_file,
                error,
            );
        }
    };
    execute_preview(prepared)
}

pub(crate) fn run_inspect_slice(
    args: InspectSliceArgs,
    verbose: u8,
    quiet: bool,
) -> ExecutionOutcome {
    let prefers_json = args.output.output == CliInspectOutputMode::Json;
    let output_file = args.output.output_file.clone();
    let prepared = match PreparedPreview::from_slice_with_logging(args, verbose, quiet) {
        Ok(prepared) => prepared,
        Err(error) => {
            return operation_error_outcome(
                htmlcut_core::OperationId::SlicePreview,
                prefers_json,
                output_file,
                error,
            );
        }
    };
    execute_preview(prepared)
}

fn operation_error_outcome(
    operation_id: htmlcut_core::OperationId,
    prefers_json: bool,
    output_file: Option<std::path::PathBuf>,
    error: crate::error::CliError,
) -> ExecutionOutcome {
    operation_error_outcome_with_report_command(
        operation_id,
        prefers_json,
        output_file,
        error,
        lookup::operation_report_command(operation_id).ok(),
    )
}

fn operation_error_outcome_with_report_command(
    operation_id: htmlcut_core::OperationId,
    prefers_json: bool,
    output_file: Option<std::path::PathBuf>,
    error: crate::error::CliError,
    report_command: Option<String>,
) -> ExecutionOutcome {
    match report_command {
        Some(command) => error_outcome(command, prefers_json, output_file, error),
        None => error_outcome(
            operation_id.as_str().to_owned(),
            prefers_json,
            output_file,
            lookup::missing_operation_contract_error(operation_id, "report command"),
        ),
    }
}

#[cfg(test)]
pub(crate) fn operation_error_outcome_for_tests(
    operation_id: htmlcut_core::OperationId,
    prefers_json: bool,
    output_file: Option<std::path::PathBuf>,
    error: crate::error::CliError,
    report_command: Option<String>,
) -> ExecutionOutcome {
    operation_error_outcome_with_report_command(
        operation_id,
        prefers_json,
        output_file,
        error,
        report_command,
    )
}
