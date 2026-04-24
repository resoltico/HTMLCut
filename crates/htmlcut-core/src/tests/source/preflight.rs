use super::*;

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
fn head_preflight_falls_back_to_get_when_head_returns_forbidden() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind forbidden-head server");
    let address = listener.local_addr().expect("forbidden-head server addr");
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
                "HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\n\r\n".to_owned()
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
    assert_eq!(loaded.load_steps[0].action, SourceLoadAction::HeadPreflight);
    assert_eq!(loaded.load_steps[0].outcome, SourceLoadOutcome::Fallback);
    assert_eq!(loaded.load_steps[0].status, Some(403));
    assert!(loaded.load_steps[0].message.contains("fell back to GET"));
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
