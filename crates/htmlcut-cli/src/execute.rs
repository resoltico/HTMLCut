use std::fs;
use std::io::Write;
use std::path::Path;

use clap::{CommandFactory, Parser, error::ErrorKind};
use htmlcut_core::{extract, inspect_source, preview_extraction};

use crate::EXIT_CODE_OUTPUT;
use crate::args::{
    CatalogArgs, Cli, CliCatalogOutputMode, CliInspectOutputMode, CliOutputMode,
    CliSchemaOutputMode, Commands, InspectCommands, InspectSelectArgs, InspectSliceArgs,
    InspectSourceArgs, SchemaArgs, SelectArgs, SliceArgs,
};
use crate::error::{
    CliError, CliErrorBody, CliErrorReport, exit_code_for_error, json_error_diagnostics,
    primary_extraction_error, primary_source_inspection_error, render_error_category, usage_error,
};
use crate::metadata::{ENGINE_NAME, HTMLCUT_VERSION, TOOL_NAME, version_banner};
use crate::prepare::{
    PreparedExtraction, PreparedPreview, PreparedSourceInspection, build_catalog_report,
    build_extraction_report, build_schema_report, build_source_inspection_report,
    extract_prefers_json,
};
use crate::render::{
    build_human_diagnostic_stderr_lines, build_verbose_lines, get_bundle_paths,
    render_catalog_text, render_extraction_output, render_preview_text, render_schema_text,
    render_source_inspection_text, to_pretty_json, write_bundle,
};

pub(crate) struct ExecutionOutcome {
    pub(crate) stdout: Option<String>,
    pub(crate) output_file: Option<std::path::PathBuf>,
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
        Commands::Catalog(args) => run_catalog(args),
        Commands::Schema(args) => run_schema(args),
        Commands::Select(args) => run_select(args, verbose, quiet),
        Commands::Slice(args) => run_slice(args, verbose, quiet),
        Commands::Inspect(args) => match args.command {
            InspectCommands::Source(args) => run_inspect_source(args),
            InspectCommands::Select(args) => run_inspect_select(args),
            InspectCommands::Slice(args) => run_inspect_slice(args),
        },
    }
}

pub(crate) fn run_catalog(args: CatalogArgs) -> ExecutionOutcome {
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

    ExecutionOutcome {
        stdout: Some(match args.output {
            CliCatalogOutputMode::Json => to_pretty_json(&report),
            CliCatalogOutputMode::Text => render_catalog_text(&report),
        }),
        output_file: args.output_file,
        stderr: Vec::new(),
        exit_code: 0,
    }
}

pub(crate) fn run_schema(args: SchemaArgs) -> ExecutionOutcome {
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

    ExecutionOutcome {
        stdout: Some(match args.output {
            CliSchemaOutputMode::Json => to_pretty_json(&report),
            CliSchemaOutputMode::Text => render_schema_text(&report),
        }),
        output_file: args.output_file,
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
            return error_outcome("select".to_owned(), prefers_json, output_file, error);
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
            return error_outcome("slice".to_owned(), prefers_json, output_file, error);
        }
    };
    execute_extraction(prepared)
}

pub(crate) fn run_inspect_source(args: InspectSourceArgs) -> ExecutionOutcome {
    let prefers_json = args.output == CliInspectOutputMode::Json;
    let output_file = args.output_file.clone();
    let prepared = match PreparedSourceInspection::new(args) {
        Ok(prepared) => prepared,
        Err(error) => {
            return error_outcome(
                "inspect-source".to_owned(),
                prefers_json,
                output_file,
                error,
            );
        }
    };
    let result = inspect_source(&prepared.source, &prepared.runtime, &prepared.options);
    let report = build_source_inspection_report(prepared.command, result);

    if !report.ok {
        let error = primary_source_inspection_error(&report.diagnostics);
        if prepared.output == CliInspectOutputMode::Json {
            return ExecutionOutcome {
                stdout: Some(to_pretty_json(&report)),
                output_file: prepared.output_file,
                stderr: Vec::new(),
                exit_code: exit_code_for_error(&error),
            };
        }

        return error_outcome(prepared.command.to_owned(), false, None, error);
    }

    ExecutionOutcome {
        stdout: Some(match prepared.output {
            CliInspectOutputMode::Json => to_pretty_json(&report),
            CliInspectOutputMode::Text => {
                render_source_inspection_text(&report, prepared.preview_chars)
            }
        }),
        output_file: prepared.output_file,
        stderr: Vec::new(),
        exit_code: 0,
    }
}

pub(crate) fn run_inspect_select(args: InspectSelectArgs) -> ExecutionOutcome {
    let prefers_json = args.output.output == CliInspectOutputMode::Json;
    let output_file = args.output.output_file.clone();
    let prepared = match PreparedPreview::from_select(args) {
        Ok(prepared) => prepared,
        Err(error) => {
            return error_outcome(
                "inspect-select".to_owned(),
                prefers_json,
                output_file,
                error,
            );
        }
    };
    execute_preview(prepared)
}

pub(crate) fn run_inspect_slice(args: InspectSliceArgs) -> ExecutionOutcome {
    let prefers_json = args.output.output == CliInspectOutputMode::Json;
    let output_file = args.output.output_file.clone();
    let prepared = match PreparedPreview::from_slice(args) {
        Ok(prepared) => prepared,
        Err(error) => {
            return error_outcome("inspect-slice".to_owned(), prefers_json, output_file, error);
        }
    };
    execute_preview(prepared)
}

pub(crate) fn execute_extraction(prepared: PreparedExtraction) -> ExecutionOutcome {
    let result = extract(&prepared.request, &prepared.runtime);
    let bundle_paths = prepared.bundle.as_deref().map(get_bundle_paths);
    let report = build_extraction_report(prepared.command, result, bundle_paths.clone());

    if !report.ok {
        let error = primary_extraction_error(&report.diagnostics);
        return if prepared.output == CliOutputMode::Json {
            ExecutionOutcome {
                stdout: Some(to_pretty_json(&report)),
                output_file: prepared.output_file,
                stderr: Vec::new(),
                exit_code: exit_code_for_error(&error),
            }
        } else {
            error_outcome(prepared.command.to_owned(), false, None, error)
        };
    }

    if let Some(bundle) = bundle_paths.as_ref()
        && let Err(error) = write_bundle(&report, bundle)
    {
        return error_outcome(
            prepared.command.to_owned(),
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

    ExecutionOutcome {
        stdout: render_extraction_output(&report, prepared.output),
        output_file: prepared.output_file,
        stderr,
        exit_code: 0,
    }
}

pub(crate) fn execute_preview(prepared: PreparedPreview) -> ExecutionOutcome {
    let result = preview_extraction(&prepared.request, &prepared.runtime);
    let report = build_extraction_report(prepared.command, result, None);

    if !report.ok {
        let error = primary_extraction_error(&report.diagnostics);
        if prepared.output == CliInspectOutputMode::Json {
            return ExecutionOutcome {
                stdout: Some(to_pretty_json(&report)),
                output_file: prepared.output_file,
                stderr: Vec::new(),
                exit_code: exit_code_for_error(&error),
            };
        }

        return error_outcome(prepared.command.to_owned(), false, None, error);
    }

    ExecutionOutcome {
        stdout: Some(match prepared.output {
            CliInspectOutputMode::Json => to_pretty_json(&report),
            CliInspectOutputMode::Text => render_preview_text(&report),
        }),
        output_file: prepared.output_file,
        stderr: Vec::new(),
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
        stderr: Vec::new(),
        exit_code,
    }
}

pub(crate) fn human_error_outcome(error: CliError) -> ExecutionOutcome {
    ExecutionOutcome {
        stdout: None,
        output_file: None,
        stderr: vec![format!("htmlcut: {}", error.message)],
        exit_code: exit_code_for_error(&error),
    }
}

pub(crate) fn write_outcome<W1, W2>(
    outcome: ExecutionOutcome,
    stdout: &mut W1,
    stderr: &mut W2,
) -> i32
where
    W1: Write,
    W2: Write,
{
    if let Some(stdout_payload) = outcome.stdout.as_ref() {
        if let Some(path) = outcome.output_file.as_deref() {
            if let Err(error) = write_stdout_payload(path, stdout_payload) {
                let _ = writeln!(
                    stderr,
                    "htmlcut: Could not write {}: {error}",
                    path.display()
                );
                return EXIT_CODE_OUTPUT;
            }
        } else {
            let _ = writeln!(stdout, "{stdout_payload}");
        }
    }
    for line in &outcome.stderr {
        let _ = writeln!(stderr, "{line}");
    }
    outcome.exit_code
}

pub(crate) fn raw_args_prefers_json(raw_args: &[String]) -> bool {
    let mut explicit_output = None;
    let mut inspect_mode = false;
    let mut structured_value = false;

    for (index, arg) in raw_args.iter().enumerate().skip(1) {
        if arg == "inspect" {
            inspect_mode = true;
        }
        if arg == "--value"
            && raw_args
                .get(index + 1)
                .is_some_and(|value| value == "structured")
        {
            structured_value = true;
        }
        if let Some(value) = arg.strip_prefix("--output=") {
            explicit_output = Some(value.to_owned());
        }
        if arg == "--output"
            && let Some(value) = raw_args.get(index + 1)
        {
            explicit_output = Some(value.clone());
        }
    }

    match explicit_output.as_deref() {
        Some("json") => true,
        Some("text") | Some("html") | Some("none") => false,
        _ => inspect_mode || structured_value,
    }
}

pub(crate) fn raw_args_requests_version(raw_args: &[String]) -> bool {
    raw_option_tokens(raw_args).any(|arg| matches!(arg, "--version" | "-V"))
}

pub(crate) fn raw_args_requests_help(raw_args: &[String]) -> bool {
    raw_option_tokens(raw_args).any(|arg| matches!(arg, "--help" | "-h"))
}

fn raw_option_tokens(raw_args: &[String]) -> impl Iterator<Item = &str> {
    raw_args
        .iter()
        .skip(1)
        .take_while(|arg| arg.as_str() != "--")
        .map(String::as_str)
}

pub(crate) fn command_name_from_raw_args(raw_args: &[String]) -> String {
    match raw_args.get(1).map(String::as_str) {
        Some("catalog") => "catalog".to_owned(),
        Some("schema") => "schema".to_owned(),
        Some("select") => "select".to_owned(),
        Some("slice") => "slice".to_owned(),
        Some("inspect") => match raw_args.get(2).map(String::as_str) {
            Some("source") => "inspect-source".to_owned(),
            Some("select") => "inspect-select".to_owned(),
            Some("slice") => "inspect-slice".to_owned(),
            _ => "inspect".to_owned(),
        },
        _ => TOOL_NAME.to_owned(),
    }
}

pub(crate) fn clap_error_message(error: &clap::Error) -> String {
    let rendered = error.to_string();
    rendered
        .lines()
        .find_map(|line| line.strip_prefix("error: ").map(ToOwned::to_owned))
        .unwrap_or_else(|| rendered.trim().to_owned())
}

fn write_stdout_payload(path: &Path, stdout_payload: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }

    fs::write(path, format!("{stdout_payload}\n"))
}

#[cfg(test)]
pub(crate) fn write_stdout_payload_for_tests(
    path: &Path,
    stdout_payload: &str,
) -> std::io::Result<()> {
    write_stdout_payload(path, stdout_payload)
}
