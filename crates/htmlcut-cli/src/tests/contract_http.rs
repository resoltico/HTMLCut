use super::*;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

fn serve(responses: Vec<Vec<u8>>) -> (String, JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!(
        "http://{}/fixture?x=1&token=SYNTHETIC_SENTINEL",
        listener.local_addr().unwrap()
    );
    listener.set_nonblocking(true).unwrap();
    let worker = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut requests = Vec::new();
        for response in responses {
            let (mut stream, _) = loop {
                match listener.accept() {
                    Ok(pair) => break pair,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(Instant::now() < deadline, "local HTTP fixture timed out");
                        std::thread::sleep(Duration::from_millis(1));
                    }
                    Err(error) => panic!("fixture accept failed: {error}"),
                }
            };
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut request = Vec::new();
            while !request.ends_with(b"\r\n\r\n") {
                let mut byte = [0];
                assert_eq!(stream.read(&mut byte).unwrap(), 1);
                request.push(byte[0]);
                assert!(request.len() <= 16 * 1024);
            }
            requests.push(String::from_utf8(request).unwrap());
            stream.write_all(&response).unwrap();
        }
        requests
    });
    (url, worker)
}

fn response(body: &[u8], headers: &str) -> Vec<u8> {
    let mut bytes = format!(
        "HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Length: {}\r\n{headers}\r\n",
        body.len()
    )
    .into_bytes();
    bytes.extend_from_slice(body);
    bytes
}

#[test]
fn t01_t04_route_parity_query_preservation_get_only_and_charset() {
    let body = b"<p id='amount'>EUR 180</p>";
    let (url, worker) = serve(vec![response(
        body,
        "Content-Type: text/html; charset=utf-8\r\n",
    )]);
    let (code, output, error) = invoke(
        &[
            "htmlcut",
            "extract",
            "--url",
            &url,
            "--css",
            "#amount",
            "--base-url",
            "https://example.test/",
        ],
        b"",
    );
    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&error));
    assert!(error.is_empty());
    let requests = worker.join().unwrap();
    assert_eq!(requests.len(), 1);
    assert!(requests[0].starts_with("GET /fixture?x=1&token=SYNTHETIC_SENTINEL HTTP/1.1\r\n"));
    assert!(
        !String::from_utf8(output.clone())
            .unwrap()
            .contains("SYNTHETIC_SENTINEL")
    );
    let (_, stdin, _) = invoke(
        &[
            "htmlcut",
            "extract",
            "--stdin",
            "--css",
            "#amount",
            "--base-url",
            "https://example.test/",
        ],
        body,
    );
    assert_eq!(output, stdin);
    let snapshot = htmlcut_core::SourceSnapshot::new(
        std::str::from_utf8(body).unwrap(),
        htmlcut_core::SnapshotMetadata {
            base_url: Some("https://example.test/".into()),
        },
    )
    .unwrap();
    let result =
        htmlcut_core::PreparedDocument::new(snapshot, htmlcut_core::PreparationLimits::default())
            .unwrap()
            .execute(
                &htmlcut_core::CompiledPlan::compile(
                    &htmlcut_core::ExtractionPlan::css("#amount").unwrap(),
                )
                .unwrap(),
            )
            .unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&output).unwrap(),
        serde_json::to_value(result).unwrap()
    );
    let encoded = b"<p id='amount'>\x80</p>";
    let (url, worker) = serve(vec![response(
        encoded,
        "Content-Type: text/html; charset=windows-1252\r\n",
    )]);
    let (code, output, error) = invoke(
        &[
            "htmlcut", "extract", "--url", &url, "--css", "#amount", "--raw",
        ],
        b"",
    );
    assert_eq!(
        (code, output, error),
        (0, "€".as_bytes().to_vec(), Vec::new())
    );
    assert_eq!(worker.join().unwrap().len(), 1);
}

#[test]
fn t03_compressed_and_chunked_bodies_are_complete_and_strict() {
    let body = b"<p>compressed</p>";
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(body).unwrap();
    let compressed = encoder.finish().unwrap();
    let (url, worker) = serve(vec![response(&compressed, "Content-Encoding: gzip\r\n")]);
    assert_eq!(
        invoke(
            &["htmlcut", "extract", "--url", &url, "--css", "p", "--raw"],
            b""
        ),
        (0, b"compressed".to_vec(), Vec::new())
    );
    let requests = worker.join().unwrap();
    assert!(
        requests[0]
            .to_ascii_lowercase()
            .contains("\r\naccept-encoding: gzip\r\n")
    );
    let chunked = b"HTTP/1.1 200 OK\r\nConnection: close\r\nTransfer-Encoding: chunked\r\n\r\n3\r\n<p>\r\n5\r\nvalue\r\n4\r\n</p>\r\n0\r\n\r\n".to_vec();
    let (url, worker) = serve(vec![chunked]);
    assert_eq!(
        invoke(
            &["htmlcut", "extract", "--url", &url, "--css", "p", "--raw"],
            b""
        ),
        (0, b"value".to_vec(), Vec::new())
    );
    worker.join().unwrap();
    let (url, worker) = serve(vec![response(
        b"\xff",
        "Content-Type: text/html; charset=utf-8\r\n",
    )]);
    let (code, output, error) = invoke(&["htmlcut", "extract", "--url", &url, "--css", "p"], b"");
    assert_eq!(code, 5);
    assert!(output.is_empty());
    assert!(
        !String::from_utf8(error)
            .unwrap()
            .contains("SYNTHETIC_SENTINEL")
    );
    worker.join().unwrap();
}

#[test]
fn t04_transient_url_persistence_is_rejected_before_network_io() {
    let root = htmlcut_tempdir::tempdir().unwrap();
    let run = root.path().join("run.json");
    let url = "http://127.0.0.1:1/?token=SYNTHETIC_SENTINEL";
    let (code, output, error) = invoke(
        &[
            "htmlcut",
            "extract",
            "--url",
            url,
            "--css",
            "p",
            "--save-run",
            run.to_str().unwrap(),
        ],
        b"",
    );
    assert_eq!(code, 2);
    assert!(output.is_empty());
    assert!(!run.exists());
    assert!(
        !String::from_utf8(error)
            .unwrap()
            .contains("SYNTHETIC_SENTINEL")
    );
}

#[test]
fn t03_transport_failures_and_invalid_response_metadata_are_bounded_and_redacted() {
    let failed =
        b"HTTP/1.1 404 Not Found\r\nConnection: close\r\nContent-Length: 0\r\n\r\n".to_vec();
    let (url, worker) = serve(vec![failed]);
    let (code, out, error) = invoke(&["htmlcut", "extract", "--url", &url, "--css", "p"], b"");
    assert_eq!(code, 5);
    assert!(out.is_empty());
    assert!(
        !String::from_utf8(error)
            .unwrap()
            .contains("SYNTHETIC_SENTINEL")
    );
    worker.join().unwrap();
    for bytes in [b"HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Type: \xff\r\nContent-Length: 0\r\n\r\n".to_vec(),
        b"HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Encoding: \xff\r\nContent-Length: 0\r\n\r\n".to_vec()] {
        let (url,worker)=serve(vec![bytes]);let (code,out,_)=invoke(&["htmlcut","extract","--url",&url,"--css","p"],b"");
        assert_eq!(code,5);assert!(out.is_empty());worker.join().unwrap();
    }
    let (code, out, error) = invoke(
        &[
            "htmlcut",
            "extract",
            "--url",
            "http://127.0.0.1:1/?secret=SYNTHETIC_SENTINEL",
            "--css",
            "p",
        ],
        b"",
    );
    assert_eq!(code, 5);
    assert!(out.is_empty());
    assert!(
        !String::from_utf8(error)
            .unwrap()
            .contains("SYNTHETIC_SENTINEL")
    );
}

#[test]
fn t03_explicit_caller_encoding_overrides_conflicting_http_labels() {
    let (url, worker) = serve(vec![response(
        b"<p>\x80</p>",
        "Content-Type: text/html; charset=utf-8; charset=unknown\r\n",
    )]);
    let (code, out, error) = invoke(
        &[
            "htmlcut",
            "extract",
            "--url",
            &url,
            "--encoding",
            "windows-1252",
            "--css",
            "p",
            "--raw",
        ],
        b"",
    );
    assert_eq!((code, out, error), (0, "€".as_bytes().to_vec(), Vec::new()));
    worker.join().unwrap();
}

#[test]
fn quoted_media_parameters_and_informational_metadata_preserve_exact_unicode() {
    let body = "<p>Café €</p>".as_bytes();
    for header in [
        "Content-Type: text/html; note=\"x;charset=windows-1252\"\r\n",
        "Content-Type: text/html; charset=utf-8; note=\"x;charset=windows-1252\"\r\n",
        "Content-Type: text/html; note=\"x;charset=unknown\"; charset=\"u\\tf-8\"\r\n",
    ] {
        let (url, worker) = serve(vec![response(body, header)]);
        assert_eq!(
            invoke(
                &["htmlcut", "extract", "--url", &url, "--css", "p", "--raw"],
                b""
            ),
            (0, "Café €".as_bytes().to_vec(), Vec::new())
        );
        worker.join().unwrap();
    }
    for interim in [
        b"HTTP/1.1 100 Continue\r\nX-Fixture: test\r\nContent-Type: text/html; charset=windows-1252\r\n\r\n".as_slice(),
        b"HTTP/1.1 103 Early Hints\r\nLink: </fixture>\r\n\r\nHTTP/1.1 100 Continue\r\nX-Fixture: test\r\n\r\n".as_slice(),
    ] {
        let mut bytes=interim.to_vec();bytes.extend(response(body,"Content-Type: text/html; charset=utf-8\r\n"));
        let (url,worker)=serve(vec![bytes]);
        assert_eq!(invoke(&["htmlcut","extract","--url",&url,"--css","p","--raw"],b""),(0,"Café €".as_bytes().to_vec(),Vec::new()));
        worker.join().unwrap();
    }
}

#[test]
fn informational_work_and_partial_representations_refuse_before_success_output() {
    let final_response = response(b"<p>A</p>", "Content-Type: text/html\r\n");
    let mut too_many = b"HTTP/1.1 100 Continue\r\nX-Fixture: test\r\n\r\n".repeat(33);
    too_many.extend(&final_response);
    let header = format!(
        "HTTP/1.1 103 Early Hints\r\nX-Padding: {}\r\n\r\n",
        "x".repeat(3000)
    );
    let mut too_large = header.as_bytes().repeat(23);
    too_large.extend(&final_response);
    for (wire,expected_exit,cause) in [
        (too_many,4,None),
        (too_large,4,None),
        (b"HTTP/1.1 206 Partial Content\r\nContent-Range: bytes 0-7/200\r\nContent-Length: 8\r\nConnection: close\r\n\r\n<p>A</p>".to_vec(),5,Some("partial_response")),
        (response(b"<p>A</p>","Content-Range: bytes 0-7/200\r\n"),5,Some("partial_response")),
    ] {
        let (url,worker)=serve(vec![wire]);let (code,stdout,stderr)=invoke(&["htmlcut","extract","--url",&url,"--css","p"],b"");
        assert_eq!(code,expected_exit,"{}",String::from_utf8_lossy(&stderr));assert!(stdout.is_empty());
        if let Some(cause)=cause {assert_eq!(serde_json::from_slice::<serde_json::Value>(&stderr).unwrap()["cause"]["kind"],cause);}
        worker.join().unwrap();
    }
}

#[test]
fn duplicated_origin_charset_metadata_is_refused_only_when_it_controls_decoding() {
    for explicit in [false, true] {
        let (url, worker) = serve(vec![response(
            b"<p>\x80</p>",
            "Content-Type: text/html; charset=utf-8\r\nContent-Type: text/html; charset=unknown\r\n",
        )]);
        let mut args = vec!["htmlcut", "extract", "--url", &url, "--css", "p", "--raw"];
        if explicit {
            args.extend(["--encoding", "windows-1252"]);
        }
        let (code, output, error) = invoke(&args, b"");
        if explicit {
            assert_eq!(
                (code, output, error),
                (0, "€".as_bytes().to_vec(), Vec::new())
            );
        } else {
            assert_eq!(code, 5);
            assert!(output.is_empty());
            assert_eq!(
                serde_json::from_slice::<serde_json::Value>(&error).unwrap()["cause"]["kind"],
                "media_type"
            );
        }
        worker.join().unwrap();
    }
}

#[test]
fn corrupt_compression_and_truncated_bodies_never_publish_partial_success() {
    for (wire, cause, problem) in [
        (
            response(b"not-a-gzip-stream", "Content-Encoding: gzip\r\n"),
            "compression",
            None,
        ),
        (
            b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\nConnection: close\r\n\r\n<p>A</p>".to_vec(),
            "transport",
            Some("incomplete_body"),
        ),
    ] {
        let (url, worker) = serve(vec![wire]);
        let (code, stdout, stderr) = invoke(
            &["htmlcut", "extract", "--url", &url, "--css", "p", "--raw"],
            b"",
        );
        assert_eq!(code, 5, "{}", String::from_utf8_lossy(&stderr));
        assert!(stdout.is_empty());
        let error: serde_json::Value = serde_json::from_slice(&stderr).unwrap();
        assert_eq!(error["cause"]["kind"], cause);
        if let Some(problem) = problem {
            assert_eq!(error["cause"]["problem"], problem);
        }
        assert!(!String::from_utf8_lossy(&stderr).contains("SYNTHETIC_SENTINEL"));
        worker.join().unwrap();
    }
}

#[test]
fn incomplete_redirect_and_final_headers_at_eof_have_safe_framing_diagnostics() {
    for wire in [
        b"HTTP/1.1 302 Found\r\nLocation: /SYNTHETIC_SENTINEL\r\n".to_vec(),
        b"HTTP/1.1 200 OK\r\nContent-Length: 1\r\n".to_vec(),
    ] {
        let (url, worker) = serve(vec![wire]);
        let (code, stdout, stderr) = invoke(
            &["htmlcut", "extract", "--url", &url, "--css", "p", "--raw"],
            b"",
        );
        assert_eq!(code, 5);
        assert!(stdout.is_empty());
        let error: serde_json::Value = serde_json::from_slice(&stderr).unwrap();
        assert_eq!(
            error["cause"],
            serde_json::json!({"kind":"transport","problem":"framing"})
        );
        assert!(!String::from_utf8_lossy(&stderr).contains("SYNTHETIC_SENTINEL"));
        worker.join().unwrap();
    }
}
