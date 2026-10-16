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
    let deadline = job.control().deadline().unwrap();
    supervise(job.control()).unwrap();
    let result = completion(job.present(
        b"complete-private-result",
        Destination::File(destination.into()),
    ));
    if let Err(error) = result {
        if let Some(path) = std::env::var_os("HTMLCUT_PUBLICATION_ERROR_OBSERVATION") {
            let observation = serde_json::json!({
                "diagnostic": String::from_utf8(error.to_json_line()).unwrap(),
                "deadline_elapsed": Instant::now() >= deadline,
            });
            std::fs::write(path, serde_json::to_vec(&observation).unwrap()).unwrap();
        }
        report(&error);
        std::process::exit(exit_status(error.error));
    }
}
#[test]
fn publication_deadlines_preserve_old_output_and_honor_commit_claims() {
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
        let canonical_root = root.path().canonicalize().unwrap();
        let destination = canonical_root.join("result.json");
        let ready = canonical_root.join("ready");
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
            assert_eq!(
                std::fs::read_dir(root.path()).unwrap().count(),
                2,
                "{phase}: successful commit stranded staging at {stage:?}"
            );
        } else {
            assert_eq!(result.status.code(), Some(4), "{result:?}");
            assert_eq!(std::fs::read(&destination).unwrap(), b"old");
            // Hard exit does not promise completion of the separate cleanup
            // worker. Account only for this invocation's known staging names.
            let relative = stage.strip_prefix(&canonical_root).unwrap_or_else(|error| {
                panic!("{phase}: candidate {stage:?} escaped {canonical_root:?}: {error}")
            });
            let parts: Vec<_> = relative.components().collect();
            assert_eq!(
                parts.len(),
                if cfg!(windows) { 1 } else { 2 },
                "{phase}: candidate is not directly staged under {canonical_root:?}: {stage:?}"
            );
            assert!(
                parts
                    .iter()
                    .all(|part| matches!(part, std::path::Component::Normal(_))),
                "{phase}: candidate contains nonordinary components: {stage:?}"
            );
            let residue = parts[0].as_os_str();
            let aliases_reserved_name = |name: &str| {
                if cfg!(windows) {
                    residue
                        .as_encoded_bytes()
                        .eq_ignore_ascii_case(name.as_bytes())
                } else {
                    residue == std::ffi::OsStr::new(name)
                }
            };
            assert!(
                !aliases_reserved_name("result.json") && !aliases_reserved_name("ready"),
                "{phase}: candidate aliases destination or checkpoint: {stage:?}"
            );
            let mut required = std::collections::BTreeSet::from([
                std::ffi::OsString::from("result.json"),
                std::ffi::OsString::from("ready"),
            ]);
            for entry in std::fs::read_dir(&canonical_root).unwrap() {
                let entry = entry.unwrap();
                if required.remove(&entry.file_name()) {
                    let kind = entry.file_type().unwrap();
                    assert!(
                        kind.is_file() && !kind.is_symlink(),
                        "{phase}: base entry has unexpected type: {:?}",
                        entry.path()
                    );
                    continue;
                }
                assert_eq!(
                    entry.file_name().as_os_str(),
                    residue,
                    "{phase}: unexpected residue {:?}; known candidate {stage:?}",
                    entry.path()
                );
                let kind = match entry.file_type() {
                    Ok(kind) => kind,
                    Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
                    Err(error) => panic!(
                        "{phase}: cannot inspect known residue {:?}: {error}",
                        entry.path()
                    ),
                };
                #[cfg(windows)]
                assert!(
                    kind.is_file() && !kind.is_symlink(),
                    "{phase}: candidate has unexpected type: {stage:?}"
                );
                #[cfg(unix)]
                {
                    assert!(
                        kind.is_dir() && !kind.is_symlink(),
                        "{phase}: candidate container has unexpected type: {:?}",
                        entry.path()
                    );
                    let contents = match std::fs::read_dir(entry.path()) {
                        Ok(contents) => contents.collect::<Result<Vec<_>, _>>().unwrap(),
                        // A kernel cleanup already in flight may finish after exit.
                        Err(error) if error.kind() == io::ErrorKind::NotFound => Vec::new(),
                        Err(error) => panic!(
                            "{phase}: cannot inspect candidate container {:?}: {error}",
                            entry.path()
                        ),
                    };
                    assert!(
                        contents.len() <= 1,
                        "{phase}: unexpected entries under candidate {stage:?}"
                    );
                    for entry in contents {
                        assert_eq!(
                            entry.file_name().as_os_str(),
                            parts[1].as_os_str(),
                            "{phase}: unexpected nested residue {:?}; known candidate {stage:?}",
                            entry.path()
                        );
                        let kind = match entry.file_type() {
                            Ok(kind) => kind,
                            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
                            Err(error) => {
                                panic!("{phase}: cannot inspect known candidate {stage:?}: {error}")
                            }
                        };
                        assert!(
                            kind.is_file() && !kind.is_symlink(),
                            "{phase}: candidate has unexpected type: {stage:?}"
                        );
                    }
                }
            }
            assert!(
                required.is_empty(),
                "{phase}: base entries disappeared near candidate {stage:?}: {required:?}"
            );
        }
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
        if mode == "ordinary" {
            std::fs::write(
                std::env::var_os("HTMLCUT_PUBLICATION_ERROR_OBSERVATION").unwrap(),
                error.to_json_line(),
            )
            .unwrap();
        }
        report(&error);
        std::process::exit(exit_status(error.error));
    }
    for mode in ["held", "ordinary", "early-disarm"] {
        let mut root = tempfile::tempdir().unwrap();
        let output = root.path().join("result");
        let ready = root.path().join("ready");
        let observation = root.path().join("error-observation.json");
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
        if mode == "ordinary" {
            command.env("HTMLCUT_PUBLICATION_ERROR_OBSERVATION", &observation);
        }
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
            if mode == "ordinary" {
                let diagnostic = std::fs::read(observation).unwrap();
                let error: serde_json::Value = serde_json::from_slice(&diagnostic).unwrap();
                assert_eq!(error["error"], "internal");
                assert_eq!(diagnostic.iter().filter(|byte| **byte == b'\n').count(), 1);
                assert!(diagnostic.starts_with(&result.stderr), "{result:?}");
            } else {
                let stage = std::fs::read_to_string(ready).unwrap();
                let detail = serde_json::to_string(&format!(
                    "Staging candidate {stage}; extraordinary blocked filesystem calls can leave residue."
                ))
                .unwrap();
                let expected = format!(
                    "{{\"error\":\"limit_exceeded\",\"message\":\"The time limit of 1.0 was exceeded.\",\"limit\":\"time\",\"bound\":1.0,\"detail\":{detail}}}\n"
                );
                // The watchdog never blocks on a test-only observation write.
                // Its bounded report may emit only a prefix, including no bytes.
                assert!(
                    expected.as_bytes().starts_with(&result.stderr),
                    "{result:?}"
                );
            }
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
    let observation = root.path().join("error-observation.json");
    std::fs::write(&destination, b"old").unwrap();
    let child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "run::tests::publication_phase_child",
            "--nocapture",
        ])
        .env("HTMLCUT_PUBLICATION_PHASE", "commit")
        .env("HTMLCUT_PUBLICATION_COMMIT_HOLD_MS", "2500")
        .env("HTMLCUT_PUBLICATION_DESTINATION", &destination)
        .env("HTMLCUT_PUBLICATION_READY", &ready)
        .env("HTMLCUT_PUBLICATION_ERROR_OBSERVATION", &observation)
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
    assert!(ready.exists(), "real commit claim was not reached");
    std::fs::rename(&destination, &saved).unwrap();
    std::fs::create_dir(&destination).unwrap();
    let result = wait_for_test_child(
        child,
        "claimed commit failure",
        started + Duration::from_secs(6),
    );
    assert_eq!(result.status.code(), Some(5), "{result:?}");
    let observation: serde_json::Value =
        serde_json::from_slice(&std::fs::read(observation).unwrap()).unwrap();
    assert_eq!(observation["deadline_elapsed"], true);
    let diagnostic = observation["diagnostic"].as_str().unwrap().as_bytes();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(diagnostic).unwrap()["error"],
        "output_unwritable"
    );
    assert!(diagnostic.starts_with(&result.stderr), "{result:?}");
    assert_eq!(std::fs::read(saved).unwrap(), b"old");
    assert!(destination.is_dir());
    assert_eq!(std::fs::read_dir(destination).unwrap().count(), 0);
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 4);
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
