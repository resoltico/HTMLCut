//! Binary composition, with private deterministic input/output seams.

use std::ffi::OsString;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use clap::Parser;
use htmlcut_core::{
    CompiledPlan, ExtractionError, ExtractionResult, PreparationLimits, PreparedDocument,
};

use crate::command::{Cli, Operation, Output};
use crate::input::{RunSpec, SourceSpec, options};
use crate::publication::{MAX_AUDIT_BYTES, MAX_OUTPUT_BYTES, Staged};

// Fixed metadata/proposal envelope allowance, expressed as its byte count.
const METADATA_BYTES: usize = 16_384;

#[cfg(test)]
thread_local! {
    static CANONICAL_PATH_RESPONSE: std::cell::RefCell<Option<PathBuf>> = const { std::cell::RefCell::new(None) };
}

fn canonical_path(path: &str) -> std::io::Result<PathBuf> {
    #[cfg(test)]
    if let Some(response) = CANONICAL_PATH_RESPONSE.with_borrow_mut(Option::take) {
        return Ok(response);
    }
    std::fs::canonicalize(path)
}

fn saved_path_utf8(path: &Path) -> Result<String, ExtractionError> {
    path.to_str()
        .map(str::to_owned)
        .ok_or_else(|| options("Saved file paths must be valid UTF-8."))
}

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
            return if stdout
                .write_all(error.to_string().as_bytes())
                .and_then(|_| stdout.flush())
                .is_ok()
            {
                0
            } else {
                report(crate::publication::failure(), stderr)
            };
        }
        Err(_) => {
            return report(
                options("Invalid command options; use --help for the closed command vocabulary."),
                stderr,
            );
        }
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
        Operation::Describe { operation } => emit_json(
            &crate::operation_metadata::describe(operation.as_deref())?,
            METADATA_BYTES,
            stdout,
        ),
        Operation::Schema { name } => {
            let schema = if name == "htmlcut.run" {
                schemars::schema_for!(RunSpec).as_value().clone()
            } else {
                htmlcut_core::schema(&name)?
            };
            emit_json(&schema, crate::input::MAX_CONFIG_BYTES, stdout)
        }
        Operation::Extract(arguments) => {
            let plan = arguments.extraction_plan()?;
            let compiled = CompiledPlan::compile(&plan)?;
            let mut source = arguments.source.source.specification()?;
            source.validate()?;
            if arguments.save_run.is_some() && arguments.source.source.url.is_some() {
                return Err(options(
                    "Automatic URL persistence requires --url-env instead of a transient --url.",
                ));
            }
            if arguments.save_run.is_some()
                && arguments.source.base_url.as_ref().is_some_and(|base| {
                    url::Url::parse(base)
                        .is_ok_and(|url| url.query().is_some() || url.fragment().is_some())
                })
            {
                return Err(options(
                    "Automatic persistence cannot store a transient query-bearing base URL; author a public run explicitly or use the source's runtime base.",
                ));
            }
            let mut inputs = Vec::new();
            if let Some(path) = &arguments.source.source.file {
                inputs.push(path.clone());
            }
            if let Some(path) = &arguments.plan {
                inputs.push(path.clone());
            }
            // Capture the replay path before reading/executing, avoiding a second resolution
            // after a successful immutable snapshot has already been extracted.
            if arguments.save_run.is_some()
                && let SourceSpec::File { path: source_path } = &mut source
            {
                *source_path = saved_path_utf8(
                    &canonical_path(source_path).map_err(|_| crate::input::acquisition())?,
                )?;
            }
            validate(&inputs, &arguments.output, arguments.save_run.as_deref())?;
            let snapshot = source.acquire(
                stdin,
                arguments.source.encoding.as_deref(),
                arguments.source.base_url.as_deref(),
                Path::new("."),
            )?;
            let result = PreparedDocument::new(snapshot, PreparationLimits::default())?
                .execute(&compiled)?;
            let saved = if let Some(path) = &arguments.save_run {
                let run = RunSpec {
                    schema: "htmlcut.run".into(),
                    version: 1,
                    source,
                    plan: compiled.plan().clone(),
                    encoding: arguments.source.encoding,
                    base_url: arguments.source.base_url,
                };
                Some(Staged::prepare(
                    path,
                    &crate::publication::json(&run, crate::input::MAX_CONFIG_BYTES)?,
                    arguments.output.overwrite,
                )?)
            } else {
                None
            };
            publish(&result, &compiled, &arguments.output, saved, stdout)
        }
        Operation::Run(arguments) => {
            let run = RunSpec::read(&arguments.file)?;
            let compiled = CompiledPlan::compile(&run.plan)?;
            let directory = arguments.file.parent().unwrap_or_else(|| Path::new("."));
            let mut inputs = vec![arguments.file.clone()];
            if let SourceSpec::File { path } = &run.source {
                inputs.push(directory.join(path));
            }
            validate(&inputs, &arguments.output, None)?;
            let snapshot = run.source.acquire(
                stdin,
                run.encoding.as_deref(),
                run.base_url.as_deref(),
                directory,
            )?;
            let result = PreparedDocument::new(snapshot, PreparationLimits::default())?
                .execute(&compiled)?;
            publish(&result, &compiled, &arguments.output, None, stdout)
        }
        Operation::Inspect(arguments) => {
            if (arguments.cursor.is_some() || arguments.propose.is_some())
                && (arguments.source.source.url.is_some()
                    || arguments.source.source.url_env.is_some())
            {
                return Err(options(
                    "Cursor inspection requires one saved snapshot; live URLs cannot be refetched with a cursor.",
                ));
            }
            let preview_plan = arguments
                .preview_plan
                .as_ref()
                .map(|path| {
                    let plan = htmlcut_core::ExtractionPlan::from_json(&crate::input::read_file(
                        path,
                        crate::input::MAX_CONFIG_BYTES,
                    )?)?;
                    CompiledPlan::compile(&plan)
                })
                .transpose()?;
            let snapshot = arguments.source.source.specification()?.acquire(
                stdin,
                arguments.source.encoding.as_deref(),
                arguments.source.base_url.as_deref(),
                Path::new("."),
            )?;
            let document = PreparedDocument::new(snapshot, PreparationLimits::default())?;
            if let Some(handle) = arguments.propose {
                let proposal = document.propose(&handle, arguments.page_size)?;
                return emit_json(&proposal, METADATA_BYTES, stdout);
            }
            if let Some(plan) = preview_plan {
                let preview = document.preview(&plan, 1024)?;
                return emit_json(&preview, 256 * 1024, stdout);
            }
            let value = document.inspect(arguments.page_size, arguments.cursor.as_deref())?;
            emit_json(&value, 512 * 1024, stdout)
        }
    }
}

fn validate(
    inputs: &[PathBuf],
    output: &Output,
    save_run: Option<&Path>,
) -> Result<(), ExtractionError> {
    let targets = output
        .output
        .iter()
        .chain(output.audit.iter())
        .cloned()
        .chain(save_run.map(Path::to_path_buf))
        .collect::<Vec<_>>();
    crate::publication::validate_destinations(inputs, &targets, output.overwrite)
}

fn publish(
    result: &ExtractionResult,
    compiled: &CompiledPlan,
    output: &Output,
    saved: Option<Staged>,
    stdout: &mut dyn Write,
) -> Result<(), ExtractionError> {
    publish_staged(result, compiled, output, saved, stdout)
        .map_err(|error| error.with_result(result))
}

fn publish_staged(
    result: &ExtractionResult,
    compiled: &CompiledPlan,
    output: &Output,
    saved: Option<Staged>,
    stdout: &mut dyn Write,
) -> Result<(), ExtractionError> {
    let bytes = if output.raw {
        if result.values.len() != 1 {
            return Err(options("Raw output requires exactly one resulting value."));
        }
        result.values[0].as_bytes().to_vec()
    } else {
        crate::publication::json(result, MAX_OUTPUT_BYTES)?
    };
    let audit = if let Some(path) = &output.audit {
        let evidence = crate::evidence::Evidence {
            fields: &output.audit_field,
            result,
            plan: compiled.plan(),
        };
        Some(Staged::prepare(
            path,
            &crate::publication::json_stream(&evidence, MAX_AUDIT_BYTES)?,
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
    if let Some(saved) = saved {
        saved.commit()?;
    }
    if let Some(audit) = audit {
        audit.commit()?;
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
        .map_err(|_| crate::publication::failure())
}

// All metadata routes stage bounded JSON before touching stdout.
fn emit_json(
    value: &impl serde::Serialize,
    maximum: usize,
    stdout: &mut dyn Write,
) -> Result<(), ExtractionError> {
    emit(crate::publication::json(value, maximum)?, stdout)
}

#[cfg(test)]
#[path = "tests/app_faults.rs"]
mod fault_tests;
