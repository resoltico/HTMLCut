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
