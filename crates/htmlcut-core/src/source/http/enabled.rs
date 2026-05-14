use std::fs;
use std::io;
use std::path::Path;
use std::time::Duration;

use serde_json::json;
use ureq::http::Response;
use ureq::tls::{Certificate, PemItem, RootCerts, TlsConfig, parse_pem};

use crate::contracts::{
    FetchPreflightMode, HttpUrl, RuntimeOptions, SourceKind, SourceLoadAction, SourceLoadOutcome,
    SourceLoadStep, SourceRequest, TlsTrustPolicy,
};
use crate::diagnostics::{DiagnosticCode, error_diagnostic};
use crate::format_byte_size;

use super::super::io::finish_url_source_from_reader;
use super::super::metadata::source_load_failure;
use super::super::{LoadedSource, SourceLoadFailure};

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

    let mut response = agent.get(fetch_url).call().map_err(|error| {
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
        .map(|base_url| base_url.to_string())
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
        .and_then(|header| header.parse::<usize>().ok())
        && content_length > runtime.max_bytes.get()
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
mod tests {
    use super::*;
    use crate::{MaxBytes, SourceInput, SourceRequest};
    use htmlcut_tempdir::tempdir;

    #[test]
    fn tls_trust_policy_helpers_cover_platform_and_custom_bundle_paths() {
        assert!(matches!(
            root_certs_for_policy(&TlsTrustPolicy::Platform).expect("platform roots"),
            RootCerts::PlatformVerifier
        ));

        let tempdir = tempdir().expect("tempdir");
        let missing_bundle = tempdir.path().join("missing-roots.pem");
        let missing_error = load_custom_ca_bundle(&missing_bundle).expect_err("missing bundle");
        assert!(
            missing_error
                .message
                .contains("Could not read custom CA bundle")
        );

        let invalid_bundle = tempdir.path().join("invalid-roots.pem");
        fs::write(
            &invalid_bundle,
            "-----BEGIN CERTIFICATE-----\n%%%not-base64%%%\n-----END CERTIFICATE-----\n",
        )
        .expect("write invalid bundle");
        let invalid_error = load_custom_ca_bundle(&invalid_bundle).expect_err("invalid bundle");
        assert!(
            invalid_error
                .message
                .contains("is not valid PEM certificate data")
        );

        let empty_bundle = tempdir.path().join("empty-roots.pem");
        fs::write(
            &empty_bundle,
            "-----BEGIN PRIVATE KEY-----\nAA==\n-----END PRIVATE KEY-----\n",
        )
        .expect("write empty bundle");
        let empty_error = load_custom_ca_bundle(&empty_bundle).expect_err("empty bundle");
        assert!(
            empty_error
                .message
                .contains("does not contain any PEM certificates")
        );

        let public_key_bundle = tempdir.path().join("public-key.pem");
        fs::write(
            &public_key_bundle,
            "-----BEGIN PUBLIC KEY-----\nAA==\n-----END PUBLIC KEY-----\n",
        )
        .expect("write public key bundle");
        let public_key_error =
            load_custom_ca_bundle(&public_key_bundle).expect_err("public key only bundle");
        assert!(
            public_key_error
                .message
                .contains("does not contain any PEM certificates")
        );

        let valid_bundle = tempdir.path().join("valid-roots.pem");
        fs::write(
            &valid_bundle,
            "-----BEGIN CERTIFICATE-----\nAA==\n-----END CERTIFICATE-----\n",
        )
        .expect("write valid bundle");
        load_custom_ca_bundle(&valid_bundle).expect("valid bundle roots");

        let custom_agent = build_http_agent(&RuntimeOptions {
            tls_trust: TlsTrustPolicy::CustomCaBundle {
                path: valid_bundle.clone(),
            },
            ..RuntimeOptions::default()
        })
        .expect("custom bundle agent");
        assert_eq!(
            custom_agent.config().timeouts().global,
            Some(Duration::from_millis(
                RuntimeOptions::default().fetch_timeout_ms.get()
            ))
        );
    }

    #[test]
    fn url_loading_reports_custom_ca_bundle_build_failures_without_leaking_secrets() {
        let tempdir = tempdir().expect("tempdir");
        let href = HttpUrl::parse("https://example.com/private?sig=secret#frag").expect("http url");
        let source = SourceRequest {
            input: SourceInput::Url { href: href.clone() },
            base_url: None,
        };
        let error = read_url_source(
            &source,
            &href,
            &RuntimeOptions {
                max_bytes: MaxBytes::new(1024).expect("max bytes"),
                tls_trust: TlsTrustPolicy::CustomCaBundle {
                    path: tempdir.path().join("missing-roots.pem"),
                },
                ..RuntimeOptions::default()
            },
        )
        .expect_err("custom bundle build failure");

        assert_eq!(error.metadata.kind, SourceKind::Url);
        assert_eq!(
            error.metadata.value,
            "https://example.com/private?[redacted]"
        );
        assert!(error.message.contains("Could not read custom CA bundle"));
        assert!(!error.message.contains("sig=secret"));
    }
}
