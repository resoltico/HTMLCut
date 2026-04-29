use super::*;

#[test]
fn file_and_url_loading_cover_successful_non_error_branches() {
    let tempdir = htmlcut_tempdir::tempdir().expect("tempdir");
    let file_path = tempdir.path().join("input.html");
    std::fs::write(&file_path, "<article>Hello</article>").expect("write html");
    let loaded = read_file_source(
        &file_source(&file_path)
            .with_base_url(Url::parse("https://example.com/base/").expect("base url")),
        &RuntimeOptions::default(),
    )
    .expect("file source");
    assert_eq!(
        loaded.input_base_url.as_deref(),
        Some("https://example.com/base/")
    );

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind success server");
    let address = listener.local_addr().expect("server addr");
    let server = thread::spawn(move || {
        for _ in 0..2 {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut request_buffer = [0u8; 512];
            let read = stream.read(&mut request_buffer).expect("read request");
            let request = String::from_utf8_lossy(&request_buffer[..read]);
            let method = request
                .lines()
                .next()
                .expect("request line")
                .split_whitespace()
                .next()
                .expect("request method");
            let body = "<html><body>Hello</body></html>";
            let response = if method == "HEAD" {
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n",
                    body.len()
                )
            } else {
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{}",
                    body.len(),
                    body
                )
            };
            stream
                .write_all(response.as_bytes())
                .expect("write response");
        }
    });
    let url = format!("http://{address}");
    let loaded_url =
        read_url_source(&url_source(&url), &RuntimeOptions::default()).expect("url source");
    server.join().expect("join server");
    let expected_url = format!("{url}/");
    assert_eq!(
        loaded_url.input_base_url.as_deref(),
        Some(expected_url.as_str())
    );

    let agent = build_http_agent(&RuntimeOptions::default());
    assert!(matches!(
        agent.config().tls_config().root_certs(),
        RootCerts::PlatformVerifier
    ));
    assert_eq!(
        agent.config().timeouts().connect,
        Some(std::time::Duration::from_millis(5_000))
    );
    assert_eq!(
        agent.config().timeouts().global,
        Some(std::time::Duration::from_millis(DEFAULT_FETCH_TIMEOUT_MS))
    );
}
#[test]
fn url_loading_get_error_and_status_failures_cover_remaining_branches() {
    let closed_listener = TcpListener::bind("127.0.0.1:0").expect("bind closed listener");
    let closed_address = closed_listener.local_addr().expect("closed listener addr");
    drop(closed_listener);

    let transport_error = read_url_source(
        &url_source(&format!("http://{closed_address}")),
        &RuntimeOptions {
            fetch_preflight: FetchPreflightMode::GetOnly,
            fetch_timeout_ms: 250,
            ..RuntimeOptions::default()
        },
    )
    .expect_err("get transport failure");
    assert_eq!(transport_error.code, "SOURCE_LOAD_FAILED");
    assert!(transport_error.message.contains("Could not fetch"));
    assert_eq!(
        transport_error
            .details
            .as_ref()
            .and_then(|details| details.get("method"))
            .and_then(Value::as_str),
        Some("GET")
    );

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind status server");
    let address = listener.local_addr().expect("status server addr");
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let mut request_buffer = [0u8; 512];
        let _ = stream.read(&mut request_buffer).expect("read request");
        let response =
            "HTTP/1.1 404 Not Found\r\nContent-Type: text/html\r\nContent-Length: 0\r\n\r\n";
        stream
            .write_all(response.as_bytes())
            .expect("write response");
    });

    let status_error = read_url_source(
        &url_source(&format!("http://{address}")),
        &RuntimeOptions {
            fetch_preflight: FetchPreflightMode::GetOnly,
            ..RuntimeOptions::default()
        },
    )
    .expect_err("unexpected get status");

    server.join().expect("join server");
    assert_eq!(status_error.code, "SOURCE_LOAD_FAILED");
    assert!(
        status_error
            .message
            .contains("returned unexpected status 404")
    );
    assert_eq!(
        status_error
            .details
            .as_ref()
            .and_then(|details| details.get("method"))
            .and_then(Value::as_str),
        Some("GET")
    );
    assert_eq!(
        status_error
            .details
            .as_ref()
            .and_then(|details| details.get("status"))
            .and_then(Value::as_u64),
        Some(404)
    );
}
#[test]
fn url_loading_accepts_headerless_and_xhtml_success_responses() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind headerless server");
    let address = listener.local_addr().expect("headerless server addr");
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let mut request_buffer = [0u8; 512];
        let _ = stream.read(&mut request_buffer).expect("read request");
        let body = "<html><body>Headerless</body></html>";
        let response = format!("HTTP/1.1 200 OK\r\n\r\n{body}");
        stream
            .write_all(response.as_bytes())
            .expect("write response");
    });

    let headerless = read_url_source(
        &url_source(&format!("http://{address}")),
        &RuntimeOptions {
            fetch_preflight: FetchPreflightMode::GetOnly,
            ..RuntimeOptions::default()
        },
    )
    .expect("headerless response");
    server.join().expect("join server");
    assert_eq!(headerless.text, "<html><body>Headerless</body></html>");

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind xhtml server");
    let address = listener.local_addr().expect("xhtml server addr");
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let mut request_buffer = [0u8; 512];
        let _ = stream.read(&mut request_buffer).expect("read request");
        let body = "<html xmlns=\"http://www.w3.org/1999/xhtml\"><body>XHTML</body></html>";
        let response =
            format!("HTTP/1.1 200 OK\r\nContent-Type: application/xhtml+xml\r\n\r\n{body}");
        stream
            .write_all(response.as_bytes())
            .expect("write response");
    });

    let xhtml = read_url_source(
        &url_source(&format!("http://{address}")),
        &RuntimeOptions {
            fetch_preflight: FetchPreflightMode::GetOnly,
            ..RuntimeOptions::default()
        },
    )
    .expect("xhtml response");
    server.join().expect("join server");
    assert!(xhtml.text.contains("XHTML"));
    assert!(!content_type_is_obviously_non_html_for_tests(""));
    assert!(!content_type_is_obviously_non_html_for_tests("text/html"));
    assert!(!content_type_is_obviously_non_html_for_tests(
        "text/html; charset=utf-8"
    ));
    assert!(!content_type_is_obviously_non_html_for_tests(
        "text/html;charset=UTF-8"
    ));
    assert!(!content_type_is_obviously_non_html_for_tests(
        "application/xhtml+xml"
    ));
    assert!(!content_type_is_obviously_non_html_for_tests(
        "application/xhtml+xml; charset=utf-8"
    ));
    assert!(content_type_is_obviously_non_html_for_tests("text/plain"));
    assert!(content_type_is_obviously_non_html_for_tests(
        "application/json"
    ));
    assert!(content_type_is_obviously_non_html_for_tests("text/xml"));
    assert!(content_type_is_obviously_non_html_for_tests("image/png"));
}
