use super::*;

#[test]
fn declared_http_charset_distinguishes_non_charset_parameters_from_malformed_charset_parameters() {
    assert_eq!(
        declared_http_charset_for_tests(Some("text/html; format=flowed; charset=\"utf-8\""))
            .expect("quoted UTF-8 charset"),
        Some("utf-8".to_owned())
    );
    assert_eq!(
        declared_http_charset_for_tests(Some("text/html; boundary"))
            .expect("unrelated bare parameter"),
        None
    );
    for content_type in ["text/html; charset", "text/html; charset=   "] {
        let error = declared_http_charset_for_tests(Some(content_type))
            .expect_err("malformed charset parameter");
        assert_eq!(error.code, "SOURCE_LOAD_FAILED");
        assert!(error.message.contains("malformed charset"));
    }
    assert_eq!(
        declared_http_charset_for_tests(Some("text/html; charset='utf-8"))
            .expect("non-empty label remains a decoding concern"),
        Some("'utf-8".to_owned())
    );
}

#[test]
fn redirected_url_sources_use_the_safe_final_url_for_the_input_base_and_load_trace() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind redirect server");
    let address = listener.local_addr().expect("redirect server address");
    let server = thread::spawn(move || {
        for expected_target in ["/start", "/deep/page?token=secret"] {
            let (mut stream, _) = listener.accept().expect("accept redirect request");
            assert_eq!(request_target(&mut stream), expected_target);
            if expected_target == "/start" {
                write_http_response(
                    &mut stream,
                    "302 Found",
                    &["Location: /deep/page?token=secret"],
                    b"",
                );
            } else {
                write_http_response(
                    &mut stream,
                    "200 OK",
                    &["Content-Type: text/html"],
                    b"<html><head><base href=\"assets/\"></head><body><a href=\"next.html\">Next</a></body></html>",
                );
            }
        }
    });

    let requested_url = format!("http://{address}/start");
    let loaded = read_url_source(
        &url_source(&requested_url),
        &RuntimeOptions {
            fetch_preflight: FetchPreflightMode::GetOnly,
            ..RuntimeOptions::default()
        },
    )
    .expect("redirected source");

    server.join().expect("join redirect server");
    let safe_final_url = format!("http://{address}/deep/page?[redacted]");
    assert_eq!(loaded.value, requested_url);
    assert_eq!(
        loaded.input_base_url.as_deref(),
        Some(safe_final_url.as_str())
    );
    assert_eq!(
        resolve_document_base_url(
            &parse_document_node(&loaded.text),
            loaded.input_base_url.as_deref(),
        )
        .as_deref(),
        Some(format!("http://{address}/deep/assets/").as_str())
    );
    assert_eq!(loaded.load_steps.len(), 2);
    assert_eq!(loaded.load_steps[1].action, SourceLoadAction::Get);
    assert_eq!(loaded.load_steps[1].outcome, SourceLoadOutcome::Succeeded);
    assert!(loaded.load_steps[1].message.contains(&safe_final_url));
    assert!(
        !loaded
            .load_steps
            .iter()
            .any(|step| step.message.contains("secret"))
    );
}

#[test]
fn explicit_base_url_overrides_a_redirected_response_url() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind redirect server");
    let address = listener.local_addr().expect("redirect server address");
    let server = thread::spawn(move || {
        for expected_target in ["/start", "/deep/page"] {
            let (mut stream, _) = listener.accept().expect("accept redirect request");
            assert_eq!(request_target(&mut stream), expected_target);
            if expected_target == "/start" {
                write_http_response(&mut stream, "302 Found", &["Location: /deep/page"], b"");
            } else {
                write_http_response(
                    &mut stream,
                    "200 OK",
                    &["Content-Type: text/html"],
                    b"<html><body>Redirected</body></html>",
                );
            }
        }
    });

    let loaded = read_url_source(
        &url_source(&format!("http://{address}/start"))
            .with_base_url(http_url("https://example.test/override/")),
        &RuntimeOptions {
            fetch_preflight: FetchPreflightMode::GetOnly,
            ..RuntimeOptions::default()
        },
    )
    .expect("redirected source with explicit base");

    server.join().expect("join redirect server");
    assert_eq!(
        loaded.input_base_url.as_deref(),
        Some("https://example.test/override/")
    );
}

#[test]
fn url_sources_decode_declared_charsets_and_reject_unsafe_or_oversized_decoded_text() {
    let expected = concat!("caf", "\u{00e9}");
    let (iso_url, iso_server) = start_single_response_server(
        "text/html; CHARSET='ISO-8859-1'",
        b"<html><body>caf\xE9</body></html>".to_vec(),
    );
    let iso_loaded = read_url_source(
        &url_source(&iso_url),
        &RuntimeOptions {
            fetch_preflight: FetchPreflightMode::GetOnly,
            ..RuntimeOptions::default()
        },
    )
    .expect("iso-8859-1 response");
    iso_server.join().expect("join iso server");
    assert!(iso_loaded.text.contains(expected));
    assert_eq!(iso_loaded.bytes_read, iso_loaded.text.len());

    let (xhtml_url, xhtml_server) = start_single_response_server(
        "application/xhtml+xml; charset=iso-8859-1",
        b"<html><body>caf\xE9</body></html>".to_vec(),
    );
    let xhtml_loaded = read_url_source(
        &url_source(&xhtml_url),
        &RuntimeOptions {
            fetch_preflight: FetchPreflightMode::GetOnly,
            ..RuntimeOptions::default()
        },
    )
    .expect("xhtml iso-8859-1 response");
    xhtml_server.join().expect("join xhtml server");
    assert!(xhtml_loaded.text.contains(expected));

    let (bom_url, bom_server) = start_single_response_server(
        "text/html; charset=iso-8859-1",
        b"\xEF\xBB\xBF<html><body>caf\xC3\xA9</body></html>".to_vec(),
    );
    let bom_loaded = read_url_source(
        &url_source(&bom_url),
        &RuntimeOptions {
            fetch_preflight: FetchPreflightMode::GetOnly,
            ..RuntimeOptions::default()
        },
    )
    .expect("bom should override the declared legacy charset");
    bom_server.join().expect("join bom server");
    assert!(bom_loaded.text.contains(expected));

    let (unknown_url, unknown_server) =
        start_single_response_server("text/html; charset=made-up", b"<p>hello</p>".to_vec());
    let unknown_error = read_url_source(
        &url_source(&unknown_url),
        &RuntimeOptions {
            fetch_preflight: FetchPreflightMode::GetOnly,
            ..RuntimeOptions::default()
        },
    )
    .expect_err("unknown charset should fail");
    unknown_server.join().expect("join unknown charset server");
    assert_eq!(unknown_error.code, "SOURCE_LOAD_FAILED");
    assert!(unknown_error.message.contains("charset"));
    assert_failed_get_without_success(&unknown_error.metadata.load_steps);

    let (invalid_utf16_url, invalid_utf16_server) =
        start_single_response_server("text/html; charset=utf-16le", vec![0x00]);
    let invalid_utf16_error = read_url_source(
        &url_source(&invalid_utf16_url),
        &RuntimeOptions {
            fetch_preflight: FetchPreflightMode::GetOnly,
            ..RuntimeOptions::default()
        },
    )
    .expect_err("incomplete UTF-16 response should fail");
    invalid_utf16_server
        .join()
        .expect("join invalid UTF-16 server");
    assert_eq!(invalid_utf16_error.code, "SOURCE_LOAD_FAILED");
    assert!(invalid_utf16_error.message.contains("not valid"));
    assert_failed_get_without_success(&invalid_utf16_error.metadata.load_steps);

    let (quoted_utf8_url, quoted_utf8_server) = start_single_response_server(
        "text/html; format=flowed; charset=\"utf-8\"",
        b"<p>quoted UTF-8</p>".to_vec(),
    );
    let quoted_utf8_loaded = read_url_source(
        &url_source(&quoted_utf8_url),
        &RuntimeOptions {
            fetch_preflight: FetchPreflightMode::GetOnly,
            ..RuntimeOptions::default()
        },
    )
    .expect("quoted UTF-8 response");
    quoted_utf8_server.join().expect("join quoted UTF-8 server");
    assert!(quoted_utf8_loaded.text.contains("quoted UTF-8"));

    for malformed_content_type in [
        "text/html; charset",
        "text/html; charset=   ",
        "text/html; charset=\"iso-8859-1",
        "text/html; charset=iso-8859-1\"",
        // These asymmetric forms would become a valid UTF-8 label if either
        // quote-boundary conjunction were weakened to a disjunction.
        "text/html; charset=\"utf-8x",
        "text/html; charset=xutf-8\"",
        "text/html; charset=xutf-8'",
    ] {
        let (malformed_url, malformed_server) = start_single_response_server(
            malformed_content_type,
            b"<html><body>caf\xE9</body></html>".to_vec(),
        );
        let malformed_error = read_url_source(
            &url_source(&malformed_url),
            &RuntimeOptions {
                fetch_preflight: FetchPreflightMode::GetOnly,
                ..RuntimeOptions::default()
            },
        )
        .expect_err("malformed charset quote should fail");
        malformed_server
            .join()
            .expect("join malformed charset server");
        assert_eq!(malformed_error.code, "SOURCE_LOAD_FAILED");
        assert!(malformed_error.message.contains("charset"));
        assert_failed_get_without_success(&malformed_error.metadata.load_steps);
    }

    let (invalid_url, invalid_server) =
        start_single_response_server("text/html", b"<p>caf\xE9</p>".to_vec());
    let invalid_error = read_url_source(
        &url_source(&invalid_url),
        &RuntimeOptions {
            fetch_preflight: FetchPreflightMode::GetOnly,
            ..RuntimeOptions::default()
        },
    )
    .expect_err("undeclared non-utf8 response should fail");
    invalid_server.join().expect("join invalid utf8 server");
    assert_eq!(invalid_error.code, "SOURCE_LOAD_FAILED");
    assert!(invalid_error.message.contains("valid UTF-8"));
    assert_failed_get_without_success(&invalid_error.metadata.load_steps);

    let (limited_url, limited_server) =
        start_single_response_server("text/html; charset=iso-8859-1", vec![0xE9, 0xE9]);
    let limited_error = read_url_source(
        &url_source(&limited_url),
        &RuntimeOptions {
            max_bytes: max_bytes_limit(3),
            fetch_preflight: FetchPreflightMode::GetOnly,
            ..RuntimeOptions::default()
        },
    )
    .expect_err("decoded text should remain bounded");
    limited_server.join().expect("join decoded size server");
    assert_eq!(limited_error.code, "SOURCE_LOAD_FAILED");
    assert!(limited_error.message.contains("exceeds"));
    assert_failed_get_without_success(&limited_error.metadata.load_steps);

    let (boundary_url, boundary_server) =
        start_single_response_server("text/html; charset=iso-8859-1", vec![0xE9]);
    let boundary_loaded = read_url_source(
        &url_source(&boundary_url),
        &RuntimeOptions {
            max_bytes: max_bytes_limit(2),
            fetch_preflight: FetchPreflightMode::GetOnly,
            ..RuntimeOptions::default()
        },
    )
    .expect("decoded text exactly at the limit");
    boundary_server
        .join()
        .expect("join decoded boundary server");
    assert_eq!(boundary_loaded.text, "\u{00e9}");
    assert_eq!(boundary_loaded.bytes_read, 2);
}

fn start_single_response_server(
    content_type: &str,
    body: Vec<u8>,
) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind response server");
    let address = listener.local_addr().expect("response server address");
    let content_type = content_type.to_owned();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept response request");
        let _ = request_target(&mut stream);
        write_http_response(
            &mut stream,
            "200 OK",
            &[&format!("Content-Type: {content_type}")],
            &body,
        );
    });
    (format!("http://{address}/page"), server)
}

fn request_target(stream: &mut std::net::TcpStream) -> String {
    let mut buffer = [0u8; 1024];
    let read = stream.read(&mut buffer).expect("read request");
    String::from_utf8_lossy(&buffer[..read])
        .lines()
        .next()
        .expect("request line")
        .split_whitespace()
        .nth(1)
        .expect("request target")
        .to_owned()
}

fn write_http_response(
    stream: &mut std::net::TcpStream,
    status: &str,
    headers: &[&str],
    body: &[u8],
) {
    let mut response = format!(
        "HTTP/1.1 {status}\r\nConnection: close\r\nContent-Length: {}\r\n",
        body.len()
    );
    for header in headers {
        response.push_str(header);
        response.push_str("\r\n");
    }
    response.push_str("\r\n");
    stream
        .write_all(response.as_bytes())
        .expect("write response headers");
    stream.write_all(body).expect("write response body");
}

fn assert_failed_get_without_success(load_steps: &[SourceLoadStep]) {
    assert!(load_steps.iter().any(|step| {
        step.action == SourceLoadAction::Get && step.outcome == SourceLoadOutcome::Failed
    }));
    assert!(!load_steps.iter().any(|step| {
        step.action == SourceLoadAction::Get && step.outcome == SourceLoadOutcome::Succeeded
    }));
}
