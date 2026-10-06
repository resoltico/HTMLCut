// SPDX-License-Identifier: MPL-2.0
//! Binary composition: validate, acquire one snapshot, execute, stage, then deliver.

use crate::command::{Cli, Operation, Output};
use crate::input::options;
use crate::publication::Staged;
use clap::Parser;
use htmlcut_core::{
    CompiledPlan, ExtractionData, ExtractionError, ExtractionResult, PreparationLimits,
    PreparedDocument,
};
use std::{
    ffi::OsString,
    io::{Read, Write},
    path::{Path, PathBuf},
};
const METADATA_BYTES: usize = 16_384;

pub(crate) fn run<I, T>(
    arguments: I,
    stdin: &mut dyn Read,
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

fn dispatch(
    operation: Operation,
    stdin: &mut dyn Read,
    stdout: &mut dyn Write,
) -> Result<(), ExtractionError> {
    match operation {
        Operation::Schema { name } => {
            let schema = if name == "htmlcut.bundle" {
                schemars::schema_for!(crate::bundle::Manifest<'static>)
                    .as_value()
                    .clone()
            } else {
                htmlcut_core::schema(&name)?
            };
            emit_json(&schema, crate::input::MAX_CONFIG_BYTES, stdout)
        }
        Operation::Extract(arguments) => {
            let compiled = CompiledPlan::compile(&arguments.extraction_plan(stdin)?)?;
            if arguments.output.raw && compiled.plan().fields.is_some() {
                return Err(options("Raw output cannot represent records."));
            }
            if arguments.output.raw
                && compiled.plan().match_mode == htmlcut_core::Match::All
                && (compiled.plan().min.unwrap_or(1) > 1 || compiled.plan().max == Some(0))
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
            validate(&inputs, &arguments.output, arguments.bundle.as_deref())?;
            let snapshot = crate::input::snapshot(
                arguments.source.source.file.as_deref(),
                stdin,
                arguments.source.base_url.as_deref(),
            )?;
            let document = PreparedDocument::new(snapshot, PreparationLimits::default())?;
            let result = document.execute(&compiled)?;
            publish(
                &result,
                &arguments.output,
                arguments.bundle.as_deref(),
                stdout,
            )
        }
        Operation::Replay(arguments) => {
            validate(
                std::slice::from_ref(&arguments.file),
                &arguments.output,
                None,
            )?;
            let replay = crate::bundle::read(&arguments.file)?;
            if arguments.output.raw && replay.plan.plan().fields.is_some() {
                return Err(options("Raw output cannot represent records."));
            }
            let result = replay.document.execute(&replay.plan)?;
            if *result.receipt()? != replay.expected {
                return Err(crate::bundle::mismatch());
            }
            publish(&result, &arguments.output, None, stdout)
        }
        Operation::Inspect(arguments) => {
            if let Some(select) = arguments.select.as_deref().or(arguments.within.as_deref()) {
                let _ = CompiledPlan::compile(&htmlcut_core::ExtractionPlan::css(select)?)?;
            }
            if arguments.select.is_some() && !(1..=10).contains(&arguments.samples.unwrap_or(3)) {
                return Err(options("Samples must be between one and ten."));
            }
            if arguments.select.is_none() && !(1..=16).contains(&arguments.limit.unwrap_or(4)) {
                return Err(options("Group limit must be between one and sixteen."));
            }
            let snapshot = crate::input::snapshot(
                arguments.source.source.file.as_deref(),
                stdin,
                arguments.source.base_url.as_deref(),
            )?;
            let document = PreparedDocument::new(snapshot, PreparationLimits::default())?;
            if let Some(select) = &arguments.select {
                emit_json(
                    &document.inspect(select, arguments.samples.unwrap_or(3))?,
                    METADATA_BYTES,
                    stdout,
                )
            } else {
                emit_json(
                    &document.survey(arguments.within.as_deref(), arguments.limit.unwrap_or(4))?,
                    METADATA_BYTES,
                    stdout,
                )
            }
        }
    }
}

fn validate(
    inputs: &[PathBuf],
    output: &Output,
    bundle: Option<&Path>,
) -> Result<(), ExtractionError> {
    let targets = output
        .output
        .iter()
        .chain(output.receipt.iter())
        .cloned()
        .chain(bundle.map(Path::to_path_buf))
        .collect::<Vec<_>>();
    crate::publication::validate_destinations(inputs, &targets, output.overwrite)
}

fn publish(
    result: &ExtractionResult,
    output: &Output,
    bundle: Option<&Path>,
    stdout: &mut dyn Write,
) -> Result<(), ExtractionError> {
    let bytes = if output.raw {
        match result.data() {
            ExtractionData::Values(values) if values.len() == 1 => values[0].as_bytes().to_vec(),
            _ => return Err(options("Raw output requires exactly one flat string.")),
        }
    } else {
        let mut bytes = result.payload().to_vec();
        bytes.push(b'\n');
        bytes
    };
    let evidence = if let Some(path) = bundle {
        Some(Staged::prepare_with(path, output.overwrite, |writer| {
            crate::bundle::write(writer, result)
        })?)
    } else if let Some(path) = &output.receipt {
        Some(Staged::prepare(
            path,
            &{
                let mut bytes = result.receipt_payload()?.to_vec();
                bytes.push(b'\n');
                bytes
            },
            output.overwrite,
        )?)
    } else {
        None
    };
    let target = output
        .output
        .as_ref()
        .map(|path| Staged::prepare(path, &bytes, output.overwrite))
        .transpose()?;
    if let Some(evidence) = evidence {
        evidence.commit()?;
    }
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
