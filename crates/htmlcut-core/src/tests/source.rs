use super::*;

#[test]
fn source_reading_helpers_cover_error_paths() {
    struct BrokenReader;
    impl Read for BrokenReader {
        fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
            Err(IoError::other("boom"))
        }
    }

    let mut broken = BrokenReader;
    let read_error = read_limited_to_string(&mut broken, 10, "Input").expect_err("read error");
    assert_eq!(read_error.code, "SOURCE_LOAD_FAILED");

    let mut oversized = Cursor::new(b"abcdef".to_vec());
    let size_error = read_limited_to_string(&mut oversized, 3, "Input").expect_err("size error");
    assert_eq!(size_error.code, "SOURCE_LOAD_FAILED");

    let mut invalid_utf8 = Cursor::new(vec![0xff, 0xfe]);
    let utf8_error =
        read_limited_to_string(&mut invalid_utf8, 10, "Input").expect_err("utf8 error");
    assert_eq!(utf8_error.code, "SOURCE_LOAD_FAILED");

    let tempdir = htmlcut_tempdir::tempdir().expect("tempdir");
    let large_path = tempdir.path().join("large.txt");
    std::fs::write(&large_path, "1234567890").expect("write");
    let file_error = read_file_source(
        &file_source(&large_path),
        &RuntimeOptions {
            max_bytes: 3,
            fetch_timeout_ms: 1000,
            ..RuntimeOptions::default()
        },
    )
    .expect_err("file too large");
    assert_eq!(file_error.code, "SOURCE_LOAD_FAILED");

    let missing_file = read_file_source(
        &file_source(tempdir.path().join("missing.txt")),
        &RuntimeOptions::default(),
    )
    .expect_err("missing file");
    assert_eq!(missing_file.code, "SOURCE_LOAD_FAILED");

    let invalid_utf8_path = tempdir.path().join("invalid-utf8.txt");
    std::fs::write(&invalid_utf8_path, [0xff, 0xfe]).expect("write invalid utf8");
    let invalid_utf8_file =
        read_file_source(&file_source(&invalid_utf8_path), &RuntimeOptions::default())
            .expect_err("invalid utf8 file");
    assert_eq!(invalid_utf8_file.code, "SOURCE_LOAD_FAILED");
    assert!(
        invalid_utf8_file
            .message
            .contains("File is not valid UTF-8:")
    );

    let directory_error =
        read_file_source(&file_source(tempdir.path()), &RuntimeOptions::default())
            .expect_err("directory input");
    assert_eq!(directory_error.code, "SOURCE_LOAD_FAILED");
    assert!(
        directory_error
            .message
            .contains("Input path is a directory, not a file:")
    );

    let closed_listener = TcpListener::bind("127.0.0.1:0").expect("bind closed listener");
    let closed_address = closed_listener.local_addr().expect("closed listener addr");
    drop(closed_listener);
    let fetch_error = read_url_source(
        &url_source(&format!("http://{closed_address}")),
        &RuntimeOptions {
            max_bytes: 1024,
            fetch_timeout_ms: 250,
            ..RuntimeOptions::default()
        },
    )
    .expect_err("fetch error");
    assert_eq!(fetch_error.code, "SOURCE_LOAD_FAILED");

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind size server");
    let address = listener.local_addr().expect("size server addr");
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let mut request_buffer = [0u8; 512];
        let _ = stream.read(&mut request_buffer).expect("read request");
        let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: 9999\r\n\r\n";
        stream
            .write_all(response.as_bytes())
            .expect("write response");
    });

    let oversized_response = read_url_source(
        &url_source(&format!("http://{address}")),
        &RuntimeOptions {
            max_bytes: 4,
            fetch_timeout_ms: 1000,
            ..RuntimeOptions::default()
        },
    )
    .expect_err("oversized response");
    server.join().expect("join server");
    assert_eq!(oversized_response.code, "SOURCE_LOAD_FAILED");
}

#[test]
fn source_loading_covers_memory_limits_and_extract_load_failures() {
    let oversized_memory = load_source(
        &memory_source("inline", "12345"),
        &RuntimeOptions {
            max_bytes: 3,
            fetch_timeout_ms: 1000,
            ..RuntimeOptions::default()
        },
    )
    .expect_err("oversized memory source");
    assert_eq!(oversized_memory.code, "SOURCE_LOAD_FAILED");

    assert_eq!(url_source("http://example.com").kind(), SourceKind::Url);

    let extract_result = extract(
        &ExtractionRequest::new(
            memory_source("inline", "12345"),
            selector_request("<article>Hello</article>").extraction,
        ),
        &RuntimeOptions {
            max_bytes: 3,
            fetch_timeout_ms: 1000,
            ..RuntimeOptions::default()
        },
    );
    assert!(!extract_result.ok);
    assert_eq!(extract_result.diagnostics[0].code, "SOURCE_LOAD_FAILED");
}

#[test]
fn stream_source_helpers_preserve_failure_metadata() {
    let runtime = RuntimeOptions {
        max_bytes: 3,
        fetch_timeout_ms: 1000,
        ..RuntimeOptions::default()
    };
    let source_value = "https://example.com/page";
    let url_request = url_source(source_value);
    let mut oversized_response = Cursor::new(b"hello".to_vec());
    let response_failure = finish_url_source_from_reader_for_tests(
        &url_request,
        &runtime,
        source_value,
        200,
        Some(format!("{source_value}/")),
        vec![
            SourceLoadStep {
                action: SourceLoadAction::HeadPreflight,
                outcome: SourceLoadOutcome::Skipped,
                status: None,
                message: "Skipped HEAD preflight because --fetch-preflight get-only was requested."
                    .to_owned(),
            },
            SourceLoadStep {
                action: SourceLoadAction::Get,
                outcome: SourceLoadOutcome::Succeeded,
                status: Some(200),
                message: "Fetched the remote source with GET.".to_owned(),
            },
        ],
        &mut oversized_response,
    )
    .expect_err("oversized response body");
    assert_eq!(response_failure.code, "SOURCE_LOAD_FAILED");
    assert_eq!(response_failure.metadata.kind, SourceKind::Url);
    assert_eq!(
        response_failure.metadata.input_base_url.as_deref(),
        Some("https://example.com/page")
    );
    assert_eq!(response_failure.metadata.load_steps.len(), 3);
    assert_eq!(
        response_failure
            .metadata
            .load_steps
            .last()
            .expect("failed GET step")
            .outcome,
        SourceLoadOutcome::Failed
    );
    assert_eq!(
        response_failure
            .metadata
            .load_steps
            .last()
            .expect("failed GET step")
            .status,
        Some(200)
    );
    assert!(
        response_failure
            .metadata
            .load_steps
            .last()
            .expect("failed GET step")
            .message
            .contains("GET body read failed after status 200")
    );

    let stdin_request =
        SourceRequest::stdin().with_base_url(Url::parse("https://example.com/base/").expect("url"));
    let mut oversized_stdin = Cursor::new(b"hello".to_vec());
    let stdin_failure =
        read_stdin_source_from_reader_for_tests(&stdin_request, &runtime, &mut oversized_stdin)
            .expect_err("oversized stdin");
    assert_eq!(stdin_failure.code, "SOURCE_LOAD_FAILED");
    assert_eq!(stdin_failure.metadata.kind, SourceKind::Stdin);
    assert_eq!(stdin_failure.metadata.value, "-");
    assert_eq!(
        stdin_failure.metadata.input_base_url.as_deref(),
        Some("https://example.com/base/")
    );
    assert!(stdin_failure.metadata.load_steps.is_empty());
}

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
}

#[test]
fn get_only_fetch_preflight_skips_head_requests() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind get-only server");
    let address = listener.local_addr().expect("get-only server addr");
    let methods = Arc::new(Mutex::new(Vec::new()));
    let methods_for_server = Arc::clone(&methods);
    let server = thread::spawn(move || {
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
            .expect("request method")
            .to_owned();
        methods_for_server
            .lock()
            .expect("lock methods")
            .push(method);

        let body = "<html><body>Hello</body></html>";
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        stream
            .write_all(response.as_bytes())
            .expect("write response");
    });

    let url = format!("http://{address}");
    let loaded = read_url_source(
        &url_source(&url),
        &RuntimeOptions {
            fetch_preflight: FetchPreflightMode::GetOnly,
            ..RuntimeOptions::default()
        },
    )
    .expect("get-only source");

    server.join().expect("join server");
    assert_eq!(methods.lock().expect("lock methods").as_slice(), ["GET"]);
    assert_eq!(loaded.text, "<html><body>Hello</body></html>");
    assert_eq!(loaded.load_steps.len(), 2);
    assert_eq!(loaded.load_steps[0].action, SourceLoadAction::HeadPreflight);
    assert_eq!(loaded.load_steps[0].outcome, SourceLoadOutcome::Skipped);
    assert_eq!(loaded.load_steps[1].action, SourceLoadAction::Get);
    assert_eq!(loaded.load_steps[1].outcome, SourceLoadOutcome::Succeeded);
}

#[test]
fn head_preflight_falls_back_to_get_when_head_is_unsupported() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind fallback server");
    let address = listener.local_addr().expect("fallback server addr");
    let methods = Arc::new(Mutex::new(Vec::new()));
    let methods_for_server = Arc::clone(&methods);
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
                .expect("request method")
                .to_owned();
            methods_for_server
                .lock()
                .expect("lock methods")
                .push(method.clone());

            let response = if method == "HEAD" {
                "HTTP/1.1 405 Method Not Allowed\r\nContent-Length: 0\r\n\r\n".to_owned()
            } else {
                let body = "<html><body>Fallback</body></html>";
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
    let loaded =
        read_url_source(&url_source(&url), &RuntimeOptions::default()).expect("fallback source");

    server.join().expect("join server");
    assert_eq!(
        methods.lock().expect("lock methods").as_slice(),
        ["HEAD", "GET"]
    );
    assert_eq!(loaded.text, "<html><body>Fallback</body></html>");
    assert_eq!(loaded.load_steps.len(), 2);
    assert_eq!(loaded.load_steps[0].action, SourceLoadAction::HeadPreflight);
    assert_eq!(loaded.load_steps[0].outcome, SourceLoadOutcome::Fallback);
    assert_eq!(loaded.load_steps[0].status, Some(405));
    assert_eq!(loaded.load_steps[1].action, SourceLoadAction::Get);
    assert_eq!(loaded.load_steps[1].outcome, SourceLoadOutcome::Succeeded);
}

#[test]
fn head_preflight_falls_back_to_get_when_head_transport_breaks() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind broken-head server");
    let address = listener.local_addr().expect("broken-head server addr");
    let methods = Arc::new(Mutex::new(Vec::new()));
    let methods_for_server = Arc::clone(&methods);
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
                .expect("request method")
                .to_owned();
            methods_for_server
                .lock()
                .expect("lock methods")
                .push(method.clone());

            if method == "HEAD" {
                continue;
            }

            let body = "<html><body>Recovered</body></html>";
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            );
            stream
                .write_all(response.as_bytes())
                .expect("write response");
        }
    });

    let url = format!("http://{address}");
    let loaded = read_url_source(
        &url_source(&url),
        &RuntimeOptions {
            fetch_timeout_ms: 250,
            ..RuntimeOptions::default()
        },
    )
    .expect("fallback source after broken head transport");

    server.join().expect("join server");
    assert_eq!(
        methods.lock().expect("lock methods").as_slice(),
        ["HEAD", "GET"]
    );
    assert_eq!(loaded.text, "<html><body>Recovered</body></html>");
    assert_eq!(loaded.load_steps.len(), 2);
    assert_eq!(loaded.load_steps[0].action, SourceLoadAction::HeadPreflight);
    assert_eq!(loaded.load_steps[0].outcome, SourceLoadOutcome::Fallback);
    assert!(loaded.load_steps[0].message.contains("fell back to GET"));
    assert_eq!(loaded.load_steps[1].action, SourceLoadAction::Get);
    assert_eq!(loaded.load_steps[1].outcome, SourceLoadOutcome::Succeeded);
}

#[test]
fn head_preflight_fallback_classifier_accepts_only_head_intolerance_errors() {
    assert!(head_error_allows_get_fallback_for_tests(
        &ureq::Error::ConnectionFailed
    ));
    assert!(head_error_allows_get_fallback_for_tests(&ureq::Error::Io(
        io::Error::new(io::ErrorKind::UnexpectedEof, "peer disconnected"),
    )));
    assert!(!head_error_allows_get_fallback_for_tests(&ureq::Error::Io(
        io::Error::new(io::ErrorKind::TimedOut, "timed out"),
    )));
    assert!(!head_error_allows_get_fallback_for_tests(
        &ureq::Error::HostNotFound
    ));
}

#[test]
fn head_preflight_rejects_non_html_responses_before_get() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind non-html server");
    let address = listener.local_addr().expect("non-html server addr");
    let methods = Arc::new(Mutex::new(Vec::new()));
    let methods_for_server = Arc::clone(&methods);
    let server = thread::spawn(move || {
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
            .expect("request method")
            .to_owned();
        methods_for_server
            .lock()
            .expect("lock methods")
            .push(method);

        let response = "HTTP/1.1 200 OK\r\nContent-Type: image/png\r\nContent-Length: 0\r\n\r\n";
        stream
            .write_all(response.as_bytes())
            .expect("write response");
    });

    let url = format!("http://{address}");
    let error = read_url_source(&url_source(&url), &RuntimeOptions::default())
        .expect_err("non-html preflight error");

    server.join().expect("join server");
    assert_eq!(methods.lock().expect("lock methods").as_slice(), ["HEAD"]);
    assert_eq!(error.code, "SOURCE_LOAD_FAILED");
    assert!(
        error
            .message
            .contains("reported non-HTML content type image/png")
    );
    assert_eq!(
        error
            .details
            .as_ref()
            .and_then(|details| details.get("method"))
            .and_then(Value::as_str),
        Some("HEAD")
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
        "application/xhtml+xml"
    ));
    assert!(content_type_is_obviously_non_html_for_tests("image/png"));
}
