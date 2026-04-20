use std::io::Write;

mod cli_io;
mod raw_args;

use clap::{CommandFactory, Parser, error::ErrorKind};
use htmlcut_core::{extract, inspect_source, preview_extraction};

pub(crate) use self::cli_io::{output_file_notice, write_outcome, write_request_definition};
#[cfg(test)]
pub(crate) use self::cli_io::{
    request_definition_parent_dir_for_tests, write_stdout_payload_for_tests,
};
pub(crate) use self::raw_args::{
    clap_error_message, command_name_from_raw_args, raw_args_prefers_json, raw_args_requests_help,
    raw_args_requests_version, report_command_for_operation,
};
use crate::args::{
    CatalogArgs, Cli, CliCatalogOutputMode, CliInspectOutputMode, CliOutputMode,
    CliSchemaOutputMode, Commands, InspectCommands, InspectSelectArgs, InspectSliceArgs,
    InspectSourceArgs, SchemaArgs, SelectArgs, SliceArgs,
};
use crate::error::{
    CliError, CliErrorBody, CliErrorReport, exit_code_for_error, json_error_diagnostics,
    primary_extraction_error, primary_source_inspection_error, render_error_category, usage_error,
    with_source_load_steps,
};
use crate::metadata::{ENGINE_NAME, HTMLCUT_VERSION, TOOL_NAME, version_banner};
use crate::prepare::{
    PreparedExtraction, PreparedPreview, PreparedSourceInspection, build_catalog_report,
    build_extraction_report, build_schema_report, build_source_inspection_report,
    extract_prefers_json,
};
use crate::render::{
    build_human_diagnostic_stderr_lines, build_source_inspection_verbose_lines,
    build_source_load_error_lines, build_verbose_lines, get_bundle_paths, render_catalog_text,
    render_extraction_output, render_preview_text, render_schema_text,
    render_source_inspection_text, to_pretty_json, write_bundle,
};

pub(crate) struct ExecutionOutcome {
    pub(crate) stdout: Option<String>,
    pub(crate) output_file: Option<std::path::PathBuf>,
    pub(crate) post_write_stderr: Vec<String>,
    pub(crate) stderr: Vec<String>,
    pub(crate) exit_code: i32,
}

/// Executes the HTMLCut CLI against one argv stream and writes the rendered result.
pub fn run<I, W1, W2>(args: I, stdout: &mut W1, stderr: &mut W2) -> i32
where
    I: IntoIterator<Item = String>,
    W1: Write,
    W2: Write,
{
    let raw_args: Vec<String> = args.into_iter().collect();
    if raw_args.len() <= 1 {
        let mut command = Cli::command();
        let _ = command.write_long_help(stdout);
        let _ = writeln!(stdout);
        return 0;
    }

    if raw_args_requests_version(&raw_args) && !raw_args_requests_help(&raw_args) {
        let _ = writeln!(stdout, "{}", version_banner());
        return 0;
    }

    let prefers_json_errors = raw_args_prefers_json(&raw_args);
    let parsed = match Cli::try_parse_from(raw_args.clone()) {
        Ok(args) => args,
        Err(error) => {
            if error.kind() == ErrorKind::DisplayHelp {
                let _ = write!(stdout, "{error}");
                return 0;
            }

            let outcome = error_outcome(
                command_name_from_raw_args(&raw_args),
                prefers_json_errors,
                None,
                usage_error("CLI_PARSE_ERROR", clap_error_message(&error)),
            );
            return write_outcome(outcome, stdout, stderr);
        }
    };

    write_outcome(execute(parsed), stdout, stderr)
}

pub(crate) fn execute(cli: Cli) -> ExecutionOutcome {
    let verbose = cli.global.verbose;
    let quiet = cli.global.quiet;
    match cli.command {
        Commands::Catalog(args) => run_catalog(args, verbose, quiet),
        Commands::Schema(args) => run_schema(args, verbose, quiet),
        Commands::Select(args) => run_select(args, verbose, quiet),
        Commands::Slice(args) => run_slice(args, verbose, quiet),
        Commands::Inspect(args) => match args.command {
            InspectCommands::Source(args) => run_inspect_source(args, verbose, quiet),
            InspectCommands::Select(args) => run_inspect_select(args, verbose, quiet),
            InspectCommands::Slice(args) => run_inspect_slice(args, verbose, quiet),
        },
    }
}

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
                report_command_for_operation(htmlcut_core::OperationId::SelectExtract),
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
                report_command_for_operation(htmlcut_core::OperationId::SliceExtract),
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
                report_command_for_operation(htmlcut_core::OperationId::SourceInspect),
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
                report_command_for_operation(htmlcut_core::OperationId::SelectPreview),
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
                report_command_for_operation(htmlcut_core::OperationId::SlicePreview),
                prefers_json,
                output_file,
                error,
            );
        }
    };
    execute_preview(prepared)
}

pub(crate) fn execute_extraction(prepared: PreparedExtraction) -> ExecutionOutcome {
    if let Some(request_definition_output) = prepared.request_definition_output.as_ref()
        && let Err(error) = write_request_definition(request_definition_output)
    {
        return error_outcome(
            prepared.command.clone(),
            prepared.output == CliOutputMode::Json,
            prepared.output_file.clone(),
            error,
        );
    }

    let result = extract(&prepared.request, &prepared.runtime);
    let bundle_paths = prepared.bundle.as_deref().map(get_bundle_paths);
    let report = build_extraction_report(prepared.command.clone(), result, bundle_paths.clone());

    if !report.ok {
        let error = with_source_load_steps(
            primary_extraction_error(&report.diagnostics),
            &report.source,
        );
        return if prepared.output == CliOutputMode::Json {
            ExecutionOutcome {
                stdout: Some(to_pretty_json(&report)),
                output_file: prepared.output_file,
                post_write_stderr: Vec::new(),
                stderr: Vec::new(),
                exit_code: exit_code_for_error(&error),
            }
        } else {
            error_outcome(prepared.command.clone(), false, None, error)
        };
    }

    if let Some(bundle) = bundle_paths.as_ref()
        && let Err(error) = write_bundle(&report, bundle)
    {
        return error_outcome(
            prepared.command.clone(),
            prepared.output == CliOutputMode::Json,
            prepared.output_file.clone(),
            error,
        );
    }

    let mut stderr = if prepared.quiet {
        Vec::new()
    } else {
        build_verbose_lines(&report, prepared.verbose)
    };
    if !prepared.quiet && prepared.output != CliOutputMode::Json {
        stderr.extend(build_human_diagnostic_stderr_lines(&report.diagnostics));
    }
    if !prepared.quiet
        && prepared.verbose > 0
        && let Some(bundle) = report.bundle.as_ref()
    {
        stderr.push(format!("htmlcut: wrote bundle to {}", bundle.dir));
    }
    if !prepared.quiet
        && prepared.verbose > 0
        && let Some(request_definition_output) = prepared.request_definition_output.as_ref()
    {
        stderr.push(format!(
            "htmlcut: wrote request file to {}",
            request_definition_output.path.display()
        ));
    }

    let post_write_stderr = output_file_notice(
        prepared.output_file.as_deref(),
        prepared.verbose,
        prepared.quiet,
    );
    ExecutionOutcome {
        stdout: render_extraction_output(&report, prepared.output),
        output_file: prepared.output_file,
        post_write_stderr,
        stderr,
        exit_code: 0,
    }
}

pub(crate) fn execute_preview(prepared: PreparedPreview) -> ExecutionOutcome {
    if let Some(request_definition_output) = prepared.request_definition_output.as_ref()
        && let Err(error) = write_request_definition(request_definition_output)
    {
        return error_outcome(
            prepared.command.clone(),
            prepared.output == CliInspectOutputMode::Json,
            prepared.output_file.clone(),
            error,
        );
    }

    let result = preview_extraction(&prepared.request, &prepared.runtime);
    let report = build_extraction_report(prepared.command.clone(), result, None);

    if !report.ok {
        let error = with_source_load_steps(
            primary_extraction_error(&report.diagnostics),
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
            CliInspectOutputMode::Text => render_preview_text(&report),
            CliInspectOutputMode::Html | CliInspectOutputMode::None => {
                unreachable!("inspect output parser only allows text/json")
            }
        }),
        output_file: prepared.output_file,
        post_write_stderr,
        stderr: if prepared.quiet {
            Vec::new()
        } else {
            let mut stderr = build_verbose_lines(&report, prepared.verbose);
            if prepared.verbose > 0
                && let Some(request_definition_output) = prepared.request_definition_output.as_ref()
            {
                stderr.push(format!(
                    "htmlcut: wrote request file to {}",
                    request_definition_output.path.display()
                ));
            }
            stderr
        },
        exit_code: 0,
    }
}

pub(crate) fn error_outcome(
    command: String,
    prefers_json: bool,
    output_file: Option<std::path::PathBuf>,
    error: CliError,
) -> ExecutionOutcome {
    match prefers_json {
        true => json_error_outcome(command, output_file, error),
        false => human_error_outcome(error),
    }
}

pub(crate) fn json_error_outcome(
    command: String,
    output_file: Option<std::path::PathBuf>,
    error: CliError,
) -> ExecutionOutcome {
    let exit_code = exit_code_for_error(&error);
    let diagnostics = json_error_diagnostics(&error);

    ExecutionOutcome {
        stdout: Some(to_pretty_json(&CliErrorReport {
            tool: TOOL_NAME.to_owned(),
            engine: ENGINE_NAME.to_owned(),
            version: HTMLCUT_VERSION.to_owned(),
            command,
            ok: false,
            exit_code,
            error: CliErrorBody {
                category: render_error_category(error.category).to_owned(),
                code: error.code,
                message: error.message,
            },
            diagnostics,
        })),
        output_file,
        post_write_stderr: Vec::new(),
        stderr: Vec::new(),
        exit_code,
    }
}

pub(crate) fn human_error_outcome(error: CliError) -> ExecutionOutcome {
    let mut stderr = vec![format!("htmlcut: {}", error.message)];
    stderr.extend(build_human_diagnostic_stderr_lines(&error.diagnostics));
    stderr.extend(build_source_load_error_lines(&error.source_load_steps));

    ExecutionOutcome {
        stdout: None,
        output_file: None,
        post_write_stderr: Vec::new(),
        stderr,
        exit_code: exit_code_for_error(&error),
    }
}
