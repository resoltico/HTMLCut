use std::fs;
use std::io::{self, Read};
use std::ops::Deref;
use std::time::Duration;

use serde_json::json;
use ureq::http::Response;
use ureq::tls::{RootCerts, TlsConfig};

use crate::contracts::{
    Diagnostic, FetchPreflightMode, RuntimeOptions, SourceInput, SourceKind, SourceLoadAction,
    SourceLoadOutcome, SourceLoadStep, SourceMetadata, SourceRequest,
};
use crate::diagnostics::{DiagnosticCode, error_diagnostic};
use crate::format_byte_size;

#[derive(Clone, Debug)]
pub(crate) struct LoadedSource {
    pub(crate) kind: SourceKind,
    pub(crate) value: String,
    pub(crate) text: String,
    pub(crate) bytes_read: usize,
    pub(crate) input_base_url: Option<String>,
    pub(crate) load_steps: Vec<SourceLoadStep>,
}

#[derive(Clone, Debug)]
pub(crate) struct SourceLoadFailure {
    pub(crate) metadata: Box<SourceMetadata>,
    pub(crate) diagnostic: Diagnostic,
}

impl Deref for SourceLoadFailure {
    type Target = Diagnostic;

    fn deref(&self) -> &Self::Target {
        &self.diagnostic
    }
}

impl SourceLoadFailure {
    pub(crate) fn into_parts(self) -> (SourceMetadata, Diagnostic) {
        (*self.metadata, self.diagnostic)
    }
}

pub(crate) fn load_source(
    source: &SourceRequest,
    runtime: &RuntimeOptions,
) -> Result<LoadedSource, SourceLoadFailure> {
    match &source.input {
        SourceInput::Memory { label, text } => {
            if text.len() > runtime.max_bytes {
                return Err(source_load_failure(
                    source,
                    SourceKind::Memory,
                    memory_label(label),
                    Vec::new(),
                    error_diagnostic(
                        DiagnosticCode::SourceLoadFailed,
                        format!(
                            "Preloaded source exceeds {} limit.",
                            format_byte_size(runtime.max_bytes)
                        ),
                        None,
                    ),
                ));
            }

            Ok(LoadedSource {
                kind: SourceKind::Memory,
                value: memory_label(label),
                bytes_read: text.len(),
                text: text.clone(),
                input_base_url: source.base_url.as_ref().map(ToString::to_string),
                load_steps: Vec::new(),
            })
        }
        SourceInput::Url { .. } => read_url_source(source, runtime),
        SourceInput::File { .. } => read_file_source(source, runtime),
        SourceInput::Stdin => read_stdin_source(source, runtime),
    }
}

pub(crate) fn read_url_source(
    source: &SourceRequest,
    runtime: &RuntimeOptions,
) -> Result<LoadedSource, SourceLoadFailure> {
    let SourceInput::Url { href } = &source.input else {
        unreachable!("read_url_source should only be called for URL sources");
    };
    let source_value = href.to_string();
    let agent = build_http_agent(runtime);
    let mut load_steps = Vec::new();
    if runtime.fetch_preflight == FetchPreflightMode::HeadFirst {
        match agent.head(&source_value).call() {
            Ok(head_response) => {
                if head_status_allows_get(&head_response) {
                    load_steps.push(SourceLoadStep {
                        action: SourceLoadAction::HeadPreflight,
                        outcome: SourceLoadOutcome::Fallback,
                        status: Some(head_response.status().as_u16()),
                        message: format!(
                            "HEAD returned {}, so HTMLCut fell back to GET.",
                            head_response.status().as_u16()
                        ),
                    });
                } else {
                    validate_url_response(&head_response, runtime, &source_value, "HEAD").map_err(
                        |diagnostic| {
                            let mut failed_steps = load_steps.clone();
                            failed_steps.push(SourceLoadStep {
                                action: SourceLoadAction::HeadPreflight,
                                outcome: SourceLoadOutcome::Failed,
                                status: Some(head_response.status().as_u16()),
                                message: format!(
                                    "HEAD preflight failed validation with status {}.",
                                    head_response.status().as_u16()
                                ),
                            });
                            source_load_failure(
                                source,
                                SourceKind::Url,
                                source_value.clone(),
                                failed_steps,
                                diagnostic,
                            )
                        },
                    )?;
                    load_steps.push(SourceLoadStep {
                        action: SourceLoadAction::HeadPreflight,
                        outcome: SourceLoadOutcome::Succeeded,
                        status: Some(head_response.status().as_u16()),
                        message: "HEAD preflight accepted the remote source.".to_owned(),
                    });
                }
            }
            Err(error) if head_error_allows_get_fallback(&error) => {
                load_steps.push(SourceLoadStep {
                    action: SourceLoadAction::HeadPreflight,
                    outcome: SourceLoadOutcome::Fallback,
                    status: None,
                    message: format!(
                        "HEAD preflight failed with {error}; HTMLCut fell back to GET."
                    ),
                });
            }
            Err(error) => {
                load_steps.push(SourceLoadStep {
                    action: SourceLoadAction::HeadPreflight,
                    outcome: SourceLoadOutcome::Failed,
                    status: None,
                    message: format!("HEAD preflight failed with {error}."),
                });
                return Err(source_load_failure(
                    source,
                    SourceKind::Url,
                    source_value.clone(),
                    load_steps,
                    error_diagnostic(
                        DiagnosticCode::SourceLoadFailed,
                        format!("Could not preflight {source_value} with HEAD: {error}"),
                        Some(json!({
                            "source": source_value,
                            "method": "HEAD",
                        })),
                    ),
                ));
            }
        }
    } else {
        load_steps.push(SourceLoadStep {
            action: SourceLoadAction::HeadPreflight,
            outcome: SourceLoadOutcome::Skipped,
            status: None,
            message: "Skipped HEAD preflight because --fetch-preflight get-only was requested."
                .to_owned(),
        });
    }

    let mut response = agent.get(&source_value).call().map_err(|error| {
        let mut failed_steps = load_steps.clone();
        failed_steps.push(SourceLoadStep {
            action: SourceLoadAction::Get,
            outcome: SourceLoadOutcome::Failed,
            status: None,
            message: format!("GET failed with {error}."),
        });
        source_load_failure(
            source,
            SourceKind::Url,
            source_value.clone(),
            failed_steps,
            error_diagnostic(
                DiagnosticCode::SourceLoadFailed,
                format!("Could not fetch {source_value}: {error}"),
                Some(json!({
                    "source": source_value,
                    "method": "GET",
                })),
            ),
        )
    })?;

    validate_url_response(&response, runtime, &source_value, "GET").map_err(|diagnostic| {
        let mut failed_steps = load_steps.clone();
        failed_steps.push(SourceLoadStep {
            action: SourceLoadAction::Get,
            outcome: SourceLoadOutcome::Failed,
            status: Some(response.status().as_u16()),
            message: format!(
                "GET failed validation with status {}.",
                response.status().as_u16()
            ),
        });
        source_load_failure(
            source,
            SourceKind::Url,
            source_value.clone(),
            failed_steps,
            diagnostic,
        )
    })?;
    load_steps.push(SourceLoadStep {
        action: SourceLoadAction::Get,
        outcome: SourceLoadOutcome::Succeeded,
        status: Some(response.status().as_u16()),
        message: "Fetched the remote source with GET.".to_owned(),
    });
    let input_base_url = source
        .base_url
        .as_ref()
        .map(ToString::to_string)
        .or(Some(source_value.clone()));
    let response_status = response.status().as_u16();
    let mut reader = response.body_mut().as_reader();
    finish_url_source_from_reader(
        source,
        runtime,
        &source_value,
        response_status,
        input_base_url,
        load_steps,
        &mut reader,
    )
}

pub(crate) fn build_http_agent(runtime: &RuntimeOptions) -> ureq::Agent {
    let tls_config = TlsConfig::builder()
        .root_certs(RootCerts::PlatformVerifier)
        .build();

    ureq::Agent::config_builder()
        .http_status_as_error(false)
        .tls_config(tls_config)
        .timeout_global(Some(Duration::from_millis(runtime.fetch_timeout_ms)))
        .build()
        .into()
}

pub(crate) fn read_file_source(
    source: &SourceRequest,
    runtime: &RuntimeOptions,
) -> Result<LoadedSource, SourceLoadFailure> {
    let SourceInput::File { path } = &source.input else {
        unreachable!("read_file_source should only be called for file sources");
    };
    let source_value = path.to_string_lossy().into_owned();
    let metadata = fs::metadata(path).map_err(|error| {
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

    if metadata.is_dir() {
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

    if metadata.len() as usize > runtime.max_bytes {
        return Err(source_load_failure(
            source,
            SourceKind::File,
            source_value.clone(),
            Vec::new(),
            error_diagnostic(
                DiagnosticCode::SourceLoadFailed,
                format!(
                    "File exceeds {} limit.",
                    format_byte_size(runtime.max_bytes)
                ),
                Some(json!({ "source": source_value })),
            ),
        ));
    }

    let bytes = fs::read(path).map_err(|error| {
        source_load_failure(
            source,
            SourceKind::File,
            source_value.clone(),
            Vec::new(),
            error_diagnostic(
                DiagnosticCode::SourceLoadFailed,
                format!("Could not read file {source_value}: {error}"),
                Some(json!({ "source": source_value })),
            ),
        )
    })?;

    let text = String::from_utf8(bytes).map_err(|error| {
        source_load_failure(
            source,
            SourceKind::File,
            source_value.clone(),
            Vec::new(),
            error_diagnostic(
                DiagnosticCode::SourceLoadFailed,
                format!("File is not valid UTF-8: {source_value}"),
                Some(json!({
                    "source": source_value,
                    "utf8_error": error.to_string(),
                })),
            ),
        )
    })?;

    let resolved_path = path
        .canonicalize()
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or(source_value);

    Ok(LoadedSource {
        kind: SourceKind::File,
        value: resolved_path,
        bytes_read: text.len(),
        text,
        input_base_url: source.base_url.as_ref().map(ToString::to_string),
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

        buffer.extend_from_slice(&chunk[..read]);
        if buffer.len() > max_bytes {
            return Err(error_diagnostic(
                DiagnosticCode::SourceLoadFailed,
                format!("{label} exceeds {} limit.", format_byte_size(max_bytes)),
                None,
            ));
        }
    }

    String::from_utf8(buffer).map_err(|error| {
        error_diagnostic(
            DiagnosticCode::SourceLoadFailed,
            format!("{label} is not valid UTF-8: {error}"),
            None,
        )
    })
}

fn finish_url_source_from_reader(
    source: &SourceRequest,
    runtime: &RuntimeOptions,
    source_value: &str,
    response_status: u16,
    input_base_url: Option<String>,
    load_steps: Vec<SourceLoadStep>,
    reader: &mut impl Read,
) -> Result<LoadedSource, SourceLoadFailure> {
    let text =
        read_limited_to_string(reader, runtime.max_bytes, "Response").map_err(|diagnostic| {
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
        })?;

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
        read_limited_to_string(reader, runtime.max_bytes, "Stdin").map_err(|diagnostic| {
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
        source.base_url.as_ref().map(ToString::to_string),
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

pub(crate) fn source_metadata(
    source: &LoadedSource,
    include_text: bool,
    effective_base_url: Option<String>,
) -> SourceMetadata {
    SourceMetadata {
        kind: source.kind,
        value: source.value.clone(),
        input_base_url: source.input_base_url.clone(),
        effective_base_url,
        bytes_read: source.bytes_read,
        load_steps: source.load_steps.clone(),
        text: include_text.then_some(source.text.clone()),
    }
}

fn source_load_failure(
    source: &SourceRequest,
    kind: SourceKind,
    value: String,
    load_steps: Vec<SourceLoadStep>,
    diagnostic: Diagnostic,
) -> SourceLoadFailure {
    let input_base_url = source
        .base_url
        .as_ref()
        .map(ToString::to_string)
        .or_else(|| matches!(kind, SourceKind::Url).then(|| value.clone()));

    SourceLoadFailure {
        metadata: Box::new(SourceMetadata {
            kind,
            value,
            input_base_url: input_base_url.clone(),
            effective_base_url: input_base_url,
            bytes_read: 0,
            load_steps,
            text: None,
        }),
        diagnostic,
    }
}

pub(crate) fn empty_source_metadata(source: &SourceRequest) -> SourceMetadata {
    let kind = source.kind();
    let value = source_locator_value(&source.input);
    let input_base_url = source
        .base_url
        .as_ref()
        .map(ToString::to_string)
        .or_else(|| matches!(source.input, SourceInput::Url { .. }).then(|| value.clone()));
    SourceMetadata {
        kind,
        value,
        input_base_url: input_base_url.clone(),
        effective_base_url: input_base_url,
        bytes_read: 0,
        load_steps: Vec::new(),
        text: None,
    }
}

fn validate_url_response(
    response: &Response<ureq::Body>,
    runtime: &RuntimeOptions,
    source_value: &str,
    method: &str,
) -> Result<(), Diagnostic> {
    let status = response.status();
    if !status.is_success() {
        return Err(error_diagnostic(
            DiagnosticCode::SourceLoadFailed,
            format!(
                "{method} {source_value} returned unexpected status {}.",
                status.as_u16()
            ),
            Some(json!({
                "source": source_value,
                "method": method,
                "status": status.as_u16(),
            })),
        ));
    }

    if let Some(content_length) = response
        .headers()
        .get("content-length")
        .and_then(|header| header.to_str().ok())
        .and_then(|header| header.parse::<usize>().ok())
        && content_length > runtime.max_bytes
    {
        return Err(error_diagnostic(
            DiagnosticCode::SourceLoadFailed,
            format!(
                "{method} response exceeds {} limit.",
                format_byte_size(runtime.max_bytes)
            ),
            Some(json!({
                "source": source_value,
                "method": method,
                "contentLength": content_length,
                "maxBytes": runtime.max_bytes,
            })),
        ));
    }

    if let Some(content_type) = response
        .headers()
        .get("content-type")
        .and_then(|header| header.to_str().ok())
        && content_type_is_obviously_non_html(content_type)
    {
        return Err(error_diagnostic(
            DiagnosticCode::SourceLoadFailed,
            format!("{method} {source_value} reported non-HTML content type {content_type}.",),
            Some(json!({
                "source": source_value,
                "method": method,
                "contentType": content_type,
            })),
        ));
    }

    Ok(())
}

fn head_status_allows_get(response: &Response<ureq::Body>) -> bool {
    matches!(response.status().as_u16(), 405 | 501)
}

fn head_error_allows_get_fallback(error: &ureq::Error) -> bool {
    match error {
        ureq::Error::Protocol(_) | ureq::Error::ConnectionFailed => true,
        ureq::Error::Io(io_error) => matches!(
            io_error.kind(),
            io::ErrorKind::ConnectionAborted
                | io::ErrorKind::ConnectionReset
                | io::ErrorKind::BrokenPipe
                | io::ErrorKind::UnexpectedEof
        ),
        _ => false,
    }
}

#[cfg(test)]
pub(crate) fn head_error_allows_get_fallback_for_tests(error: &ureq::Error) -> bool {
    head_error_allows_get_fallback(error)
}

#[cfg(test)]
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

fn content_type_is_obviously_non_html(content_type: &str) -> bool {
    let normalized = content_type
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();

    !(normalized.is_empty() || normalized == "text/html" || normalized == "application/xhtml+xml")
}

#[cfg(test)]
pub(crate) fn content_type_is_obviously_non_html_for_tests(content_type: &str) -> bool {
    content_type_is_obviously_non_html(content_type)
}

#[cfg(test)]
pub(crate) fn read_stdin_source_from_reader_for_tests(
    source: &SourceRequest,
    runtime: &RuntimeOptions,
    reader: &mut impl Read,
) -> Result<LoadedSource, SourceLoadFailure> {
    read_stdin_source_from_reader(source, runtime, reader)
}

fn source_locator_value(input: &SourceInput) -> String {
    match input {
        SourceInput::Url { href } => href.to_string(),
        SourceInput::File { path } => path.to_string_lossy().into_owned(),
        SourceInput::Stdin => "-".to_owned(),
        SourceInput::Memory { label, .. } => memory_label(label),
    }
}

fn memory_label(label: &str) -> String {
    if label.trim().is_empty() {
        "memory".to_owned()
    } else {
        label.to_owned()
    }
}
