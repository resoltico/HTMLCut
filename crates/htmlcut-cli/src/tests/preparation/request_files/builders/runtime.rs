use super::*;

#[test]
fn request_file_runtime_builders_cover_get_only_and_missing_input() {
    let fixture = request_file_fixture();

    let get_only_runtime = build_runtime(&SourceArgs {
        input: Some(fixture.input.clone()),
        base_url: None,
        max_bytes: DEFAULT_MAX_BYTES.to_string(),
        fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
        fetch_connect_timeout_ms: htmlcut_core::DEFAULT_FETCH_CONNECT_TIMEOUT_MS,
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
            base_url: None,
            max_bytes: DEFAULT_MAX_BYTES.to_string(),
            fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
            fetch_connect_timeout_ms: htmlcut_core::DEFAULT_FETCH_CONNECT_TIMEOUT_MS,
            fetch_preflight: CliFetchPreflightMode::HeadFirst,
        })
        .expect_err("missing input")
        .code,
        "CLI_REQUIRED_PARAMETER_MISSING"
    );
}
