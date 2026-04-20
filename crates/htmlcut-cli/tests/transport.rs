mod support;
use support::*;

#[test]
fn url_select_recovers_when_head_preflight_transport_breaks() {
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

            let body = "<html><body><article>Recovered</article></body></html>";
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
    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    command
        .args([
            "select",
            &url,
            "--css",
            "article",
            "--fetch-timeout-ms",
            "250",
        ])
        .assert()
        .success()
        .stdout("Recovered\n");

    server.join().expect("join server");
    assert_eq!(
        methods.lock().expect("lock methods").as_slice(),
        ["HEAD", "GET"]
    );
}

#[test]
fn output_file_writes_the_stdout_payload_without_emitting_stdout() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture(
        tempdir.path(),
        "output-file.html",
        "<article><p>Hello file output</p></article>",
    );
    let output_path = tempdir.path().join("selection.txt");

    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    command
        .args(["select"])
        .arg(&input_path)
        .args(["--css", "article", "--output-file"])
        .arg(&output_path)
        .assert()
        .success()
        .stdout("")
        .stderr("");

    assert_eq!(
        fs::read_to_string(&output_path).expect("read output file"),
        "Hello file output\n"
    );
}

#[test]
fn quiet_suppresses_non_fatal_success_stderr() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture(
        tempdir.path(),
        "quiet.html",
        "<article>First</article><article>Second</article>",
    );

    let mut noisy = Command::cargo_bin("htmlcut").expect("binary");
    noisy
        .args(["select"])
        .arg(&input_path)
        .args(["--css", "article"])
        .assert()
        .success()
        .stdout("First\n")
        .stderr(predicate::str::contains("warning MULTIPLE_MATCHES"));

    let mut quiet = Command::cargo_bin("htmlcut").expect("binary");
    quiet
        .args(["select", "--quiet"])
        .arg(&input_path)
        .args(["--css", "article"])
        .assert()
        .success()
        .stdout("First\n")
        .stderr("");
}
