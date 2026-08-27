#[cfg(test)]
use std::cell::RefCell;
use std::fs;
use std::io;
use std::path::Path;
use std::time::Duration;

use serde_json::json;
use ureq::ResponseExt;
use ureq::http::Response;
use ureq::tls::{Certificate, PemItem, RootCerts, TlsConfig, parse_pem};

use crate::contracts::{
    FetchPreflightMode, HttpUrl, RuntimeOptions, SourceKind, SourceLoadAction, SourceLoadOutcome,
    SourceLoadStep, SourceRequest, TlsTrustPolicy,
};
use crate::diagnostics::{DiagnosticCode, error_diagnostic};
use crate::format_byte_size;

use super::super::io::{UrlResponseContext, finish_url_source_from_reader};
use super::super::metadata::source_load_failure;
use super::super::{LoadedSource, SourceLoadFailure};

#[cfg(test)]
thread_local! {
    static FINAL_RESPONSE_URI_OVERRIDE: RefCell<Option<String>> = const { RefCell::new(None) };
}

pub(crate) fn read_url_source(
    source: &SourceRequest,
    href: &HttpUrl,
    runtime: &RuntimeOptions,
) -> Result<LoadedSource, SourceLoadFailure> {
    let source_value = href.to_string();
    let agent = build_http_agent(runtime).map_err(|diagnostic| {
        source_load_failure(
            source,
            SourceKind::Url,
            source_value.clone(),
            Vec::new(),
            diagnostic,
        )
    })?;
    let fetch_url = href.as_fetch_str();
    let mut load_steps = Vec::new();
    if runtime.fetch_preflight == FetchPreflightMode::HeadFirst {
        match agent.head(fetch_url).call() {
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
            Err(error) => {
                if head_error_requires_failure(&error) {
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
                load_steps.push(SourceLoadStep {
                    action: SourceLoadAction::HeadPreflight,
                    outcome: SourceLoadOutcome::Fallback,
                    status: None,
                    message: format!(
                        "HEAD preflight failed with {error}; HTMLCut fell back to GET."
                    ),
                });
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

    let mut response = agent.get(fetch_url).call().map_err(|error| {
        let mut failed_steps = load_steps.to_vec();
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
        let mut failed_steps = load_steps.to_vec();
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
            source_value.to_owned(),
            failed_steps,
            diagnostic,
        )
    })?;
    let response_uri = final_response_uri(&response);
    let final_url = final_response_url(
        source,
        &source_value,
        response.status().as_u16(),
        &load_steps,
        &response_uri,
    )?;
    let input_base_url = source
        .base_url
        .as_ref()
        .map(|base_url| base_url.to_string())
        .or_else(|| Some(final_url.to_string()));
    let response_status = response.status().as_u16();
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|header| header.to_str().ok())
        .map(str::to_owned);
    let get_success_message = if final_url.as_url() != href.as_url() {
        format!("Fetched the remote source with GET after redirect to {final_url}.")
    } else {
        "Fetched the remote source with GET.".to_owned()
    };
    let mut reader = response.body_mut().as_reader();
    finish_url_source_from_reader(
        source,
        runtime,
        UrlResponseContext {
            source_value,
            response_status,
            input_base_url,
            load_steps,
            content_type,
            get_success_message,
        },
        &mut reader,
    )
}

fn final_response_uri(response: &Response<ureq::Body>) -> String {
    #[cfg(test)]
    if let Some(response_uri) =
        FINAL_RESPONSE_URI_OVERRIDE.with(|override_uri| override_uri.borrow_mut().take())
    {
        return response_uri;
    }

    response.get_uri().to_string()
}

fn final_response_url(
    source: &SourceRequest,
    source_value: &str,
    response_status: u16,
    load_steps: &[SourceLoadStep],
    response_uri: &str,
) -> Result<HttpUrl, SourceLoadFailure> {
    HttpUrl::parse(response_uri).map_err(|_| {
        let mut failed_steps = load_steps.to_vec();
        failed_steps.push(SourceLoadStep {
            action: SourceLoadAction::Get,
            outcome: SourceLoadOutcome::Failed,
            status: Some(response_status),
            message: "GET reached a final URL that HTMLCut could not represent safely.".to_owned(),
        });
        source_load_failure(
            source,
            SourceKind::Url,
            source_value.to_owned(),
            failed_steps,
            error_diagnostic(
                DiagnosticCode::SourceLoadFailed,
                format!("Could not safely represent the final URL reached from {source_value}."),
                Some(json!({
                    "source": source_value,
                    "method": "GET",
                })),
            ),
        )
    })
}

pub(crate) fn build_http_agent(
    runtime: &RuntimeOptions,
) -> Result<ureq::Agent, crate::contracts::Diagnostic> {
    let tls_config = TlsConfig::builder()
        .root_certs(root_certs_for_policy(&runtime.tls_trust)?)
        .build();

    Ok(ureq::Agent::config_builder()
        .http_status_as_error(false)
        .tls_config(tls_config)
        .timeout_connect(Some(Duration::from_millis(
            runtime.fetch_connect_timeout_ms.get(),
        )))
        .timeout_global(Some(Duration::from_millis(runtime.fetch_timeout_ms.get())))
        .build()
        .into())
}

fn root_certs_for_policy(
    policy: &TlsTrustPolicy,
) -> Result<RootCerts, crate::contracts::Diagnostic> {
    match policy {
        TlsTrustPolicy::WebPki => Ok(RootCerts::WebPki),
        TlsTrustPolicy::Platform => Ok(RootCerts::PlatformVerifier),
        TlsTrustPolicy::CustomCaBundle { path } => load_custom_ca_bundle(path),
    }
}

fn load_custom_ca_bundle(path: &Path) -> Result<RootCerts, crate::contracts::Diagnostic> {
    let bundle = fs::read(path).map_err(|error| {
        error_diagnostic(
            DiagnosticCode::SourceLoadFailed,
            format!(
                "Could not read custom CA bundle {}: {error}",
                path.display()
            ),
            Some(json!({
                "tlsTrust": {
                    "kind": "custom-ca-bundle",
                    "path": path,
                }
            })),
        )
    })?;
    let certs = parse_pem(&bundle)
        .map(|item| match item {
            Ok(item) => Ok(if let PemItem::Certificate(cert) = item {
                Some(cert)
            } else {
                None
            }),
            Err(error) => Err(error),
        })
        .collect::<Result<Vec<Option<Certificate<'static>>>, _>>()
        .map_err(|error| {
            error_diagnostic(
                DiagnosticCode::SourceLoadFailed,
                format!(
                    "Custom CA bundle {} is not valid PEM certificate data: {error}",
                    path.display()
                ),
                Some(json!({
                    "tlsTrust": {
                        "kind": "custom-ca-bundle",
                        "path": path,
                    }
                })),
            )
        })?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();

    if certs.is_empty() {
        return Err(error_diagnostic(
            DiagnosticCode::SourceLoadFailed,
            format!(
                "Custom CA bundle {} does not contain any PEM certificates.",
                path.display()
            ),
            Some(json!({
                "tlsTrust": {
                    "kind": "custom-ca-bundle",
                    "path": path,
                }
            })),
        ));
    }

    Ok(RootCerts::new_with_certs(&certs))
}

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
        .and_then(|header| declared_content_length_exceedance(header, runtime.max_bytes.get()))
    {
        return Err(error_diagnostic(
            DiagnosticCode::SourceLoadFailed,
            format!(
                "{method} response exceeds {} limit.",
                format_byte_size(runtime.max_bytes.get())
            ),
            Some(json!({
                "source": source_value,
                "method": method,
                "contentLength": content_length,
                "maxBytes": runtime.max_bytes.get(),
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

fn declared_content_length_exceedance(header: &str, max_bytes: usize) -> Option<usize> {
    header
        .parse::<usize>()
        .ok()
        .filter(|content_length| *content_length > max_bytes)
}

fn head_error_allows_get_fallback(error: &ureq::Error) -> bool {
    match error {
        ureq::Error::Protocol(_) => true,
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

fn head_error_requires_failure(error: &ureq::Error) -> bool {
    !head_error_allows_get_fallback(error)
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
pub(crate) fn head_error_allows_get_fallback_for_tests(error: &ureq::Error) -> bool {
    head_error_allows_get_fallback(error)
}

#[cfg(test)]
pub(crate) fn head_error_requires_failure_for_tests(error: &ureq::Error) -> bool {
    head_error_requires_failure(error)
}

#[cfg(test)]
#[path = "../../tests/source/http_enabled.rs"]
mod tests;
