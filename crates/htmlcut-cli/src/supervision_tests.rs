// SPDX-License-Identifier: MPL-2.0
//! Instrumented ownership proofs use the production CLI supervisor. Shipped
//! binary stream/authorization evidence remains in CLI integration tests.
use super::*;
use std::{io, time::Instant};
struct TestChild<'a> {
    child: Option<std::process::Child>,
    fixture: Option<&'a mut tempfile::TempDir>,
}
impl TestChild<'_> {
    fn try_wait(&mut self) -> io::Result<Option<std::process::ExitStatus>> {
        self.child.as_mut().unwrap().try_wait()
    }
}
impl Drop for TestChild<'_> {
    fn drop(&mut self) {
        let Some(mut child) = self.child.take() else {
            return;
        };
        let pid = child.id();
        let kill_error = child.kill().err();
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            match child.try_wait() {
                Ok(Some(_)) => {
                    // These selected test children are leaves. Reaping
                    // closes all writers before their fixture can drop.
                    let output = child.wait_with_output();
                    eprintln!("Test child {pid} stopped and reaped: {output:?}");
                    return;
                }
                Ok(None) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(1))
                }
                _ => break,
            }
        }
        let retained = self.fixture.as_mut().map(|fixture| {
            fixture.disable_cleanup(true);
            fixture.path().to_path_buf()
        });
        eprintln!(
            "Test child {pid} cleanup unconfirmed; kill error {kill_error:?}; retained fixture {retained:?}"
        );
    }
}
fn wait_for_test_child(
    mut child: TestChild<'_>,
    description: &str,
    deadline: Instant,
) -> std::process::Output {
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return child.child.take().unwrap().wait_with_output().unwrap(),
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(1)),
            outcome => panic!("{description} did not terminate: {outcome:?}"),
        }
    }
}

#[test]
fn publication_phase_child() {
    let Some(destination) = std::env::var_os("HTMLCUT_PUBLICATION_DESTINATION") else {
        return;
    };
    std::panic::set_hook(Box::new(|_| {}));
    let job = Job::new(Some(Duration::from_secs(2))).unwrap();
    supervise(job.control()).unwrap();
    let result = completion(job.present(
        b"complete-private-result",
        Destination::File(destination.into()),
    ));
    if let Err(error) = result {
        report(&error);
        std::process::exit(exit_status(error.error));
    }
}
#[test]
fn local_creation_handoffs_clean_owned_files_and_commit_wins_expiry() {
    for phase in [
        "reserved",
        "created",
        "restricted",
        "filled",
        "metadata",
        "policy",
        "precommit",
        "commit",
    ] {
        let mut root = tempfile::tempdir().unwrap();
        let destination = root.path().join("result.json");
        let ready = root.path().join("ready");
        std::fs::write(&destination, b"old").unwrap();
        let child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "run::tests::publication_phase_child",
                "--nocapture",
            ])
            .env("HTMLCUT_PUBLICATION_PHASE", phase)
            .env("HTMLCUT_PUBLICATION_COMMIT_HOLD_MS", "2500")
            .env("HTMLCUT_PUBLICATION_DESTINATION", &destination)
            .env("HTMLCUT_PUBLICATION_READY", &ready)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let child = TestChild {
            child: Some(child),
            fixture: Some(&mut root),
        };
        let started = Instant::now();
        while !ready.exists() && started.elapsed() < Duration::from_secs(6) {
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(ready.exists(), "checkpoint {phase} was not reached");
        let stage = std::path::PathBuf::from(std::fs::read_to_string(&ready).unwrap());
        if matches!(phase, "created" | "restricted" | "filled") {
            assert!(
                std::fs::read(&stage).is_err(),
                "{phase} stage is readable before completed policy"
            );
        }
        let result = wait_for_test_child(
            child,
            "publication checkpoint",
            started + Duration::from_secs(6),
        );
        if phase == "commit" {
            assert!(result.status.success(), "{result:?}");
            assert_eq!(
                std::fs::read(&destination).unwrap(),
                b"complete-private-result"
            );
            assert!(result.stderr.is_empty());
        } else {
            assert_eq!(result.status.code(), Some(4), "{result:?}");
            assert_eq!(std::fs::read(&destination).unwrap(), b"old");
            // Deadline diagnostics are best effort within the reporting grace.
            assert!(!stage.exists());
        }
        assert_eq!(
            std::fs::read_dir(root.path()).unwrap().count(),
            2,
            "{phase} stranded owned staging"
        );
    }
}

#[test]
fn unwind_cleanup_remains_supervised_and_early_disarm_is_detected() {
    const CHILD: &str = "HTMLCUT_CLI_UNWIND_CONTROL";
    if let Ok(mode) = std::env::var(CHILD) {
        std::panic::set_hook(Box::new(|_| {}));
        let path =
            std::path::PathBuf::from(std::env::var_os("HTMLCUT_PUBLICATION_DESTINATION").unwrap());
        // Ordinary cleanup must complete before expiry: its two-second parent
        // bound is strictly shorter than this operation's ten-second deadline.
        let duration = if mode == "ordinary" {
            Duration::from_secs(10)
        } else {
            Duration::from_secs(1)
        };
        let job = Job::new(Some(duration)).unwrap();
        supervise(job.control()).unwrap();
        let result = completion(job.present(b"complete-private-result", Destination::File(path)));
        let error = result.unwrap_err();
        report(&error);
        std::process::exit(exit_status(error.error));
    }
    for mode in ["held", "ordinary", "early-disarm"] {
        let mut root = tempfile::tempdir().unwrap();
        let output = root.path().join("result");
        let ready = root.path().join("ready");
        std::fs::write(&output, b"old").unwrap();
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "run::tests::unwind_cleanup_remains_supervised_and_early_disarm_is_detected",
                "--nocapture",
            ])
            .env(CHILD, mode)
            .env("HTMLCUT_PUBLICATION_DESTINATION", &output)
            .env("HTMLCUT_PUBLICATION_PANIC_AT", "restricted")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped());
        if mode != "ordinary" {
            command
                .env("HTMLCUT_PUBLICATION_PHASE", "cleanup")
                .env("HTMLCUT_PUBLICATION_READY", &ready);
        }
        if mode == "early-disarm" {
            command.env("HTMLCUT_TEST_EARLY_COMPLETION", "1");
        }
        let mut child = TestChild {
            child: Some(command.spawn().unwrap()),
            fixture: Some(&mut root),
        };
        let started = Instant::now();
        if mode != "ordinary" {
            while !ready.exists() && started.elapsed() < Duration::from_secs(2) {
                std::thread::sleep(Duration::from_millis(1));
            }
            assert!(ready.exists(), "owned unwind cleanup was not entered");
        }
        if mode == "early-disarm" {
            let limit = Instant::now() + Duration::from_secs(2);
            while child.try_wait().unwrap().is_none() && Instant::now() < limit {
                std::thread::sleep(Duration::from_millis(1));
            }
            assert!(
                child.try_wait().unwrap().is_none(),
                "negative control unexpectedly enforced the deadline"
            );
            // TestChild kills/reaps only its own failed-control process before cleanup.
            drop(child);
        } else {
            let result = wait_for_test_child(
                child,
                "production unwind supervisor",
                started + Duration::from_secs(2),
            );
            assert_eq!(
                result.status.code(),
                Some(if mode == "held" { 4 } else { 6 }),
                "{mode}: {result:?}"
            );
            let error: serde_json::Value = serde_json::from_slice(&result.stderr).unwrap();
            assert_eq!(
                error["error"],
                if mode == "held" {
                    "limit_exceeded"
                } else {
                    "internal"
                }
            );
            assert_eq!(
                result.stderr.iter().filter(|byte| **byte == b'\n').count(),
                1
            );
        }
        assert_eq!(std::fs::read(output).unwrap(), b"old");
    }
}

#[test]
fn real_commit_failure_after_claim_beats_late_expiry() {
    let mut root = tempfile::tempdir().unwrap();
    let destination = root.path().join("result");
    let saved = root.path().join("saved");
    let ready = root.path().join("ready");
    std::fs::write(&destination, b"old").unwrap();
    let child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "run::tests::publication_phase_child",
            "--nocapture",
        ])
        .env("HTMLCUT_PUBLICATION_PHASE", "commit")
        .env("HTMLCUT_PUBLICATION_DESTINATION", &destination)
        .env("HTMLCUT_PUBLICATION_READY", &ready)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let child = TestChild {
        child: Some(child),
        fixture: Some(&mut root),
    };
    let started = Instant::now();
    while !ready.exists() && started.elapsed() < Duration::from_secs(2) {
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(ready.exists(), "real commit claim was not reached");
    std::fs::rename(&destination, &saved).unwrap();
    std::fs::create_dir(&destination).unwrap();
    let result = wait_for_test_child(
        child,
        "claimed commit failure",
        started + Duration::from_secs(3),
    );
    assert_eq!(result.status.code(), Some(5), "{result:?}");
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&result.stderr).unwrap()["error"],
        "output_unwritable"
    );
    assert_eq!(std::fs::read(saved).unwrap(), b"old");
    assert!(destination.is_dir());
}

#[path = "test_pipe.rs"]
mod test_pipe;
#[test]
fn simultaneous_blocked_reporting_and_cleanup_share_one_grace() {
    let mut root = tempfile::tempdir().unwrap();
    let output = root.path().join("result");
    let ready = root.path().join("ready");
    let cleanup_wait = root.path().join("cleanup-wait-nanos");
    std::fs::write(&output, b"old").unwrap();
    let (reader, mut writer) = std::io::pipe().unwrap();
    test_pipe::fill_pipe(&mut writer);
    let child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "run::tests::unwind_cleanup_remains_supervised_and_early_disarm_is_detected",
            "--nocapture",
        ])
        .env("HTMLCUT_CLI_UNWIND_CONTROL", "held")
        .env("HTMLCUT_PUBLICATION_DESTINATION", &output)
        .env("HTMLCUT_PUBLICATION_PANIC_AT", "restricted")
        .env("HTMLCUT_PUBLICATION_PHASE", "cleanup")
        .env("HTMLCUT_PUBLICATION_READY", &ready)
        .env("HTMLCUT_CLI_CLEANUP_WAIT_NANOS", &cleanup_wait)
        .stdout(std::process::Stdio::null())
        .stderr(writer.try_clone().unwrap())
        .spawn()
        .unwrap();
    let child = TestChild {
        child: Some(child),
        fixture: Some(&mut root),
    };
    let started = Instant::now();
    while !ready.exists() && started.elapsed() < Duration::from_secs(2) {
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(ready.exists(), "native unwind cleanup was not held");
    let result = wait_for_test_child(
        child,
        "shared reporting/cleanup grace",
        started + Duration::from_secs(2),
    );
    drop(writer);
    drop(reader);
    assert_eq!(result.status.code(), Some(4));
    let nanos: u128 = std::fs::read_to_string(cleanup_wait)
        .unwrap()
        .parse()
        .unwrap();
    assert_eq!(
        nanos, 0,
        "blocked reporting consumed the entire central grace; cleanup must not receive a second wait budget"
    );
}

#[test]
fn completion_fallback_leaves_the_reporting_owner_its_grace() {
    const READY: &str = "HTMLCUT_CLI_FALLBACK_READY";
    if let Some(ready) = std::env::var_os(READY) {
        std::fs::write(ready, b"reporting owner pending").unwrap();
        let _ = completion::<()>(Completion::Supervised(Error::limit("time", 1.0)));
        unreachable!("the process fallback must terminate");
    }
    let mut root = tempfile::tempdir().unwrap();
    let ready = root.path().join("ready");
    let child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "run::tests::completion_fallback_leaves_the_reporting_owner_its_grace",
            "--nocapture",
        ])
        .env(READY, &ready)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let child = TestChild {
        child: Some(child),
        fixture: Some(&mut root),
    };
    let deadline = Instant::now() + Duration::from_secs(2);
    while !ready.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(ready.exists(), "fallback was not entered");
    let observed = Instant::now();
    let output = wait_for_test_child(child, "completion fallback", deadline);
    assert_eq!(output.status.code(), Some(4), "{output:?}");
    assert!(output.stderr.is_empty());
    assert!(
        observed.elapsed() >= REPORT_GRACE,
        "fallback overtook the reporting owner's central grace"
    );
}
