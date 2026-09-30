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
        "shortwait" => std::thread::sleep(Duration::from_millis(250)),
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

#[test]
fn optional_fixture_broken_pipe_preserves_consumer_exit_status_and_other_errors() {
    struct Failure(io::ErrorKind);
    impl Write for Failure {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::from(self.0))
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    write_fixture(&mut Failure(io::ErrorKind::BrokenPipe), b"unused").unwrap();
    assert_eq!(
        write_fixture(&mut Failure(io::ErrorKind::PermissionDenied), b"input")
            .unwrap_err()
            .kind(),
        io::ErrorKind::PermissionDenied
    );
    assert_eq!(
        capture(
            &mut helper("failure"),
            Some(b"unused fixture"),
            Duration::from_secs(10)
        )
        .unwrap()
        .code,
        17
    );
}

#[test]
fn owned_process_cleanup_reaps_even_when_termination_reports_an_error() {
    use std::cell::RefCell;
    use std::rc::Rc;
    struct Process {
        calls: Rc<RefCell<Vec<&'static str>>>,
        failure: bool,
    }
    impl Lifecycle for Process {
        fn terminate(&mut self) -> io::Result<()> {
            self.calls.borrow_mut().push("terminate");
            if self.failure {
                Err(io::Error::other("termination error"))
            } else {
                Ok(())
            }
        }
        fn reap(&mut self) -> io::Result<()> {
            self.calls.borrow_mut().push("reap");
            Ok(())
        }
    }
    for failure in [false, true] {
        let calls = Rc::new(RefCell::new(Vec::new()));
        {
            let _running = Running(Process {
                calls: Rc::clone(&calls),
                failure,
            });
        }
        assert_eq!(*calls.borrow(), ["terminate", "reap"]);
    }
}

#[test]
fn native_reap_waits_for_child_completion() {
    let mut child = helper("shortwait")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    Lifecycle::reap(&mut child).unwrap();
    assert!(
        child
            .try_wait()
            .unwrap()
            .is_some_and(|status| status.success())
    );
}

#[test]
fn exact_fixture_and_completed_capture_byte_bounds_are_inclusive() {
    let input = vec![b'x'; 4096];
    let captured = capture(&mut helper("echo"), Some(&input), Duration::from_secs(10)).unwrap();
    assert_eq!(captured.code, 0);
    assert!(
        captured
            .stdout
            .windows(input.len())
            .any(|bytes| bytes == input)
    );
    let status = helper("echo")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap();
    let root = htmlcut_tempdir::tempdir().unwrap();
    let stdout = root.path().join("stdout");
    let stderr = root.path().join("stderr");
    fs::write(&stderr, b"").unwrap();
    for (length, success) in [(2_097_151, true), (2_097_152, true), (2_097_153, false)] {
        File::create(&stdout).unwrap().set_len(length).unwrap();
        let result = finish(status, &stdout, &stderr);
        assert_eq!(result.is_ok(), success);
        if success {
            assert_eq!(result.unwrap().stdout.len(), length as usize);
        }
    }
}
