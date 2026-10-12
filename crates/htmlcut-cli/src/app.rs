// SPDX-License-Identifier: MPL-2.0
//! Binary composition: validate, acquire one snapshot, execute, stage, then deliver.

use crate::command::{Cli, Operation, Output};
use crate::input::options;
use crate::publication::Staged;
use clap::Parser;
use htmlcut_core::ExtractionError;
use std::{ffi::OsString, io::Write, path::PathBuf};

pub(crate) fn run<I, T, R: crate::input::InputRead + ?Sized>(
    arguments: I,
    stdin: &mut R,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let cli = match Cli::try_parse_from(arguments) {
        Ok(cli) => cli,
        Err(error)
            if matches!(
                error.kind(),
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
            ) =>
        {
            return match stdout
                .write_all(error.to_string().as_bytes())
                .and_then(|_| stdout.flush())
            {
                Ok(()) => 0,
                Err(error) => report(
                    crate::publication::io_failure(error, htmlcut_core::IoOperation::Stdout),
                    stderr,
                ),
            };
        }
        Err(error) => return report(crate::command_diagnostics::failure(&error), stderr),
    };
    match dispatch(cli.operation, stdin, stdout) {
        Ok(()) => 0,
        Err(error) => report(error, stderr),
    }
}

#[cfg(test)]
pub(crate) fn run_anonymous<I, T>(
    arguments: I,
    stdin: &mut dyn std::io::Read,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> i32
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    run(arguments, stdin, stdout, stderr)
}

fn report(error: ExtractionError, stderr: &mut dyn Write) -> i32 {
    let code = error.code.exit_class() as i32;
    if let Ok(bytes) = crate::publication::json(&error, 4096) {
        if stderr
            .write_all(&bytes)
            .and_then(|_| stderr.flush())
            .is_err()
        {
            return 5;
        }
    } else {
        return 5;
    }
    code
}

fn dispatch<R: crate::input::InputRead + ?Sized>(
    operation: Operation,
    stdin: &mut R,
    stdout: &mut dyn Write,
) -> Result<(), ExtractionError> {
    match operation {
        Operation::Schema { name } => {
            let schema = htmlcut_core::schema(&name)?;
            emit_json(&schema, crate::input::MAX_CONFIG_BYTES, stdout)
        }
        Operation::Extract(arguments) => {
            let plan = arguments.extraction_plan(stdin)?;
            let query_stdin = arguments
                .plan
                .as_ref()
                .is_some_and(|path| path.as_os_str() == "-");
            if arguments.output.raw && plan.fields.is_some() {
                return Err(options("Raw output cannot represent records."));
            }
            if arguments.output.raw
                && plan.match_mode == htmlcut_core::Match::All
                && (plan.min.unwrap_or(1) > 1 || plan.max == Some(0))
            {
                return Err(options(
                    "Raw output requires exactly one scalar; declared all bounds cannot provide it.",
                ));
            }
            let inputs = arguments
                .source
                .source
                .file
                .iter()
                .cloned()
                .chain(
                    arguments
                        .plan
                        .iter()
                        .filter(|path| path.as_os_str() != "-")
                        .cloned(),
                )
                .collect::<Vec<_>>();
            let bytes = crate::worker::execute(
                crate::worker::Request {
                    plan: Some(plan),
                    inspect: None,
                    samples: 0,
                    raw: arguments.output.raw,
                },
                || {
                    if query_stdin {
                        validate_stream(stdin, &arguments.output)?;
                    }
                    validate(&inputs, &arguments.output)?;
                    let snapshot = crate::input::snapshot(
                        arguments.source.source.file.as_deref(),
                        stdin,
                        arguments.source.base_url.as_deref(),
                    )?;
                    if arguments.source.source.stdin {
                        validate_stream(stdin, &arguments.output)?;
                    }
                    Ok(snapshot)
                },
            )?;
            if query_stdin || arguments.source.source.stdin {
                validate_stream(stdin, &arguments.output)?;
            }
            publish(bytes, &arguments.output, stdout)
        }
        Operation::Inspect(arguments) => {
            if !(1..=10).contains(&arguments.samples) {
                return Err(options("Samples must be between one and ten."));
            }
            emit(
                crate::worker::execute(
                    crate::worker::Request {
                        plan: None,
                        inspect: Some(arguments.select),
                        samples: arguments.samples,
                        raw: false,
                    },
                    || {
                        crate::input::snapshot(
                            arguments.source.source.file.as_deref(),
                            stdin,
                            arguments.source.base_url.as_deref(),
                        )
                    },
                )?,
                stdout,
            )
        }
    }
}

fn validate_stream<R: crate::input::InputRead + ?Sized>(
    stdin: &R,
    output: &Output,
) -> Result<(), ExtractionError> {
    if output.output.is_none() {
        return Ok(());
    }
    let identity = stdin
        .regular_file_identity()
        .map_err(crate::input::io_failure)?;
    crate::publication::validate_stdin_identity(identity, output.output.as_deref())
}

fn validate(inputs: &[PathBuf], output: &Output) -> Result<(), ExtractionError> {
    let targets = output.output.iter().cloned().collect::<Vec<_>>();
    crate::publication::validate_destinations(inputs, &targets, output.overwrite)
}

fn publish(bytes: Vec<u8>, output: &Output, stdout: &mut dyn Write) -> Result<(), ExtractionError> {
    let target = output
        .output
        .as_ref()
        .map(|path| Staged::prepare(path, &bytes, output.overwrite))
        .transpose()?;
    match target {
        Some(target) => target.commit(),
        None => emit(bytes, stdout),
    }
}

fn emit(bytes: Vec<u8>, stdout: &mut dyn Write) -> Result<(), ExtractionError> {
    stdout
        .write_all(&bytes)
        .and_then(|_| stdout.flush())
        .map_err(|error| crate::publication::io_failure(error, htmlcut_core::IoOperation::Stdout))
}

fn emit_json(
    value: &impl serde::Serialize,
    maximum: usize,
    stdout: &mut dyn Write,
) -> Result<(), ExtractionError> {
    emit(crate::publication::json(value, maximum)?, stdout)
}

#[cfg(test)]
mod report_tests {
    use super::*;
    #[test]
    fn oversized_error_fails_without_emitting_partial_diagnostics() {
        let mut error = ExtractionError::new(
            htmlcut_core::ErrorCode::InternalInvariant,
            "report",
            "bounded",
        );
        error.message = "x".repeat(4096);
        let mut stderr = Vec::new();
        assert_eq!(report(error, &mut stderr), 5);
        assert!(stderr.is_empty());
    }
}
