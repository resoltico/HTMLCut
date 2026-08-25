use super::*;
use crate::{MaxBytes, SourceInput, SourceRequest};
use htmlcut_tempdir::tempdir;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

#[test]
fn final_response_url_rejects_an_unsafe_response_uri_with_a_failed_get_trace() {
    let source = SourceRequest {
        input: SourceInput::Url {
            href: HttpUrl::parse("https://example.test/start").expect("source URL"),
        },
        base_url: None,
    };
    let error = final_response_url(&source, "https://example.test/start", 200, &[], "not a URL")
        .expect_err("unsafe final URI");

    assert_eq!(error.code, "SOURCE_LOAD_FAILED");
    assert_eq!(error.metadata.load_steps.len(), 1);
    assert_eq!(error.metadata.load_steps[0].action, SourceLoadAction::Get);
    assert_eq!(
        error.metadata.load_steps[0].outcome,
        SourceLoadOutcome::Failed
    );
    assert_eq!(error.metadata.load_steps[0].status, Some(200));
}

#[test]
fn url_loader_records_an_unsafe_final_response_uri_as_a_failed_get() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
    let address = listener.local_addr().expect("server address");
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept request");
        let mut request = [0u8; 1024];
        let request_bytes = stream.read(&mut request).expect("read request");
        assert!(request_bytes > 0, "request should not be empty");
        stream
            .write_all(
                b"HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Type: text/html\r\nContent-Length: 12\r\n\r\n<p>hello</p>",
            )
            .expect("write response");
    });
    let source_url = format!("http://{address}/page");
    let source = SourceRequest {
        input: SourceInput::Url {
            href: HttpUrl::parse(&source_url).expect("source URL"),
        },
        base_url: None,
    };
    FINAL_RESPONSE_URI_OVERRIDE.with(|override_uri| {
        *override_uri.borrow_mut() = Some("ftp://example.test/final".to_owned());
    });
    let href = match &source.input {
        SourceInput::Url { href } => href,
        _ => unreachable!("test source is a URL"),
    };
    let error = read_url_source(
        &source,
        href,
        &RuntimeOptions {
            fetch_preflight: FetchPreflightMode::GetOnly,
            ..RuntimeOptions::default()
        },
    )
    .expect_err("unsafe final URI");
    server.join().expect("join test server");

    assert_eq!(error.code, "SOURCE_LOAD_FAILED");
    assert_eq!(error.metadata.load_steps.len(), 2);
    assert_eq!(error.metadata.load_steps[1].action, SourceLoadAction::Get);
    assert_eq!(
        error.metadata.load_steps[1].outcome,
        SourceLoadOutcome::Failed
    );
    assert_eq!(error.metadata.load_steps[1].status, Some(200));
}

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
