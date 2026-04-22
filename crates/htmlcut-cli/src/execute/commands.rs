use htmlcut_core::inspect_source;

use super::{
    ExecutionOutcome, error_outcome, execute_extraction, execute_preview, output_file_notice,
};
use crate::args::{
    CatalogArgs, CliCatalogOutputMode, CliInspectOutputMode, CliSchemaOutputMode,
    InspectSelectArgs, InspectSliceArgs, InspectSourceArgs, SchemaArgs, SelectArgs, SliceArgs,
};
use crate::error::{exit_code_for_error, primary_source_inspection_error, with_source_load_steps};
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
            CliCatalogOutputMode::Html | CliCatalogOutputMode::None => {
                unreachable!("catalog output parser only allows text/json")
            }
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
            CliSchemaOutputMode::Html | CliSchemaOutputMode::None => {
                unreachable!("schema output parser only allows text/json")
            }
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
            return error_outcome(
                htmlcut_core::cli_operation_report_command(
                    htmlcut_core::OperationId::SelectExtract,
                )
                .expect("CLI-visible operation should expose a report command"),
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
            return error_outcome(
                htmlcut_core::cli_operation_report_command(htmlcut_core::OperationId::SliceExtract)
                    .expect("CLI-visible operation should expose a report command"),
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
            return error_outcome(
                htmlcut_core::cli_operation_report_command(
                    htmlcut_core::OperationId::SourceInspect,
                )
                .expect("CLI-visible operation should expose a report command"),
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
            CliInspectOutputMode::Html | CliInspectOutputMode::None => {
                unreachable!("inspect output parser only allows text/json")
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
            return error_outcome(
                htmlcut_core::cli_operation_report_command(
                    htmlcut_core::OperationId::SelectPreview,
                )
                .expect("CLI-visible operation should expose a report command"),
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
            return error_outcome(
                htmlcut_core::cli_operation_report_command(htmlcut_core::OperationId::SlicePreview)
                    .expect("CLI-visible operation should expose a report command"),
                prefers_json,
                output_file,
                error,
            );
        }
    };
    execute_preview(prepared)
}
