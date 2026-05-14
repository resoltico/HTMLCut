use std::io::Write;
use std::path::Path;

use crate::EXIT_CODE_OUTPUT;
use crate::error::CliError;
use crate::file_output::{FileWriteMode, write_text_file};
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
            if let Err(error) = write_stdout_payload(path, stdout_payload, outcome.write_mode) {
                for line in &outcome.stderr {
                    writeln!(stderr, "{line}")?;
                }
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

pub(crate) fn output_file_notice(path: Option<&Path>, quiet: bool) -> Vec<String> {
    if quiet {
        return Vec::new();
    }

    path.map(|path| format!("htmlcut: wrote output file to {}", path.display()))
        .into_iter()
        .collect()
}

pub(crate) fn write_request_definition(
    request_definition_output: &crate::prepare::PendingExtractionDefinitionWrite,
    write_mode: FileWriteMode,
) -> Result<(), CliError> {
    let definition = crate::render::render_json_string(
        &request_definition_output.document,
        &format!(
            "request definition {}",
            request_definition_output.path.display()
        ),
    )?;

    write_text_file(
        &request_definition_output.path,
        &format!("{definition}\n"),
        write_mode,
    )
    .map_err(|error| {
        crate::error::output_error(
            CliErrorCode::RequestFileWriteFailed,
            format!(
                "Could not write request file {}: {error}",
                request_definition_output.path.display(),
            ),
        )
    })
}

fn write_stdout_payload(
    path: &Path,
    stdout_payload: &str,
    write_mode: FileWriteMode,
) -> std::io::Result<()> {
    write_text_file(path, &format!("{stdout_payload}\n"), write_mode)
}

#[cfg(test)]
fn request_definition_parent_dir(path: &Path) -> Option<&Path> {
    let parent = path.parent()?;
    (!parent.as_os_str().is_empty()).then_some(parent)
}

#[cfg(test)]
pub(crate) fn write_stdout_payload_for_tests(
    path: &Path,
    stdout_payload: &str,
    write_mode: FileWriteMode,
) -> std::io::Result<()> {
    write_stdout_payload(path, stdout_payload, write_mode)
}

#[cfg(test)]
pub(crate) fn request_definition_parent_dir_for_tests(path: &Path) -> Option<&Path> {
    request_definition_parent_dir(path)
}
