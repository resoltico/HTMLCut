use super::{ExecutionOutcome, output_file_notice, write_request_definition};
use crate::args::{CliInspectOutputMode, CliOutputMode};
use crate::error::{
    CliError, CliErrorBody, CliErrorReport, exit_code_for_error, json_error_diagnostics,
    primary_extraction_error, render_error_category, with_source_load_steps,
};
use crate::metadata::{ENGINE_NAME, HTMLCUT_VERSION, TOOL_NAME};
use crate::prepare::{PreparedExtraction, PreparedPreview, build_extraction_report};
use crate::render::{
    build_human_diagnostic_stderr_lines, build_source_load_error_lines, build_verbose_lines,
    get_bundle_paths, render_extraction_output, render_preview_text, to_pretty_json, write_bundle,
};
use htmlcut_core::{extract, preview_extraction};

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
