use super::*;

#[test]
fn request_file_runtime_builders_cover_get_only_and_missing_input() {
    let fixture = request_file_fixture();

    let get_only_runtime = build_runtime(&SourceArgs {
        input: Some(fixture.input.clone()),
        input_html: None,
        base_url: None,
        max_bytes: DEFAULT_MAX_BYTES.to_string(),
        fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
        fetch_connect_timeout_ms: htmlcut_core::DEFAULT_FETCH_CONNECT_TIMEOUT_MS,
        tls_trust: CliTlsTrustMode::WebPki,
        tls_ca_bundle: None,
        fetch_preflight: CliFetchPreflightMode::GetOnly,
    })
    .expect("runtime");
    assert_eq!(
        get_only_runtime.fetch_preflight,
        FetchPreflightMode::GetOnly
    );

    assert_eq!(
        build_source_request(&SourceArgs {
            input: None,
            input_html: None,
            base_url: None,
            max_bytes: DEFAULT_MAX_BYTES.to_string(),
            fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
            fetch_connect_timeout_ms: htmlcut_core::DEFAULT_FETCH_CONNECT_TIMEOUT_MS,
            tls_trust: CliTlsTrustMode::WebPki,
            tls_ca_bundle: None,
            fetch_preflight: CliFetchPreflightMode::HeadFirst,
        })
        .expect_err("missing input")
        .code,
        "CLI_REQUIRED_PARAMETER_MISSING"
    );
    assert_eq!(
        build_source_request(&SourceArgs {
            input: None,
            input_html: Some("<article>Hello</article>".to_owned()),
            base_url: None,
            max_bytes: DEFAULT_MAX_BYTES.to_string(),
            fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
            fetch_connect_timeout_ms: htmlcut_core::DEFAULT_FETCH_CONNECT_TIMEOUT_MS,
            tls_trust: CliTlsTrustMode::WebPki,
            tls_ca_bundle: None,
            fetch_preflight: CliFetchPreflightMode::HeadFirst,
        })
        .expect("inline html source")
        .input,
        htmlcut_core::SourceInput::Memory {
            label: "inline-html".to_owned(),
            text: "<article>Hello</article>".to_owned(),
        }
    );

    let platform_runtime = build_runtime(&SourceArgs {
        input: Some(fixture.input.clone()),
        input_html: None,
        base_url: None,
        max_bytes: DEFAULT_MAX_BYTES.to_string(),
        fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
        fetch_connect_timeout_ms: htmlcut_core::DEFAULT_FETCH_CONNECT_TIMEOUT_MS,
        tls_trust: CliTlsTrustMode::Platform,
        tls_ca_bundle: None,
        fetch_preflight: CliFetchPreflightMode::HeadFirst,
    })
    .expect("platform runtime");
    assert_eq!(platform_runtime.tls_trust, TlsTrustPolicy::Platform);

    let custom_bundle = fixture.tempdir.path().join("roots.pem");
    let custom_runtime = build_runtime(&SourceArgs {
        input: Some(fixture.input.clone()),
        input_html: None,
        base_url: None,
        max_bytes: DEFAULT_MAX_BYTES.to_string(),
        fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
        fetch_connect_timeout_ms: htmlcut_core::DEFAULT_FETCH_CONNECT_TIMEOUT_MS,
        tls_trust: CliTlsTrustMode::CustomCaBundle,
        tls_ca_bundle: Some(custom_bundle.clone()),
        fetch_preflight: CliFetchPreflightMode::HeadFirst,
    })
    .expect("custom runtime");
    assert_eq!(
        custom_runtime.tls_trust,
        TlsTrustPolicy::CustomCaBundle {
            path: custom_bundle.clone(),
        }
    );

    assert_eq!(
        build_runtime(&SourceArgs {
            input: Some(fixture.input.clone()),
            input_html: None,
            base_url: None,
            max_bytes: DEFAULT_MAX_BYTES.to_string(),
            fetch_timeout_ms: 0,
            fetch_connect_timeout_ms: htmlcut_core::DEFAULT_FETCH_CONNECT_TIMEOUT_MS,
            tls_trust: CliTlsTrustMode::WebPki,
            tls_ca_bundle: None,
            fetch_preflight: CliFetchPreflightMode::HeadFirst,
        })
        .expect_err("zero fetch timeout")
        .code,
        "CLI_FETCH_TIMEOUT_INVALID"
    );
    assert_eq!(
        build_runtime(&SourceArgs {
            input: Some(fixture.input.clone()),
            input_html: None,
            base_url: None,
            max_bytes: DEFAULT_MAX_BYTES.to_string(),
            fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
            fetch_connect_timeout_ms: 0,
            tls_trust: CliTlsTrustMode::WebPki,
            tls_ca_bundle: None,
            fetch_preflight: CliFetchPreflightMode::HeadFirst,
        })
        .expect_err("zero connect timeout")
        .code,
        "CLI_FETCH_CONNECT_TIMEOUT_INVALID"
    );
    assert_eq!(
        build_runtime(&SourceArgs {
            input: Some(fixture.input.clone()),
            input_html: None,
            base_url: None,
            max_bytes: DEFAULT_MAX_BYTES.to_string(),
            fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
            fetch_connect_timeout_ms: htmlcut_core::DEFAULT_FETCH_CONNECT_TIMEOUT_MS,
            tls_trust: CliTlsTrustMode::CustomCaBundle,
            tls_ca_bundle: None,
            fetch_preflight: CliFetchPreflightMode::HeadFirst,
        })
        .expect_err("custom trust requires bundle")
        .code,
        "CLI_TLS_CA_BUNDLE_REQUIRED"
    );
    assert_eq!(
        build_runtime(&SourceArgs {
            input: Some(fixture.input.clone()),
            input_html: None,
            base_url: None,
            max_bytes: DEFAULT_MAX_BYTES.to_string(),
            fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
            fetch_connect_timeout_ms: htmlcut_core::DEFAULT_FETCH_CONNECT_TIMEOUT_MS,
            tls_trust: CliTlsTrustMode::Platform,
            tls_ca_bundle: Some(custom_bundle),
            fetch_preflight: CliFetchPreflightMode::HeadFirst,
        })
        .expect_err("bundle conflicts with platform trust")
        .code,
        "CLI_TLS_CA_BUNDLE_CONFLICT"
    );

    assert_eq!(
        validate_base_url(Some("https://alice:secret@example.com/docs"))
            .expect_err("userinfo must be rejected")
            .code,
        "CLI_BASE_URL_INVALID"
    );
    assert_eq!(
        validate_base_url(Some("::not-a-url::"))
            .expect_err("malformed base url")
            .code,
        "CLI_BASE_URL_INVALID"
    );
}
