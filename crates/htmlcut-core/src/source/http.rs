#[cfg(feature = "http-client")]
use std::io;
#[cfg(feature = "http-client")]
use std::time::Duration;

use serde_json::json;
#[cfg(feature = "http-client")]
use ureq::http::Response;
#[cfg(feature = "http-client")]
use ureq::tls::{RootCerts, TlsConfig};

#[cfg(feature = "http-client")]
use crate::contracts::{FetchPreflightMode, SourceLoadAction, SourceLoadOutcome, SourceLoadStep};
use crate::contracts::{RuntimeOptions, SourceInput, SourceKind, SourceRequest};
use crate::diagnostics::{DiagnosticCode, error_diagnostic};
#[cfg(feature = "http-client")]
use crate::format_byte_size;

#[cfg(feature = "http-client")]
use super::io::finish_url_source_from_reader;
use super::metadata::source_load_failure;
use super::{LoadedSource, SourceLoadFailure};

#[cfg(not(feature = "http-client"))]
const HTTP_CLIENT_FEATURE: &str = "http-client";

#[cfg(feature = "http-client")]
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
                if !head_response.status().is_success() {
                    load_steps.push(SourceLoadStep {
                        action: SourceLoadAction::HeadPreflight,
                        outcome: SourceLoadOutcome::Fallback,
                        status: Some(head_response.status().as_u16()),
                        message: format!(
                            "HEAD returned {}, so HTMLCut treated the advisory preflight as non-authoritative and fell back to GET.",
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

#[cfg(not(feature = "http-client"))]
pub(crate) fn read_url_source(
    source: &SourceRequest,
    _runtime: &RuntimeOptions,
) -> Result<LoadedSource, SourceLoadFailure> {
    let SourceInput::Url { href } = &source.input else {
        unreachable!("read_url_source should only be called for URL sources");
    };
    let source_value = href.to_string();

    Err(source_load_failure(
        source,
        SourceKind::Url,
        source_value.clone(),
        Vec::new(),
        error_diagnostic(
            DiagnosticCode::SourceLoadFailed,
            format!(
                "Could not load {source_value}: this htmlcut-core build was compiled without the `{HTTP_CLIENT_FEATURE}` feature required for URL sources."
            ),
            Some(json!({
                "source": source_value,
                "requiredFeature": HTTP_CLIENT_FEATURE,
            })),
        ),
    ))
}

#[cfg(feature = "http-client")]
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

#[cfg(feature = "http-client")]
fn validate_url_response(
    response: &Response<ureq::Body>,
    runtime: &RuntimeOptions,
    source_value: &str,
    method: &str,
) -> Result<(), crate::contracts::Diagnostic> {
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

#[cfg(feature = "http-client")]
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

#[cfg(feature = "http-client")]
fn content_type_is_obviously_non_html(content_type: &str) -> bool {
    let normalized = content_type
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();

    !(normalized.is_empty() || normalized == "text/html" || normalized == "application/xhtml+xml")
}

#[cfg(all(test, feature = "http-client"))]
pub(crate) fn head_error_allows_get_fallback_for_tests(error: &ureq::Error) -> bool {
    head_error_allows_get_fallback(error)
}

#[cfg(all(test, feature = "http-client"))]
pub(crate) fn content_type_is_obviously_non_html_for_tests(content_type: &str) -> bool {
    content_type_is_obviously_non_html(content_type)
}
