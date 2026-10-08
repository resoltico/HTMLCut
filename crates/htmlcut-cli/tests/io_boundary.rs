// SPDX-License-Identifier: MPL-2.0
//! Real process and descriptor assertions; mocked writes cannot establish delivery.
use std::fs::File;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn binary() -> std::path::PathBuf {
    std::env::var_os("HTMLCUT_TEST_BINARY")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| env!("CARGO_BIN_EXE_htmlcut").into())
}

fn command() -> Command {
    Command::new(binary())
}

#[cfg(target_os = "macos")]
#[path = "io_boundary/io_replacement.rs"]
mod replacement;

#[cfg(unix)]
#[path = "io_boundary/io_file_kinds.rs"]
mod file_kinds;

#[cfg(windows)]
#[test]
fn native_console_unicode_and_mixed_streams_preserve_values_and_error_channels() {
    let root = tempfile::tempdir().unwrap();
    let evidence = root.path().join("windows-stdio.json");
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../scripts/native-windows-stdio.ps1");
    let mut child = Command::new("powershell.exe")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
        .arg(script)
        .arg("-Binary")
        .arg(binary())
        .arg("-Evidence")
        .arg(&evidence)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(120);
    while child.try_wait().unwrap().is_none() {
        if Instant::now() >= deadline {
            child.kill().unwrap();
            let _ = child.wait();
            panic!("native Windows console fixture exceeded its process bound");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let output = child.wait_with_output().unwrap();
    let proof = std::fs::read_to_string(&evidence).unwrap_or_default();
    if let Some(destination) = std::env::var_os("HTMLCUT_WINDOWS_STDIO_EVIDENCE")
        && evidence.is_file()
    {
        std::fs::copy(&evidence, destination).unwrap();
    }
    assert!(
        output.status.success(),
        "{proof}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let proof: serde_json::Value = serde_json::from_str(&proof).unwrap();
    assert_eq!(proof["passed"], true);
    assert_eq!(proof["rows"].as_array().unwrap().len(), 8);
}

#[cfg(windows)]
#[test]
fn native_named_pipe_paths_are_refused_for_every_file_role() {
    let root = tempfile::tempdir().unwrap();
    let evidence = root.path().join("windows-pipe-input.json");
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../scripts/native-windows-pipe-input.ps1");
    let mut child = Command::new("powershell.exe")
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
        .arg(script)
        .arg("-Binary")
        .arg(binary())
        .arg("-Evidence")
        .arg(&evidence)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(30);
    while child.try_wait().unwrap().is_none() {
        if Instant::now() >= deadline {
            child.kill().unwrap();
            let _ = child.wait();
            panic!("native named-pipe fixture exceeded its bound");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let output = child.wait_with_output().unwrap();
    let proof = std::fs::read_to_string(&evidence).unwrap_or_default();
    if let Some(destination) = std::env::var_os("HTMLCUT_WINDOWS_PIPE_EVIDENCE")
        && evidence.is_file()
    {
        std::fs::copy(&evidence, destination).unwrap();
    }
    assert!(
        output.status.success(),
        "{proof}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let proof: serde_json::Value = serde_json::from_str(&proof).unwrap();
    assert_eq!(proof["passed"], true);
    assert_eq!(proof["rows"].as_array().unwrap().len(), 3);
}

#[test]
fn readonly_stdout_never_reports_nonempty_delivery_success() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source.html");
    std::fs::write(&source, "<p>Hello</p>").unwrap();
    let sink = root.path().join("sink.txt");
    std::fs::write(&sink, "KEEP").unwrap();
    let routes = vec![
        vec![
            "extract",
            "--file",
            source.to_str().unwrap(),
            "--select",
            "p",
        ],
        vec![
            "extract",
            "--file",
            source.to_str().unwrap(),
            "--select",
            "p",
            "--raw",
        ],
        vec!["--help"],
        vec!["--version"],
        vec!["extract", "--help"],
        vec!["schema", "htmlcut.extraction.plan"],
    ];
    for args in routes {
        let output = command()
            .args(&args)
            .stdout(File::open(&sink).unwrap())
            .stderr(Stdio::piped())
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(5),
            "{args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(std::fs::read(&sink).unwrap(), b"KEEP");
        let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(error["code"], "publication");
    }
    let result = root.path().join("result.txt");
    let output = command()
        .args([
            "extract",
            "--file",
            source.to_str().unwrap(),
            "--select",
            "p",
            "--raw",
            "--output",
            result.to_str().unwrap(),
        ])
        .stdout(File::open(&sink).unwrap())
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(std::fs::read(&result).unwrap(), b"Hello");
    std::fs::write(&source, "<p></p>").unwrap();
    let output = command()
        .args([
            "extract",
            "--file",
            source.to_str().unwrap(),
            "--select",
            "p",
            "--raw",
        ])
        .stdout(File::open(&sink).unwrap())
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(std::fs::read(&sink).unwrap(), b"KEEP");
}

#[cfg(unix)]
#[test]
fn special_file_paths_fail_without_waiting_for_a_writer() {
    let root = tempfile::tempdir().unwrap();
    let fifo = root.path().join("input");
    assert!(
        Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .unwrap()
            .success()
    );
    for args in [
        vec!["extract", "--file", fifo.to_str().unwrap(), "--select", "p"],
        vec!["extract", "--stdin", "--plan", fifo.to_str().unwrap()],
    ] {
        let mut child = command()
            .args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            if child.try_wait().unwrap().is_some() {
                break;
            }
            if Instant::now() > deadline {
                child.kill().unwrap();
                let _ = child.wait();
                panic!("nonregular input blocked: {args:?}");
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        let output = child.wait_with_output().unwrap();
        assert_eq!(output.status.code(), Some(5));
        assert!(output.stdout.is_empty());
        let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(error["cause"]["problem"], "unsupported_kind");
    }
}

#[test]
fn unreadable_stdin_and_undeliverable_diagnostics_are_real_failures() {
    let root = tempfile::tempdir().unwrap();
    let sink = root.path().join("sink");
    let output = command()
        .args(["extract", "--stdin", "--select", "p"])
        .stdin(File::create(&sink).unwrap())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(5));
    assert!(output.stdout.is_empty());
    let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["cause"]["operation"], "input");
    #[cfg(unix)]
    assert_eq!(error["cause"]["problem"], "invalid_descriptor");

    std::fs::write(&sink, "KEEP").unwrap();
    let output = command()
        .arg("--UNDECLARED_SECRET")
        .stderr(File::open(&sink).unwrap())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(5));
    assert!(output.stdout.is_empty());
    assert_eq!(std::fs::read(&sink).unwrap(), b"KEEP");
}

#[cfg(unix)]
#[test]
fn inherited_closed_descriptors_are_repaired_by_the_pinned_runtime() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source.html");
    std::fs::write(&source, "<p>Hello</p>").unwrap();
    for (close, args, expected) in [
        ("exec 1>&-; exec \"$@\"", vec!["--version"], 0),
        (
            "exec 0<&-; exec \"$@\"",
            vec!["extract", "--stdin", "--select", "p"],
            3,
        ),
        ("exec 2>&-; exec \"$@\"", vec!["--UNDECLARED_SECRET"], 2),
        (
            "exec 0<&-; exec \"$@\"",
            vec![
                "extract",
                "--file",
                source.to_str().unwrap(),
                "--select",
                "p",
                "--raw",
            ],
            0,
        ),
    ] {
        let output = Command::new("bash")
            .args(["-c", close, "descriptor-test"])
            .arg(binary())
            .args(&args)
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(expected),
            "{args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        if args.contains(&"--file") {
            assert_eq!(output.stdout, b"Hello");
        } else {
            assert!(output.stdout.is_empty());
        }
    }
}

#[cfg(target_os = "macos")]
#[test]
fn descriptor_closed_at_delivery_is_not_sanitized_into_success() {
    let root = tempfile::tempdir().unwrap();
    let fixture = root.path().join("closed-writer.dylib");
    let compiler = Command::new("clang")
        .args(["-Wall", "-Wextra", "-Werror", "-dynamiclib"])
        .arg(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/closed-writer.c"),
        )
        .arg("-o")
        .arg(&fixture)
        .output()
        .unwrap();
    assert!(
        compiler.status.success(),
        "{}",
        String::from_utf8_lossy(&compiler.stderr)
    );
    let source = root.path().join("source.html");
    std::fs::write(&source, "<p>Hello</p>").unwrap();
    for args in [
        vec!["--version"],
        vec!["--help"],
        vec!["extract", "--help"],
        vec!["schema", "htmlcut.extraction.plan"],
        vec![
            "extract",
            "--file",
            source.to_str().unwrap(),
            "--select",
            "p",
        ],
        vec![
            "extract",
            "--file",
            source.to_str().unwrap(),
            "--select",
            "p",
            "--raw",
        ],
    ] {
        let output = command()
            .args(&args)
            .env("DYLD_INSERT_LIBRARIES", &fixture)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(5), "{args:?}");
        assert!(output.stdout.is_empty());
        let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(error["cause"]["operation"], "stdout");
        assert_eq!(error["cause"]["problem"], "invalid_descriptor");
    }
}

#[test]
fn a_reader_disappearing_after_a_real_prefix_prevents_delivery_success() {
    use std::io::Read;
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("source.html");
    std::fs::write(&source, format!("<p>{}</p>", "x".repeat(1_000_000))).unwrap();
    let (mut reader, writer) = os_pipe::pipe().unwrap();
    let mut child = command()
        .args([
            "extract",
            "--file",
            source.to_str().unwrap(),
            "--select",
            "p",
            "--raw",
        ])
        .stdout(writer)
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let (sender, receiver) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        let mut bytes = vec![0; 4096];
        let read = reader.read(&mut bytes).unwrap();
        bytes.truncate(read);
        drop(reader);
        sender.send(bytes).unwrap();
    });
    let prefix = match receiver.recv_timeout(Duration::from_secs(5)) {
        Ok(bytes) => bytes,
        Err(error) => {
            child.kill().unwrap();
            let _ = child.wait();
            panic!("no bounded output prefix: {error}");
        }
    };
    worker.join().unwrap();
    assert!(!prefix.is_empty());
    assert!(prefix.iter().all(|byte| *byte == b'x'));
    let deadline = Instant::now() + Duration::from_secs(5);
    while child.try_wait().unwrap().is_none() {
        if Instant::now() >= deadline {
            child.kill().unwrap();
            let _ = child.wait();
            panic!("writer did not terminate after reader closed");
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(5));
    let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["cause"]["operation"], "stdout");
    assert_eq!(error["cause"]["problem"], "broken_pipe");
}
