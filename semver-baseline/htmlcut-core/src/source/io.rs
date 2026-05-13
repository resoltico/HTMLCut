use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use serde_json::json;

use crate::contracts::{Diagnostic, RuntimeOptions, SourceKind, SourceLoadStep, SourceRequest};
#[cfg(feature = "http-client")]
use crate::contracts::{SourceLoadAction, SourceLoadOutcome};
use crate::diagnostics::{DiagnosticCode, error_diagnostic};
use crate::format_byte_size;

use super::metadata::source_load_failure;
use super::{LoadedSource, SourceLoadFailure};

pub(crate) fn read_file_source(
    source: &SourceRequest,
    path: &Path,
    runtime: &RuntimeOptions,
) -> Result<LoadedSource, SourceLoadFailure> {
    let source_value = path.to_string_lossy().into_owned();
    if path.is_dir() {
        return Err(source_load_failure(
            source,
            SourceKind::File,
            source_value.clone(),
            Vec::new(),
            error_diagnostic(
                DiagnosticCode::SourceLoadFailed,
                format!("Input path is a directory, not a file: {source_value}"),
                Some(json!({ "source": source_value, "kind": "directory" })),
            ),
        ));
    }

    let mut file = File::open(path).map_err(|error| {
        source_load_failure(
            source,
            SourceKind::File,
            source_value.clone(),
            Vec::new(),
            error_diagnostic(
                DiagnosticCode::SourceLoadFailed,
                format!("Could not access file {source_value}: {error}"),
                Some(json!({ "source": source_value })),
            ),
        )
    })?;

    let text = read_limited_to_string(&mut file, runtime.max_bytes.get(), "File").map_err(
        |diagnostic| {
            source_load_failure(
                source,
                SourceKind::File,
                source_value.clone(),
                Vec::new(),
                diagnostic,
            )
        },
    )?;

    let resolved_path = path
        .canonicalize()
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or(source_value);

    Ok(LoadedSource {
        kind: SourceKind::File,
        value: resolved_path,
        bytes_read: text.len(),
        text,
        input_base_url: source
            .base_url
            .as_ref()
            .map(|base_url| base_url.to_string()),
        load_steps: Vec::new(),
    })
}

pub(crate) fn read_stdin_source(
    source: &SourceRequest,
    runtime: &RuntimeOptions,
) -> Result<LoadedSource, SourceLoadFailure> {
    let mut stdin = io::stdin().lock();
    read_stdin_source_from_reader(source, runtime, &mut stdin)
}

pub(crate) fn read_limited_to_string(
    reader: &mut impl Read,
    max_bytes: usize,
    label: &str,
) -> Result<String, Diagnostic> {
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 8192];

    loop {
        let read = reader.read(&mut chunk).map_err(|error| {
            error_diagnostic(
                DiagnosticCode::SourceLoadFailed,
                format!("Could not read {label}: {error}"),
                None,
            )
        })?;

        if read == 0 {
            break;
        }

        if buffer.len() + read > max_bytes {
            return Err(error_diagnostic(
                DiagnosticCode::SourceLoadFailed,
                format!("{label} exceeds {} limit.", format_byte_size(max_bytes)),
                None,
            ));
        }
        buffer.extend_from_slice(&chunk[..read]);
    }

    String::from_utf8(buffer).map_err(|error| {
        error_diagnostic(
            DiagnosticCode::SourceLoadFailed,
            format!("{label} is not valid UTF-8: {error}"),
            None,
        )
    })
}

#[cfg(feature = "http-client")]
pub(super) fn finish_url_source_from_reader(
    source: &SourceRequest,
    runtime: &RuntimeOptions,
    source_value: &str,
    response_status: u16,
    input_base_url: Option<String>,
    load_steps: Vec<SourceLoadStep>,
    reader: &mut impl Read,
) -> Result<LoadedSource, SourceLoadFailure> {
    let text = read_limited_to_string(reader, runtime.max_bytes.get(), "Response").map_err(
        |diagnostic| {
            let mut failed_steps = load_steps.clone();
            failed_steps.push(SourceLoadStep {
                action: SourceLoadAction::Get,
                outcome: SourceLoadOutcome::Failed,
                status: Some(response_status),
                message: format!("GET body read failed after status {response_status}."),
            });
            source_load_failure(
                source,
                SourceKind::Url,
                source_value.to_owned(),
                failed_steps,
                diagnostic,
            )
        },
    )?;

    Ok(loaded_source(
        SourceKind::Url,
        source_value.to_owned(),
        text,
        input_base_url,
        load_steps,
    ))
}

fn read_stdin_source_from_reader(
    source: &SourceRequest,
    runtime: &RuntimeOptions,
    reader: &mut impl Read,
) -> Result<LoadedSource, SourceLoadFailure> {
    let text =
        read_limited_to_string(reader, runtime.max_bytes.get(), "Stdin").map_err(|diagnostic| {
            source_load_failure(
                source,
                SourceKind::Stdin,
                "-".to_owned(),
                Vec::new(),
                diagnostic,
            )
        })?;

    Ok(loaded_source(
        SourceKind::Stdin,
        "-".to_owned(),
        text,
        source
            .base_url
            .as_ref()
            .map(|base_url| base_url.to_string()),
        Vec::new(),
    ))
}

fn loaded_source(
    kind: SourceKind,
    value: String,
    text: String,
    input_base_url: Option<String>,
    load_steps: Vec<SourceLoadStep>,
) -> LoadedSource {
    LoadedSource {
        kind,
        value,
        bytes_read: text.len(),
        text,
        input_base_url,
        load_steps,
    }
}

#[cfg(all(test, feature = "http-client"))]
pub(crate) fn finish_url_source_from_reader_for_tests(
    source: &SourceRequest,
    runtime: &RuntimeOptions,
    source_value: &str,
    response_status: u16,
    input_base_url: Option<String>,
    load_steps: Vec<SourceLoadStep>,
    reader: &mut impl Read,
) -> Result<LoadedSource, SourceLoadFailure> {
    finish_url_source_from_reader(
        source,
        runtime,
        source_value,
        response_status,
        input_base_url,
        load_steps,
        reader,
    )
}

#[cfg(test)]
pub(crate) fn read_stdin_source_from_reader_for_tests(
    source: &SourceRequest,
    runtime: &RuntimeOptions,
    reader: &mut impl Read,
) -> Result<LoadedSource, SourceLoadFailure> {
    read_stdin_source_from_reader(source, runtime, reader)
}
