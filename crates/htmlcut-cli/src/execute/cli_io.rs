use std::fs;
use std::io::Write;
use std::path::Path;

use crate::EXIT_CODE_OUTPUT;
use crate::error::CliError;
use crate::model::CliErrorCode;

use super::ExecutionOutcome;

pub(crate) fn write_outcome<W1, W2>(
    outcome: ExecutionOutcome,
    stdout: &mut W1,
    stderr: &mut W2,
) -> std::io::Result<i32>
where
    W1: Write,
    W2: Write,
{
    if let Some(stdout_payload) = outcome.stdout.as_ref() {
        if let Some(path) = outcome.output_file.as_deref() {
            if let Err(error) = write_stdout_payload(path, stdout_payload) {
                writeln!(
                    stderr,
                    "htmlcut: Could not write {}: {error}",
                    path.display()
                )?;
                return Ok(EXIT_CODE_OUTPUT);
            }
        } else {
            writeln!(stdout, "{stdout_payload}")?;
        }
    }
    for line in &outcome.stderr {
        writeln!(stderr, "{line}")?;
    }
    for line in &outcome.post_write_stderr {
        writeln!(stderr, "{line}")?;
    }
    Ok(outcome.exit_code)
}

pub(crate) fn output_file_notice(path: Option<&Path>, verbose: u8, quiet: bool) -> Vec<String> {
    if quiet || verbose == 0 {
        return Vec::new();
    }

    path.map(|path| format!("htmlcut: wrote output file to {}", path.display()))
        .into_iter()
        .collect()
}

pub(crate) fn write_request_definition(
    request_definition_output: &crate::prepare::PendingExtractionDefinitionWrite,
) -> Result<(), CliError> {
    if let Some(parent) = request_definition_parent_dir(&request_definition_output.path) {
        fs::create_dir_all(parent).map_err(|error| {
            crate::error::output_error(
                CliErrorCode::RequestFileWriteFailed,
                format!(
                    "Could not create request file directory {}: {error}",
                    parent.display()
                ),
            )
        })?;
    }

    let definition = crate::render::render_json_string(
        &request_definition_output.definition,
        &format!(
            "request definition {}",
            request_definition_output.path.display()
        ),
    )?;

    fs::write(&request_definition_output.path, format!("{definition}\n")).map_err(|error| {
        crate::error::output_error(
            CliErrorCode::RequestFileWriteFailed,
            format!(
                "Could not write request file {}: {error}",
                request_definition_output.path.display(),
            ),
        )
    })
}

fn write_stdout_payload(path: &Path, stdout_payload: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }

    fs::write(path, format!("{stdout_payload}\n"))
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
