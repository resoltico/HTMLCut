use super::*;

fn helper(mode: &str) -> Command {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command.args([
        "--exact",
        "docs::commands::process::tests::subprocess_fixture",
        "--ignored",
        "--nocapture",
    ]);
    command.env("HTMLCUT_DOCS_PROCESS_TEST", mode);
    command
}

#[test]
#[ignore = "only executed as a child process by the capture boundary tests"]
fn subprocess_fixture() {
    use std::io::Read;
    match std::env::var("HTMLCUT_DOCS_PROCESS_TEST").unwrap().as_str() {
        "echo" => {
            let mut bytes = Vec::new();
            std::io::stdin().read_to_end(&mut bytes).unwrap();
            std::io::stdout().write_all(&bytes).unwrap();
            std::io::stderr().write_all(b"diagnostic").unwrap();
        }
        "wait" => std::thread::sleep(Duration::from_secs(5)),
        "large" => {
            for _ in 0..100 {
                let _ = std::io::stdout().write_all(&vec![b'x'; 64 * 1024]);
                std::thread::sleep(Duration::from_millis(1));
            }
        }
        "failure" => std::process::exit(17),
        _ => panic!("unknown child fixture"),
    }
}

#[test]
fn subprocess_capture_handles_stdin_status_and_finite_capture_deadlines() {
    let output = capture(
        &mut helper("echo"),
        Some(b"input sentinel"),
        Duration::from_secs(10),
    )
    .unwrap();
    assert_eq!(output.code, 0);
    assert!(output.stdout.windows(14).any(|v| v == b"input sentinel"));
    assert!(output.stderr.ends_with(b"diagnostic"));
    let output = capture(&mut helper("failure"), None, Duration::from_secs(10)).unwrap();
    assert_eq!(output.code, 17);
    let started = Instant::now();
    let error = capture(&mut helper("wait"), None, Duration::from_millis(100))
        .err()
        .unwrap();
    assert!(error.to_string().contains("capture or time budget"));
    assert!(started.elapsed() < Duration::from_secs(3));
    let error = capture(&mut helper("large"), None, Duration::from_secs(10))
        .err()
        .unwrap();
    assert!(error.to_string().contains("capture"));
}

#[test]
fn oversized_input_fails_before_spawn_and_completed_output_is_still_bounded() {
    let mut absent = Command::new("htmlcut-nonexistent-docs-test-829317");
    let error = capture(&mut absent, Some(&vec![b'x'; 4097]), Duration::from_secs(1))
        .err()
        .unwrap();
    assert!(error.to_string().contains("stdin fixture exceeded"));
    assert_eq!(
        capture(&mut absent, None, Duration::from_secs(1))
            .err()
            .unwrap()
            .kind(),
        io::ErrorKind::NotFound
    );
    let status = helper("failure")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();
    let root = htmlcut_tempdir::tempdir().unwrap();
    let stdout = root.path().join("stdout");
    let stderr = root.path().join("stderr");
    for large in [&stdout, &stderr] {
        fs::write(&stdout, b"normal").unwrap();
        fs::write(&stderr, b"normal").unwrap();
        File::create(large)
            .unwrap()
            .set_len(MAX_CAPTURE_BYTES + 1)
            .unwrap();
        assert!(
            finish(status, &stdout, &stderr)
                .err()
                .unwrap()
                .to_string()
                .contains("capture budget")
        );
    }
}

#[test]
fn failed_build_status_and_missing_binary_cannot_be_cached_as_success() {
    let root = htmlcut_tempdir::tempdir().unwrap();
    let path = root.path().join("htmlcut");
    for mode in ["failure", "echo"] {
        let error = build_with(&mut helper(mode), path.clone()).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("Could not build the maintained CLI")
        );
    }
    fs::write(&path, b"fixture build output").unwrap();
    assert_eq!(build_with(&mut helper("echo"), path.clone()).unwrap(), path);
}

#[test]
fn actual_subprocess_roots_honor_empty_relative_and_absolute_overrides() {
    let root = std::env::current_dir().unwrap();
    let fallback = root.join("fallback");
    assert_eq!(override_root(&root, None, fallback.clone()), fallback);
    assert_eq!(
        override_root(&root, Some(std::ffi::OsStr::new("")), fallback.clone()),
        fallback
    );
    assert_eq!(
        override_root(
            &root,
            Some(std::ffi::OsStr::new("target")),
            fallback.clone()
        ),
        root.join("target")
    );
    let absolute = root.join("absolute");
    assert_eq!(
        override_root(&root, Some(absolute.as_os_str()), fallback),
        absolute
    );
}
