// SPDX-License-Identifier: MPL-2.0
//! The disposable executable must finish even while native streams remain open.
#[path = "../src/test_pipe.rs"]
mod test_pipe;
use std::{
    process::{Command, Output, Stdio},
    time::{Duration, Instant},
};
use test_pipe::fill_pipe;
fn binary() -> std::ffi::OsString {
    std::env::var_os("HTMLCUT_TEST_BINARY").unwrap_or_else(|| env!("CARGO_BIN_EXE_htmlcut").into())
}
fn finish(mut child: std::process::Child) -> Output {
    let deadline = Instant::now() + Duration::from_secs(3);
    while child.try_wait().unwrap().is_none() {
        if Instant::now() >= deadline {
            let _ = child.kill();
            let output = child.wait_with_output().unwrap();
            panic!("executable did not terminate itself: {output:?}");
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    child.wait_with_output().unwrap()
}
#[test]
fn held_input_and_blocked_stderr_cannot_prevent_timeout_or_invalid_request_exit() {
    for arguments in [vec!["get", "h1", "--timeout", ".05"], vec!["run", "{"]] {
        let expected = if arguments[0] == "get" { 4 } else { 2 };
        let (reader, mut writer) = std::io::pipe().unwrap();
        fill_pipe(&mut writer);
        let child_writer = writer.try_clone().unwrap();
        let mut child = Command::new(binary())
            .args(arguments)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(child_writer)
            .spawn()
            .unwrap();
        let input = child.stdin.take().unwrap();
        let output = finish(child);
        assert_eq!(output.status.code(), Some(expected), "{output:?}");
        assert!(output.stdout.is_empty());
        drop(input);
        drop(writer);
        drop(reader);
    }
}
#[test]
fn readable_stderr_receives_exactly_one_complete_timeout_diagnostic() {
    let mut child = Command::new(binary())
        .args(["get", "h1", "--timeout", ".05"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let input = child.stdin.take().unwrap();
    let output = finish(child);
    drop(input);
    assert_eq!(output.status.code(), Some(4));
    assert!(output.stdout.is_empty());
    assert_eq!(output.stderr.iter().filter(|b| **b == b'\n').count(), 1);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&output.stderr).unwrap()["limit"],
        "time"
    );
}
#[cfg(unix)]
#[test]
fn unconnected_input_contract_and_output_fifos_obey_the_executable_deadline() {
    let root = tempfile::tempdir().unwrap();
    let page = root.path().join("page.html");
    std::fs::write(&page, b"<h1>complete</h1>").unwrap();
    let fifo = root.path().join("fifo");
    let status = Command::new("mkfifo").arg(&fifo).status().unwrap();
    assert!(status.success());
    for role in ["input", "contract", "output"] {
        let mut command = Command::new(binary());
        match role {
            "input" => {
                command.args(["get", "h1"]).arg(&fifo);
            }
            "contract" => {
                command.args(["run", "--from-file"]).arg(&fifo).arg(&page);
            }
            "output" => {
                command.args(["get", "h1"]).arg(&page).arg("-o").arg(&fifo);
            }
            _ => unreachable!(),
        };
        let child = command
            .args(["--timeout", ".05"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let output = finish(child);
        assert_eq!(output.status.code(), Some(4), "{role}: {output:?}");
        assert!(output.stdout.is_empty());
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&output.stderr).unwrap()["limit"],
            "time"
        );
    }
}

#[test]
fn a_consumer_holding_stdout_open_cannot_block_hard_expiry() {
    use std::io::Read;
    let root = tempfile::tempdir().unwrap();
    let page = root.path().join("large.html");
    std::fs::write(&page, format!("<p>{}</p>", "x".repeat(1024 * 1024))).unwrap();
    let mut child = Command::new(binary())
        .args(["get", "p"])
        .arg(page)
        .args(["--timeout", "2"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut consumer = child.stdout.take().unwrap();
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut prefix = [0];
        let result = consumer.read_exact(&mut prefix);
        let _ = sender.send((result, prefix, consumer));
    });
    let (read, prefix, consumer) = match receiver.recv_timeout(Duration::from_secs(4)) {
        Ok(ready) => ready,
        Err(error) => {
            let _ = child.kill();
            let output = child.wait_with_output().unwrap();
            panic!("stdout readiness was not observed: {error}; {output:?}");
        }
    };
    read.expect("the production binary must begin delivery before expiry");
    assert_eq!(&prefix, b"\"");
    let output = finish(child);
    drop(consumer);
    assert_eq!(output.status.code(), Some(4), "{output:?}");
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&output.stderr).unwrap()["limit"],
        "time"
    );
}
