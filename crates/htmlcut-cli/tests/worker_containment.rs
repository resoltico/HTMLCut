// SPDX-License-Identifier: MPL-2.0
//! Execute the delivered binary's native policy and supervisor negative controls.
use std::{
    process::{Command, Stdio},
    time::{Duration, Instant},
};
fn binary() -> std::ffi::OsString {
    std::env::var_os("HTMLCUT_TEST_BINARY").unwrap_or_else(|| env!("CARGO_BIN_EXE_htmlcut").into())
}
#[test]
fn native_memory_policy_refuses_an_allocation_larger_than_allowance() {
    let output = Command::new(binary())
        .env("HTMLCUT_PRIVATE_WORKER", "verify-memory")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "policy did not refuse allocation: {output:?}"
    );
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
}
#[test]
fn supervision_rejects_floods_abnormal_exit_and_invalid_frames_without_data() {
    for mode in ["stdout-flood", "stderr-flood", "abnormal", "malformed"] {
        let output = Command::new(binary())
            .env("HTMLCUT_PRIVATE_WORKER", format!("verify-{mode}"))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "supervisor falsely accepted {mode}: {output:?}"
        );
        assert!(output.stdout.is_empty());
        assert!(output.stderr.is_empty());
    }
}
#[test]
fn deadline_kills_and_reaps_a_worker_that_never_finishes() {
    let start = Instant::now();
    let output = Command::new(binary())
        .env("HTMLCUT_PRIVATE_WORKER", "verify-timeout")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
    assert!(start.elapsed() >= Duration::from_secs(9));
    assert!(start.elapsed() < Duration::from_secs(20));
}
#[test]
fn historical_parser_stress_cannot_replace_an_existing_destination() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("source.html");
    let target = directory.path().join("result.json");
    std::fs::write(
        &source,
        format!("{}{}", "<b>".repeat(70_000), "<p>X</b>Y".repeat(70_000)),
    )
    .unwrap();
    std::fs::write(&target, b"old result").unwrap();
    let output = Command::new(binary())
        .args(["extract", "--file"])
        .arg(&source)
        .args(["--select", "b", "--output"])
        .arg(&target)
        .arg("--overwrite")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert_eq!(std::fs::read(target).unwrap(), b"old result");
    assert!(!String::from_utf8_lossy(&output.stderr).contains("source.html"));
}

#[cfg(unix)]
#[test]
fn killing_the_supervisor_does_not_leave_a_running_worker() {
    let mut supervisor = Command::new(binary())
        .env("HTMLCUT_PRIVATE_WORKER", "verify-timeout")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let parent = supervisor.id();
    let deadline = Instant::now() + Duration::from_secs(3);
    let worker = loop {
        let output = Command::new("ps")
            .args(["-axo", "pid=,ppid="])
            .output()
            .unwrap();
        let found = String::from_utf8_lossy(&output.stdout)
            .lines()
            .find_map(|line| {
                let mut words = line.split_whitespace();
                let pid = words.next()?.parse::<u32>().ok()?;
                let ppid = words.next()?.parse::<u32>().ok()?;
                (ppid == parent).then_some(pid)
            });
        if let Some(pid) = found {
            break pid;
        }
        assert!(Instant::now() < deadline, "private worker did not start");
        std::thread::sleep(Duration::from_millis(20));
    };
    // Wait until the worker installs its independent lifetime guard.
    std::thread::sleep(Duration::from_millis(200));
    supervisor.kill().unwrap();
    supervisor.wait().unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        let output = Command::new("ps")
            .args(["-p", &worker.to_string(), "-o", "stat="])
            .output()
            .unwrap();
        let state = String::from_utf8_lossy(&output.stdout);
        // An exited zombie awaits OS adoption/reaping; it performs no untrusted work.
        if state.trim().is_empty() || state.trim().starts_with('Z') {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "orphan worker {worker} remains running: {state}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn maximum_size_snapshot_remains_usable_under_native_policy() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("large.html");
    let prefix = b"<h1>accepted</h1><script>";
    let suffix = b"</script>";
    let mut bytes = prefix.to_vec();
    bytes.resize(htmlcut_core::MAX_SOURCE_BYTES - suffix.len(), b'x');
    bytes.extend_from_slice(suffix);
    std::fs::write(&source, &bytes).unwrap();
    drop(bytes);
    let output = Command::new(binary())
        .args(["extract", "--file"])
        .arg(source)
        .args(["--select", "h1"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "maximum admitted snapshot refused: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"[\"accepted\"]\n");
}

#[test]
fn native_policy_refuses_touched_memory_pressure_while_supervisor_survives() {
    let output = Command::new(binary())
        .env("HTMLCUT_PRIVATE_WORKER", "verify-pressure")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "native memory pressure did not prove refusal: {output:?}"
    );
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
}

#[test]
fn deadline_breaks_a_blocked_snapshot_pipe_and_reaps_the_worker() {
    let start = Instant::now();
    let output = Command::new(binary())
        .env("HTMLCUT_PRIVATE_WORKER", "verify-blocked-input")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "blocked source pump was not recovered: {output:?}"
    );
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
    assert!(start.elapsed() >= Duration::from_secs(9));
    assert!(start.elapsed() < Duration::from_secs(20));
}

#[test]
fn output_aliases_are_compared_by_physical_identity_not_content() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("source.html");
    let alias = directory.path().join("alias.html");
    let distinct = directory.path().join("distinct.html");
    let html = b"<p>complete</p>";
    std::fs::write(&source, html).unwrap();
    std::fs::hard_link(&source, &alias).unwrap();
    std::fs::write(&distinct, html).unwrap();
    let refusal = Command::new(binary())
        .args(["extract", "--file"])
        .arg(&source)
        .args(["--select", "p", "--output"])
        .arg(&alias)
        .arg("--overwrite")
        .output()
        .unwrap();
    assert_eq!(refusal.status.code(), Some(2));
    assert!(refusal.stdout.is_empty());
    assert_eq!(std::fs::read(&source).unwrap(), html);
    assert_eq!(std::fs::read(&alias).unwrap(), html);
    let error: serde_json::Value = serde_json::from_slice(&refusal.stderr).unwrap();
    assert_eq!(error["code"], "invalid_options");
    assert!(!String::from_utf8_lossy(&refusal.stderr).contains("source.html"));
    let success = Command::new(binary())
        .args(["extract", "--file"])
        .arg(&source)
        .args(["--select", "p", "--output"])
        .arg(&distinct)
        .arg("--overwrite")
        .output()
        .unwrap();
    assert!(success.status.success());
    assert_eq!(std::fs::read(distinct).unwrap(), b"[\"complete\"]\n");
    assert_eq!(std::fs::read(source).unwrap(), html);
}
