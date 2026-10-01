use std::io::{Read, Write};
use std::process::{Command, Stdio};

#[test]
fn runtime_url_environment_references_are_persisted_without_their_values() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!(
        "http://{}/?token=SYNTHETIC_RUNTIME_SECRET",
        listener.local_addr().unwrap()
    );
    listener.set_nonblocking(true).unwrap();
    let server = std::thread::spawn(move || {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let (mut stream, _) = loop {
            match listener.accept() {
                Ok(pair) => break pair,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(std::time::Instant::now() < deadline);
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
                Err(error) => panic!("accept failed: {error}"),
            }
        };
        stream.set_nonblocking(false).unwrap();
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(3)))
            .unwrap();
        let mut request = Vec::new();
        while !request.ends_with(b"\r\n\r\n") {
            let mut byte = [0];
            assert_eq!(stream.read(&mut byte).unwrap(), 1);
            request.push(byte[0]);
            assert!(request.len() < 16384);
        }
        assert!(
            String::from_utf8(request)
                .unwrap()
                .starts_with("GET /?token=SYNTHETIC_RUNTIME_SECRET HTTP/1.1")
        );
        stream
            .write_all(
                b"HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Length: 10\r\n\r\n<p>180</p>",
            )
            .unwrap();
    });
    let root = htmlcut_tempdir::tempdir().unwrap();
    let run = root.path().join("run.json");
    let output = Command::new(env!("CARGO_BIN_EXE_htmlcut"))
        .args([
            "extract",
            "--url-env",
            "HTMLCUT_TEST_SOURCE",
            "--css",
            "p",
            "--save-run",
            run.to_str().unwrap(),
        ])
        .env("HTMLCUT_TEST_SOURCE", url)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    server.join().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let saved = std::fs::read_to_string(run).unwrap();
    assert!(saved.contains("HTMLCUT_TEST_SOURCE"));
    assert!(!saved.contains("SYNTHETIC_RUNTIME_SECRET"));
    assert!(
        !String::from_utf8(output.stdout)
            .unwrap()
            .contains("SYNTHETIC_RUNTIME_SECRET")
    );
}
