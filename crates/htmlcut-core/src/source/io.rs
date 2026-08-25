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
    decode_utf8(read_limited_bytes(reader, max_bytes, label)?, label)
}

pub(crate) fn read_limited_bytes(
    reader: &mut impl Read,
    max_bytes: usize,
    label: &str,
) -> Result<Vec<u8>, Diagnostic> {
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

    Ok(buffer)
}

fn decode_utf8(bytes: Vec<u8>, label: &str) -> Result<String, Diagnostic> {
    String::from_utf8(bytes).map_err(|error| {
        error_diagnostic(
            DiagnosticCode::SourceLoadFailed,
            format!("{label} is not valid UTF-8: {error}"),
            None,
        )
    })
}

#[cfg(feature = "http-client")]
pub(crate) struct UrlResponseContext {
    pub(crate) source_value: String,
    pub(crate) response_status: u16,
    pub(crate) input_base_url: Option<String>,
    pub(crate) load_steps: Vec<SourceLoadStep>,
    pub(crate) content_type: Option<String>,
    pub(crate) get_success_message: String,
}

#[cfg(feature = "http-client")]
pub(super) fn finish_url_source_from_reader(
    source: &SourceRequest,
    runtime: &RuntimeOptions,
    response: UrlResponseContext,
    reader: &mut impl Read,
) -> Result<LoadedSource, SourceLoadFailure> {
    let text = read_limited_bytes(reader, runtime.max_bytes.get(), "Response")
        .and_then(|bytes| {
            decode_http_response_text(
                bytes,
                runtime.max_bytes.get(),
                response.content_type.as_deref(),
            )
        })
        .map_err(|diagnostic| {
            let mut failed_steps = response.load_steps.clone();
            failed_steps.push(SourceLoadStep {
                action: SourceLoadAction::Get,
                outcome: SourceLoadOutcome::Failed,
                status: Some(response.response_status),
                message: format!(
                    "GET body read failed after status {}.",
                    response.response_status
                ),
            });
            source_load_failure(
                source,
                SourceKind::Url,
                response.source_value.clone(),
                failed_steps,
                diagnostic,
            )
        })?;

    let mut successful_steps = response.load_steps;
    successful_steps.push(SourceLoadStep {
        action: SourceLoadAction::Get,
        outcome: SourceLoadOutcome::Succeeded,
        status: Some(response.response_status),
        message: response.get_success_message,
    });

    Ok(loaded_source(
        SourceKind::Url,
        response.source_value,
        text,
        response.input_base_url,
        successful_steps,
    ))
}

#[cfg(feature = "http-client")]
fn decode_http_response_text(
    bytes: Vec<u8>,
    max_bytes: usize,
    content_type: Option<&str>,
) -> Result<String, Diagnostic> {
    let Some(charset) = declared_http_charset(content_type)? else {
        return decode_utf8(bytes, "Response");
    };
    let encoding =
        encoding_rs::Encoding::for_label_no_replacement(charset.as_bytes()).ok_or_else(|| {
            error_diagnostic(
                DiagnosticCode::SourceLoadFailed,
                format!("Response declares unsupported charset {charset:?}."),
                Some(json!({
                    "contentType": content_type,
                    "charset": charset,
                })),
            )
        })?;
    let (decoded, _, had_errors) = encoding.decode(&bytes);
    if had_errors {
        return Err(error_diagnostic(
            DiagnosticCode::SourceLoadFailed,
            format!("Response is not valid {} text.", encoding.name()),
            Some(json!({
                "contentType": content_type,
                "charset": charset,
            })),
        ));
    }

    let text = decoded.into_owned();
    if text.len() > max_bytes {
        return Err(error_diagnostic(
            DiagnosticCode::SourceLoadFailed,
            format!(
                "Response exceeds {} limit after character decoding.",
                format_byte_size(max_bytes)
            ),
            Some(json!({
                "contentType": content_type,
                "charset": charset,
                "maxBytes": max_bytes,
            })),
        ));
    }
    Ok(text)
}

#[cfg(feature = "http-client")]
fn declared_http_charset(content_type: Option<&str>) -> Result<Option<String>, Diagnostic> {
    let Some(content_type) = content_type else {
        return Ok(None);
    };

    for parameter in content_type.split(';').skip(1) {
        let parameter = parameter.trim();
        let Some((name, value)) = parameter.split_once('=') else {
            if parameter.eq_ignore_ascii_case("charset") {
                return Err(invalid_declared_charset(content_type));
            }
            continue;
        };
        if !name.trim().eq_ignore_ascii_case("charset") {
            continue;
        }

        let value = value.trim();
        let value = if value.len() >= 2
            && ((value.starts_with('\"') && value.ends_with('\"'))
                || (value.starts_with('\'') && value.ends_with('\'')))
        {
            &value[1..value.len() - 1]
        } else {
            value
        };
        let value = value.trim();
        if value.is_empty() {
            return Err(invalid_declared_charset(content_type));
        }
        return Ok(Some(value.to_owned()));
    }

    Ok(None)
}

#[cfg(feature = "http-client")]
fn invalid_declared_charset(content_type: &str) -> Diagnostic {
    error_diagnostic(
        DiagnosticCode::SourceLoadFailed,
        format!("Response declares an empty or malformed charset in {content_type:?}."),
        Some(json!({ "contentType": content_type })),
    )
}

#[cfg(all(test, feature = "http-client"))]
pub(crate) fn declared_http_charset_for_tests(
    content_type: Option<&str>,
) -> Result<Option<String>, Diagnostic> {
    declared_http_charset(content_type)
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
    response: UrlResponseContext,
    reader: &mut impl Read,
) -> Result<LoadedSource, SourceLoadFailure> {
    finish_url_source_from_reader(source, runtime, response, reader)
}

#[cfg(test)]
pub(crate) fn read_stdin_source_from_reader_for_tests(
    source: &SourceRequest,
    runtime: &RuntimeOptions,
    reader: &mut impl Read,
) -> Result<LoadedSource, SourceLoadFailure> {
    read_stdin_source_from_reader(source, runtime, reader)
}
