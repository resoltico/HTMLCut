use std::fs;
use std::io::Write;
use std::path::Path;

use clap::ValueEnum;
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
    for line in &outcome.post_write_stderr {
        let _ = writeln!(stderr, "{line}");
    }
    outcome.exit_code
}

pub(crate) fn raw_args_prefers_json(raw_args: &[String]) -> bool {
    let mut explicit_output = None;
    let mut inspect_mode = false;
    let mut structured_value = false;
    let structured_value_mode = value_enum_name(crate::args::CliValueMode::Structured);
    let json_output_mode = value_enum_name(crate::args::CliOutputMode::Json);
    let text_output_mode = value_enum_name(crate::args::CliOutputMode::Text);
    let html_output_mode = value_enum_name(crate::args::CliOutputMode::Html);
    let none_output_mode = value_enum_name(crate::args::CliOutputMode::None);

    for (index, arg) in raw_args.iter().enumerate().skip(1) {
        if arg == "inspect" {
            inspect_mode = true;
        }
        if arg == "--value"
            && raw_args
                .get(index + 1)
                .is_some_and(|value| value == structured_value_mode.as_str())
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
        Some(value) if value == json_output_mode.as_str() => true,
        Some(value)
            if value == text_output_mode.as_str()
                || value == html_output_mode.as_str()
                || value == none_output_mode.as_str() =>
        {
            false
        }
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
    let command_tokens = raw_command_tokens(raw_args);
    let Some(first_token) = command_tokens.first().copied() else {
        return TOOL_NAME.to_owned();
    };

    if let Some(contract) = htmlcut_core::cli_operation_catalog()
        .iter()
        .filter(|contract| command_tokens.len() >= contract.command_path.len())
        .find(|contract| command_tokens.starts_with(contract.command_path))
    {
        return contract.report_command();
    }

    match first_token {
        "catalog" => "catalog".to_owned(),
        "schema" => "schema".to_owned(),
        "inspect" => "inspect".to_owned(),
        command => command.to_owned(),
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

fn output_file_notice(path: Option<&Path>, verbose: u8, quiet: bool) -> Vec<String> {
    if quiet || verbose == 0 {
        return Vec::new();
    }

    path.map(|path| format!("htmlcut: wrote output file to {}", path.display()))
        .into_iter()
        .collect()
}

fn value_enum_name<T: ValueEnum>(value: T) -> String {
    value
        .to_possible_value()
        .expect("CLI value-enum variant should always render")
        .get_name()
        .to_owned()
}

fn raw_command_tokens(raw_args: &[String]) -> Vec<&str> {
    raw_args
        .iter()
        .skip(1)
        .take_while(|arg| arg.as_str() != "--")
        .map(String::as_str)
        .skip_while(|arg| arg.starts_with('-'))
        .take_while(|arg| !arg.starts_with('-'))
        .collect()
}

fn report_command_for_operation(operation_id: htmlcut_core::OperationId) -> String {
    htmlcut_core::cli_operation_report_command(operation_id)
        .expect("CLI-visible operation should expose a report command")
}

fn write_request_definition(
    request_definition_output: &crate::prepare::PendingExtractionDefinitionWrite,
) -> Result<(), CliError> {
    if let Some(parent) = request_definition_parent_dir(&request_definition_output.path) {
        fs::create_dir_all(parent).map_err(|error| {
            crate::error::output_error(
                "CLI_REQUEST_FILE_WRITE_FAILED",
                format!(
                    "Could not create request file directory {}: {error}",
                    parent.display()
                ),
            )
        })?;
    }

    fs::write(
        &request_definition_output.path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&request_definition_output.definition)
                .expect("request definitions should always serialize"),
        ),
    )
    .map_err(|error| {
        crate::error::output_error(
            "CLI_REQUEST_FILE_WRITE_FAILED",
            format!(
                "Could not write request file {}: {error}",
                request_definition_output.path.display()
            ),
        )
    })
}

fn request_definition_parent_dir(path: &Path) -> Option<&Path> {
    let parent = path.parent()?;
    (!parent.as_os_str().is_empty()).then_some(parent)
}

#[cfg(test)]
pub(crate) fn write_stdout_payload_for_tests(
    path: &Path,
    stdout_payload: &str,
) -> std::io::Result<()> {
    write_stdout_payload(path, stdout_payload)
}

#[cfg(test)]
pub(crate) fn request_definition_parent_dir_for_tests(path: &Path) -> Option<&Path> {
    request_definition_parent_dir(path)
}
