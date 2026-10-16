use super::*;

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn inherited_stdin_is_recognized_through_a_descriptor_directory_alias() {
    let fixture = tempfile::tempdir().unwrap();
    let alias = fixture.path().join("inherited");
    std::os::unix::fs::symlink("/dev/fd", &alias).unwrap();
    let context = htmlcut_core::ExecutionContext::new(None).unwrap();
    assert!(stdin_alias(&alias.join("0"), &context).unwrap());
    assert!(!stdin_alias(&alias.join("1"), &context).unwrap());

    let ordinary = fixture.path().join("input");
    std::fs::write(&ordinary, b"ordinary input").unwrap();
    assert!(!stdin_alias(&ordinary, &context).unwrap());
}

#[test]
fn interrupted_input_retries_without_losing_bytes_and_rechecks_cancellation() {
    struct InterruptedOnce {
        attempts: usize,
        cancel: Option<crate::JobControl>,
        bytes: io::Cursor<&'static [u8]>,
    }
    impl Read for InterruptedOnce {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            self.attempts += 1;
            if self.attempts == 1 {
                if let Some(control) = &self.cancel {
                    control.cancel();
                }
                return Err(io::ErrorKind::Interrupted.into());
            }
            self.bytes.read(buffer)
        }
    }
    for cancelled in [false, true] {
        let job = crate::Job::new(None).unwrap();
        let mut reader = InterruptedOnce {
            attempts: 0,
            cancel: cancelled.then(|| job.control()),
            bytes: io::Cursor::new(b"data"),
        };
        let result = read(&mut reader, 4, Path::new("input"), false, &job.context());
        if cancelled {
            assert_eq!(result.unwrap_err().error, ErrorCode::Cancelled);
            assert_eq!(reader.attempts, 1);
            assert_eq!(reader.bytes.position(), 0);
        } else {
            assert_eq!(result.unwrap(), b"data");
            assert_eq!(reader.attempts, 3);
        }
    }
}

#[test]
fn interrupted_output_retries_partial_writes_and_rechecks_cancellation() {
    struct InterruptedOnce {
        attempts: usize,
        cancel: Option<crate::JobControl>,
        bytes: Vec<u8>,
    }
    impl Write for InterruptedOnce {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.attempts += 1;
            if self.attempts == 1 {
                if let Some(control) = &self.cancel {
                    control.cancel();
                }
                return Err(io::ErrorKind::Interrupted.into());
            }
            let count = bytes.len().min(2);
            self.bytes.extend_from_slice(&bytes[..count]);
            Ok(count)
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    for cancelled in [false, true] {
        let job = crate::Job::new(None).unwrap();
        let mut writer = InterruptedOnce {
            attempts: 0,
            cancel: cancelled.then(|| job.control()),
            bytes: Vec::new(),
        };
        let result = write_all_timed(&mut writer, b"data", Some(&job));
        if cancelled {
            let error = result.unwrap_err();
            assert_eq!(
                error
                    .get_ref()
                    .unwrap()
                    .downcast_ref::<Error>()
                    .unwrap()
                    .error,
                ErrorCode::Cancelled
            );
            assert_eq!(writer.attempts, 1);
            assert!(writer.bytes.is_empty());
        } else {
            result.unwrap();
            assert_eq!(writer.bytes, b"data");
            assert_eq!(writer.attempts, 3);
        }
    }
}

fn exit_status(code: ErrorCode) -> i32 {
    match code {
        ErrorCode::LimitExceeded => 4,
        ErrorCode::InvalidRequest => 2,
        ErrorCode::Internal => 6,
        ErrorCode::InputUnreadable | ErrorCode::OutputUnwritable => 5,
        _ => panic!("unexpected native test diagnostic {code:?}"),
    }
}
fn report(error: &Error) {
    let _ = io::stderr().lock().write_all(&error.to_json_line());
}
const PENDING_CREATION_CHILD: &str = "HTMLCUT_PENDING_CREATION_CHILD";

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

pub(super) fn complete_test_creation_after_pending_snapshot(records: &[Arc<StageOwnership>]) {
    if std::env::var_os(PENDING_CREATION_CHILD).is_none() {
        return;
    }
    let owned = records.last().unwrap();
    if owned.created_identity.get().is_some() {
        return;
    }
    let _handoff = CreationHandoff(owned);
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(&owned.path)
        .unwrap();
    #[cfg(windows)]
    let file = crate::authorization::stage_options()
        .open(&owned.path)
        .unwrap();
    owned
        .created_identity
        .set(identity(&file).unwrap())
        .unwrap();
    #[cfg(windows)]
    {
        *owned.deletion_handle.lock().unwrap() = Some(file.try_clone().unwrap());
    }
    assert_eq!(file.metadata().unwrap().len(), 0);
    std::fs::write(owned.path.with_file_name("handoff-confirmed"), b"confirmed").unwrap();
}

#[test]
fn cleanup_returns_after_its_budget_when_creation_remains_unconfirmed() {
    const CHILD: &str = "HTMLCUT_UNCONFIRMED_CREATION_CHILD";
    if let Some(root) = std::env::var_os(CHILD) {
        let owned = Arc::new(StageOwnership {
            path: Path::new(&root).join("foreign"),
            created_identity: std::sync::OnceLock::new(),
            creation_finished: std::sync::atomic::AtomicBool::new(false),
            result_stage: true,
            #[cfg(any(target_os = "macos", target_os = "linux"))]
            directory: false,
            #[cfg(windows)]
            deletion_handle: std::sync::Mutex::new(None),
        });
        cleanup_owned_stages(&[owned], Instant::now(), Duration::ZERO);
        std::process::exit(0);
    }

    let mut root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("foreign"), b"foreign bytes").unwrap();
    let child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "local_io::tests::cleanup_returns_after_its_budget_when_creation_remains_unconfirmed",
            "--nocapture",
        ])
        .env(CHILD, root.path())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let child = TestChild {
        child: Some(child),
        fixture: Some(&mut root),
    };
    let output = wait_for_test_child(
        child,
        "unconfirmed creation cleanup budget",
        Instant::now() + Duration::from_secs(2),
    );
    assert!(output.status.success(), "{output:?}");
    assert!(output.stderr.is_empty());
    assert_eq!(
        std::fs::read(root.path().join("foreign")).unwrap(),
        b"foreign bytes"
    );
}

#[test]
fn cleanup_retries_when_reserved_creation_finishes_during_grace() {
    if let Some(root) = std::env::var_os(PENDING_CREATION_CHILD) {
        let owned = Arc::new(StageOwnership {
            path: Path::new(&root).join("pending-stage"),
            created_identity: std::sync::OnceLock::new(),
            creation_finished: std::sync::atomic::AtomicBool::new(false),
            result_stage: true,
            #[cfg(any(target_os = "macos", target_os = "linux"))]
            directory: false,
            #[cfg(windows)]
            deletion_handle: std::sync::Mutex::new(None),
        });
        // The first cleanup pass sees an unconfirmed reservation. The
        // test-only handoff completes after its pending snapshot, forcing
        // a second real cleanup pass within the supplied cleanup budget.
        let records = vec![owned];
        // Exercise retry logic synchronously with a generous test deadline,
        // independent of the hard-exit worker's 50ms scheduling window.
        cleanup_owned_stages(&records, Instant::now(), Duration::from_secs(2));
        assert!(
            !records[0].path.exists(),
            "the retry did not clean the completed handoff"
        );
        std::process::exit(0);
    }
    let mut root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("destination"), b"old").unwrap();
    std::fs::write(root.path().join("foreign"), b"foreign").unwrap();
    let child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "local_io::tests::cleanup_retries_when_reserved_creation_finishes_during_grace",
            "--nocapture",
        ])
        .env(PENDING_CREATION_CHILD, root.path())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let child = TestChild {
        child: Some(child),
        fixture: Some(&mut root),
    };
    let output = wait_for_test_child(
        child,
        "reserved creation expiry",
        Instant::now() + Duration::from_secs(3),
    );
    assert!(output.status.success(), "{output:?}");
    assert!(output.stderr.is_empty());
    assert_eq!(
        std::fs::read(root.path().join("handoff-confirmed")).unwrap(),
        b"confirmed"
    );
    assert_eq!(
        std::fs::read(root.path().join("destination")).unwrap(),
        b"old"
    );
    assert_eq!(
        std::fs::read(root.path().join("foreign")).unwrap(),
        b"foreign"
    );
    assert!(
        !root.path().join("pending-stage").exists(),
        "confirmed empty stage survived expiry"
    );
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 3);
}

#[test]
fn competing_stdin_arguments_keep_priority_over_a_zero_deadline() {
    const CHILD: &str = "HTMLCUT_ZERO_DEADLINE_ALIAS_CHILD";
    if let Ok(source) = std::env::var(CHILD) {
        let source = if source == "current-process-stdin" {
            format!("/proc/{}/fd/0", std::process::id())
        } else {
            source
        };
        let request = crate::Request {
            operation: crate::Operation::Extract {
                contract: crate::ContractInput::Stdin,
                format: crate::ExportFormat::Json,
            },
            input: if source == "-" {
                crate::Input::Stdin {
                    allow_terminal: true,
                }
            } else {
                crate::Input::File(source.into())
            },
            encoding: None,
            base_url: None,
            limits: crate::Limits::default(),
            destination: crate::Destination::Return,
        };
        let error = request.validate_syntax().unwrap_err();
        report(&error);
        std::process::exit(exit_status(error.error));
    }
    let sources = vec!["-"];
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    let sources = {
        let mut sources = sources;
        sources.extend(["/dev/stdin", "/dev/fd/0"]);
        #[cfg(target_os = "linux")]
        sources.extend([
            "/proc/self/fd/0",
            "/proc/thread-self/fd/0",
            "current-process-stdin",
        ]);
        sources
    };
    for source in sources {
        let child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "local_io::tests::competing_stdin_arguments_keep_priority_over_a_zero_deadline",
                "--nocapture",
            ])
            .env(CHILD, source)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let child = TestChild {
            child: Some(child),
            fixture: None,
        };
        let output = wait_for_test_child(
            child,
            "zero-deadline stdin argument refusal",
            Instant::now() + Duration::from_secs(3),
        );
        assert_eq!(output.status.code(), Some(2), "{source}: {output:?}");
        let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(error["error"], "invalid_request");
    }
}

#[test]
fn publication_revalidation_child() {
    let Some(destination) = std::env::var_os("HTMLCUT_REVALIDATION_DESTINATION") else {
        return;
    };
    if let Err(error) = publish(b"complete-private-result", Some(Path::new(&destination))) {
        report(&error);
        std::process::exit(exit_status(error.error));
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn existing_unwritable_output_reports_the_native_admission_failure() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join("existing");
    std::fs::write(&target, b"preserve").unwrap();
    std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o444)).unwrap();
    let native = OpenOptions::new().write(true).open(&target).unwrap_err();
    assert_eq!(native.kind(), io::ErrorKind::PermissionDenied);
    let error = publish(b"replacement", Some(&target)).unwrap_err();
    assert_eq!(error.error, ErrorCode::OutputUnwritable);
    assert_eq!(error.detail.as_deref(), Some(native.to_string().as_str()));
    assert_eq!(std::fs::read(&target).unwrap(), b"preserve");
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn changed_targets_and_aliases_refuse_before_commit_without_erasing_foreign_work() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    for change in [
        "identity",
        "policy",
        "appeared-readonly",
        "alias-name",
        "alias-parent",
    ] {
        let mut root = tempfile::tempdir().unwrap();
        let parent = root.path().join("parent");
        std::fs::create_dir(&parent).unwrap();
        let target = parent.join("target");
        if change != "appeared-readonly" {
            std::fs::write(&target, b"old").unwrap();
            std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o644)).unwrap();
        }
        let alternate = if change == "alias-parent" {
            let alternate_parent = root.path().join("alternate-parent");
            std::fs::create_dir(&alternate_parent).unwrap();
            alternate_parent.join("target")
        } else {
            parent.join("alternate")
        };
        let alias = root.path().join("output-alias");
        let destination = match change {
            "alias-name" => {
                std::fs::write(&alternate, b"foreign").unwrap();
                symlink(&target, &alias).unwrap();
                alias.clone()
            }
            "alias-parent" => {
                std::fs::write(&alternate, b"foreign").unwrap();
                symlink(&parent, &alias).unwrap();
                alias.join("target")
            }
            _ => target.clone(),
        };
        let ready = root.path().join("ready");
        let release = root.path().join("release");
        let child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "local_io::tests::publication_revalidation_child",
                "--nocapture",
            ])
            .env("HTMLCUT_REVALIDATION_DESTINATION", &destination)
            .env("HTMLCUT_PUBLICATION_PHASE", "policy")
            .env("HTMLCUT_PUBLICATION_READY", &ready)
            .env("HTMLCUT_PUBLICATION_RELEASE", &release)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let mut child = TestChild {
            child: Some(child),
            fixture: Some(&mut root),
        };
        let deadline = Instant::now() + Duration::from_secs(3);
        while !ready.exists() && matches!(child.try_wait(), Ok(None)) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(1));
        }
        if !ready.exists() {
            panic!("{change} did not reach completed policy");
        }
        let stage = std::path::PathBuf::from(std::fs::read_to_string(&ready).unwrap());
        assert_eq!(std::fs::read(&stage).unwrap(), b"complete-private-result");
        match change {
            "identity" => {
                std::fs::rename(&target, &alternate).unwrap();
                std::fs::write(&target, b"foreign").unwrap();
                std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o644)).unwrap();
            }
            "policy" => {
                std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o600)).unwrap();
                assert!(OpenOptions::new().write(true).open(&target).is_ok());
            }
            "appeared-readonly" => {
                std::fs::write(&target, b"foreign").unwrap();
                std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o444)).unwrap();
                assert_eq!(
                    OpenOptions::new()
                        .write(true)
                        .open(&target)
                        .unwrap_err()
                        .kind(),
                    io::ErrorKind::PermissionDenied
                );
            }
            "alias-name" => {
                std::fs::remove_file(&alias).unwrap();
                symlink(&alternate, &alias).unwrap();
            }
            "alias-parent" => {
                std::fs::remove_file(&alias).unwrap();
                symlink(alternate.parent().unwrap(), &alias).unwrap();
            }
            _ => unreachable!(),
        }
        std::fs::write(&release, b"continue").unwrap();
        let output = wait_for_test_child(child, "publication revalidation", deadline);
        assert_eq!(output.status.code(), Some(5), "{change}: {output:?}");
        let diagnostic: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(diagnostic["error"], "output_unwritable");
        assert_eq!(
            std::fs::read(&target).unwrap(),
            if matches!(change, "identity" | "appeared-readonly") {
                b"foreign".as_slice()
            } else {
                b"old".as_slice()
            }
        );
        if change.starts_with("alias-") {
            assert_eq!(std::fs::read(&alternate).unwrap(), b"foreign");
        } else if change == "identity" {
            assert_eq!(std::fs::read(&alternate).unwrap(), b"old");
        }
        if change == "policy" {
            assert_eq!(
                target.metadata().unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        assert!(!stage.exists(), "{change} stranded its owned result stage");
        assert!(
            !stage.parent().unwrap().exists(),
            "{change} stranded its owned private directory"
        );
    }
}

#[test]
fn publication_phase_child() {
    let Some(destination) = std::env::var_os("HTMLCUT_PUBLICATION_DESTINATION") else {
        return;
    };
    let job = crate::Job::new(None).unwrap();
    let result = job.perform(|| {
        publish_timed(
            b"complete-private-result",
            Some(Path::new(&destination)),
            Some(&job),
        )
    });
    match result {
        crate::Completion::Finished(Ok(_)) => (),
        crate::Completion::Finished(Err(error)) | crate::Completion::Supervised(error) => {
            report(&error);
            std::process::exit(exit_status(error.error));
        }
    }
}
#[cfg(windows)]
#[test]
fn reader_retained_from_actual_policy_probe_never_observes_result_bytes() {
    let mut root = tempfile::tempdir().unwrap();
    let destination = root.path().join("result.json");
    let ready = root.path().join("ready");
    let release = root.path().join("release");
    std::fs::write(&destination, b"old").unwrap();
    let child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "local_io::tests::publication_phase_child",
            "--nocapture",
        ])
        .env("HTMLCUT_PUBLICATION_PHASE", "policy-probe")
        .env("HTMLCUT_PUBLICATION_DESTINATION", &destination)
        .env("HTMLCUT_PUBLICATION_READY", &ready)
        .env("HTMLCUT_PUBLICATION_RELEASE", &release)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let child = TestChild {
        child: Some(child),
        fixture: Some(&mut root),
    };
    let started = Instant::now();
    while !ready.exists() && started.elapsed() < Duration::from_secs(3) {
        std::thread::sleep(Duration::from_millis(1));
    }
    if !ready.exists() {
        panic!("production policy probe was not reached");
    }
    let probe = std::path::PathBuf::from(std::fs::read_to_string(&ready).unwrap());
    let mut reader = File::open(&probe).unwrap();
    let probe_id = identity(&reader).unwrap();
    assert_eq!(reader.metadata().unwrap().len(), 0);
    std::fs::write(&release, b"continue").unwrap();
    let result = wait_for_test_child(
        child,
        "released policy probe",
        started + Duration::from_secs(3),
    );
    assert!(
        result.status.success() && result.stderr.is_empty(),
        "{result:?}"
    );
    assert_eq!(
        std::fs::read(&destination).unwrap(),
        b"complete-private-result"
    );
    assert_ne!(
        identity(&File::open(&destination).unwrap()).unwrap(),
        probe_id
    );
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes).unwrap();
    assert!(
        bytes.is_empty(),
        "actual empty policy probe was reused for result bytes"
    );
    assert_eq!(reader.metadata().unwrap().len(), 0);
    assert!(!probe.exists(), "actual owned policy probe was not removed");
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 3);
}
#[test]
fn policy_probe_cleanup_does_not_claim_a_result_checkpoint() {
    const CHILD: &str = "HTMLCUT_POLICY_PROBE_CLEANUP_CHILD";
    if let Some(root) = std::env::var_os(CHILD) {
        let probe = create_stage(Path::new(&root), None, StageKind::InheritedPolicyProbe).unwrap();
        drop(probe);
        return;
    }
    let mut root = tempfile::tempdir().unwrap();
    let ready = root.path().join("ready");
    let child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "local_io::tests::policy_probe_cleanup_does_not_claim_a_result_checkpoint",
            "--nocapture",
        ])
        .env(CHILD, root.path())
        .env("HTMLCUT_PUBLICATION_PHASE", "cleanup")
        .env("HTMLCUT_PUBLICATION_READY", &ready)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let child = TestChild {
        child: Some(child),
        fixture: Some(&mut root),
    };
    let output = wait_for_test_child(
        child,
        "empty policy-probe cleanup",
        Instant::now() + Duration::from_secs(2),
    );
    assert!(output.status.success(), "{output:?}");
    assert!(output.stderr.is_empty());
    assert!(
        !ready.exists(),
        "a policy probe claimed the result cleanup checkpoint"
    );
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

#[test]
fn cleanup_never_unlinks_unconfirmed_or_replaced_objects() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("collision");
    std::fs::write(&path, b"foreign").unwrap();
    let unconfirmed = StageOwnership {
        path: path.clone(),
        created_identity: std::sync::OnceLock::new(),
        creation_finished: std::sync::atomic::AtomicBool::new(false),
        result_stage: true,
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        directory: false,
        #[cfg(windows)]
        deletion_handle: std::sync::Mutex::new(None),
    };
    unconfirmed.cleanup();
    assert_eq!(std::fs::read(&path).unwrap(), b"foreign");
    let stage = create_stage(root.path(), None, StageKind::InheritedPolicyProbe).unwrap();
    let displaced = root.path().join("displaced");
    std::fs::rename(stage.path(), &displaced).unwrap();
    std::fs::write(stage.path(), b"foreign replacement").unwrap();
    let name = stage.path().to_path_buf();
    drop(stage);
    assert_eq!(std::fs::read(name).unwrap(), b"foreign replacement");
    std::fs::remove_file(displaced).unwrap();
}
#[test]
fn output_under_a_regular_file_refuses_and_preserves_the_parent() {
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("file");
    std::fs::write(&file, b"unchanged").unwrap();
    let child = file.join("child");
    let expected = std::fs::symlink_metadata(&child).unwrap_err();
    if expected.kind() == io::ErrorKind::NotFound {
        // Windows reports PATH_NOT_FOUND here. Resolution does not promise
        // that a candidate's parent is a directory; publication checks that.
        assert_eq!(
            resolve_output_links(&child, &crate::Job::new(None).unwrap().context()).unwrap(),
            child
        );
    } else {
        let error =
            resolve_output_links(&child, &crate::Job::new(None).unwrap().context()).unwrap_err();
        assert_eq!(error.kind(), expected.kind());
        assert_eq!(error.raw_os_error(), expected.raw_os_error());
    }
    let error = publish(b"new", Some(&child)).unwrap_err();
    assert_eq!(error.error, ErrorCode::OutputUnwritable);
    assert_eq!(
        error.file.as_deref(),
        Some(child.to_string_lossy().as_ref())
    );
    assert!(error.detail.is_some());
    assert_eq!(std::fs::read(file).unwrap(), b"unchanged");
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
}
#[test]
fn output_ignores_only_broken_pipe_from_writing_or_flushing() {
    struct FailingWriter {
        kind: io::ErrorKind,
        on_flush: bool,
    }
    impl Write for FailingWriter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if self.on_flush {
                Ok(bytes.len())
            } else {
                Err(io::Error::new(self.kind, "destination failure"))
            }
        }
        fn flush(&mut self) -> io::Result<()> {
            if self.on_flush {
                Err(io::Error::new(self.kind, "destination failure"))
            } else {
                Ok(())
            }
        }
    }
    for on_flush in [false, true] {
        let path = Path::new("output.json");
        let mut writer = FailingWriter {
            kind: io::ErrorKind::BrokenPipe,
            on_flush,
        };
        assert_eq!(
            write_output(&mut writer, b"complete", path, None).unwrap(),
            crate::Delivery::Disconnected
        );
        writer.kind = io::ErrorKind::PermissionDenied;
        let error = write_output(&mut writer, b"complete", path, None).unwrap_err();
        assert_eq!(error.error, ErrorCode::OutputUnwritable);
        assert_eq!(error.file.as_deref(), Some("output.json"));
        assert_eq!(error.detail.as_deref(), Some("destination failure"));
    }
}

#[test]
fn ordinary_output_names_are_not_descriptor_aliases() {
    let directory = tempfile::tempdir().unwrap();
    for name in ["stdout", "stdin", "stderr", "1", "123"] {
        let path = directory.path().join(name);
        std::fs::write(&path, b"ordinary file").unwrap();
        assert!(!descriptor_alias(&path), "{path:?}");
    }
    let ordinary_fd_directory = directory.path().join("fd");
    std::fs::create_dir(&ordinary_fd_directory).unwrap();
    assert!(!descriptor_alias(&ordinary_fd_directory.join("1")));
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn descriptor_names_require_the_stream_namespace_and_numeric_fd_names() {
    for path in ["/dev/stdin", "/dev/stdout", "/dev/stderr", "/dev/fd/1"] {
        assert!(descriptor_alias(Path::new(path)), "{path}");
    }
    for path in ["/dev/null", "/dev/1", "/dev/fd/not-a-descriptor"] {
        assert!(!descriptor_alias(Path::new(path)), "{path}");
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn output_link_resolution_checks_the_target_after_forty_links() {
    use std::os::unix::fs::symlink;
    let directory = tempfile::tempdir().unwrap();
    let target = directory.path().join("target");
    let mut link = target.clone();
    for index in 0..40 {
        let next = directory.path().join(format!("link-{index}"));
        symlink(link.file_name().unwrap(), &next).unwrap();
        link = next;
    }
    assert_eq!(
        resolve_output_links(&link, &crate::Job::new(None).unwrap().context()).unwrap(),
        target
    );
    std::fs::write(&target, b"existing").unwrap();
    assert_eq!(
        resolve_output_links(&link, &crate::Job::new(None).unwrap().context()).unwrap(),
        target
    );
    assert_eq!(std::fs::read(target).unwrap(), b"existing");
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn output_link_cycles_with_growing_relative_paths_are_refused() {
    use std::os::unix::fs::symlink;
    let directory = tempfile::tempdir().unwrap();
    std::fs::create_dir(directory.path().join("child")).unwrap();
    let link = directory.path().join("cycle");
    symlink("child/../cycle", &link).unwrap();
    let error = resolve_output_links(&link, &crate::Job::new(None).unwrap().context()).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    assert_eq!(error.to_string(), "symbolic link cycle");
}

#[test]
fn publication_replaces_a_complete_file_and_cleans_failed_staging() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("result.json");
    std::fs::write(&path, b"old").unwrap();
    publish(b"\"complete\"\n", Some(&path)).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), b"\"complete\"\n");
    let target_directory = directory.path().join("destination");
    std::fs::create_dir(&target_directory).unwrap();
    let error = publish(b"new", Some(&target_directory)).unwrap_err();
    assert_eq!(error.error, ErrorCode::OutputUnwritable);
    assert!(error.detail.is_some());
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 2);
}
#[test]
fn cooperative_checks_stop_short_reads_and_writes_before_the_next_call() {
    struct Reader {
        control: crate::JobControl,
        calls: usize,
    }
    impl Read for Reader {
        fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
            self.calls += 1;
            bytes[0] = b'x';
            self.control.cancel();
            Ok(1)
        }
    }
    let job = crate::Job::new(None).unwrap();
    let mut reader = Reader {
        control: job.control(),
        calls: 0,
    };
    let error = read(&mut reader, 8, Path::new("input"), false, &job.context()).unwrap_err();
    assert_eq!(error.error, ErrorCode::Cancelled);
    assert_eq!(reader.calls, 1);
    struct Writer {
        control: crate::JobControl,
        calls: usize,
    }
    impl Write for Writer {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            self.calls += 1;
            self.control.cancel();
            Ok(1)
        }
        fn flush(&mut self) -> io::Result<()> {
            panic!("cancelled write must not flush")
        }
    }
    let job = crate::Job::new(None).unwrap();
    let mut writer = Writer {
        control: job.control(),
        calls: 0,
    };
    let error =
        write_output(&mut writer, b"complete", Path::new("output"), Some(&job)).unwrap_err();
    assert_eq!(error.error, ErrorCode::Cancelled);
    assert_eq!(writer.calls, 1);
    assert!(error.limit.is_none());
    assert!(error.bound.is_none());
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
#[test]
fn link_resolution_preserves_stop_errors_between_links_without_a_supervisor() {
    use std::{
        os::unix::fs::symlink,
        sync::atomic::{AtomicBool, AtomicUsize, Ordering},
    };
    for deadline in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let last = root.path().join("last");
        std::fs::write(&last, b"value").unwrap();
        let second = root.path().join("second");
        let first = root.path().join("first");
        symlink(&last, &second).unwrap();
        symlink(&second, &first).unwrap();
        let stopped = Arc::new(AtomicBool::new(false));
        let read_stop = stopped.clone();
        let lookups = Arc::new(AtomicUsize::new(0));
        let observed = lookups.clone();
        let context = htmlcut_core::ExecutionContext::with_checkpoint(move || {
            if read_stop.load(Ordering::Relaxed) {
                if deadline {
                    Err(Error::limit("time", 0.0))
                } else {
                    Err(Error::new(
                        ErrorCode::Cancelled,
                        "controlled cancellation between links",
                    ))
                }
            } else {
                Ok(())
            }
        });
        let error = with_link_lookup_probe(
            move |_| {
                if observed.fetch_add(1, Ordering::Relaxed) == 1 {
                    stopped.store(true, Ordering::Relaxed);
                }
                Ok(())
            },
            || stdin_alias(&first, &context),
        )
        .unwrap_err();
        assert_eq!(lookups.load(Ordering::Relaxed), 2);
        assert_eq!(
            error.error,
            if deadline {
                ErrorCode::LimitExceeded
            } else {
                ErrorCode::Cancelled
            }
        );
        if !deadline {
            assert!(error.limit.is_none());
            assert!(error.bound.is_none());
        }
    }
}

#[test]
fn missing_regular_filenames_preserve_native_punctuation() {
    let names = ["plain", "name.", "name.."];
    #[cfg(unix)]
    let names = names.into_iter().chain(["name\\", "name\\."]);
    for name in names {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join(name);
        let native = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap();
        drop(native);
        std::fs::remove_file(&path).unwrap();
        publish(b"complete-filename", Some(&path)).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"complete-filename");
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
    }
}

#[cfg(windows)]
#[allow(unsafe_code)]
mod windows_namespace {
    use super::*;
    use std::os::windows::fs::{symlink_dir, symlink_file};
    use windows_sys::Win32::{
        Foundation::{ERROR_ACCESS_DENIED, LocalFree},
        Security::{Authorization::*, *},
    };

    struct LocalAllocation(*mut std::ffi::c_void);
    impl Drop for LocalAllocation {
        fn drop(&mut self) {
            // SAFETY: this fixture owns the allocation returned by the security API.
            unsafe { LocalFree(self.0) };
        }
    }
    fn native_status(status: u32) {
        assert_eq!(status, 0, "{}", io::Error::from_raw_os_error(status as i32));
    }
    fn wide_path(path: &Path) -> Vec<u16> {
        use std::os::windows::ffi::OsStrExt;
        path.as_os_str().encode_wide().chain(Some(0)).collect()
    }
    fn author_native_dacl(path: &Path, sddl: &str) {
        let text = sddl.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
        let mut descriptor = std::ptr::null_mut();
        // SAFETY: fixed terminated SDDL and a valid output pointer.
        assert_ne!(
            unsafe {
                ConvertStringSecurityDescriptorToSecurityDescriptorW(
                    text.as_ptr(),
                    1,
                    &mut descriptor,
                    std::ptr::null_mut(),
                )
            },
            0,
            "{}",
            io::Error::last_os_error()
        );
        let descriptor = LocalAllocation(descriptor);
        let mut acl = std::ptr::null_mut();
        let mut present = 0;
        let mut defaulted = 0;
        // SAFETY: descriptor and its borrowed DACL remain live across assignment.
        assert_ne!(
            unsafe {
                GetSecurityDescriptorDacl(descriptor.0, &mut present, &mut acl, &mut defaulted)
            },
            0
        );
        assert_ne!(present, 0);
        assert!(!acl.is_null());
        // SAFETY: the terminated owned pathname and descriptor's borrowed DACL
        // remain live across this DACL-only assignment.
        native_status(unsafe {
            SetNamedSecurityInfoW(
                wide_path(path).as_ptr(),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                acl,
                std::ptr::null_mut(),
            )
        });
    }
    fn deny_data_writes(path: &Path) {
        // Leave owner read, metadata and DELETE available; deny data/append only.
        author_native_dacl(path, "D:P(D;;0x00000006;;;WD)(A;;FA;;;OW)");
    }
    fn native_policy(path: &Path, information: u32) -> String {
        let mut descriptor = std::ptr::null_mut();
        // SAFETY: terminated owned path and initialized output; only the requested components are queried.
        native_status(unsafe {
            GetNamedSecurityInfoW(
                wide_path(path).as_ptr(),
                SE_FILE_OBJECT,
                information,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut descriptor,
            )
        });
        let descriptor = LocalAllocation(descriptor);
        let mut text = std::ptr::null_mut();
        // SAFETY: native descriptor and output locals are valid; returned text is LocalFree-owned.
        assert_ne!(
            unsafe {
                ConvertSecurityDescriptorToStringSecurityDescriptorW(
                    descriptor.0,
                    1,
                    information,
                    &mut text,
                    std::ptr::null_mut(),
                )
            },
            0
        );
        let _text = LocalAllocation(text.cast());
        // The API returns a NUL-terminated SDDL allocation. Its optional
        // size output is not the length of every requested component view.
        let mut units = 0;
        // SAFETY: conversion succeeded and the owned SDDL remains live until after decoding.
        while unsafe { *text.add(units) } != 0 {
            units += 1;
        }
        // SAFETY: only initialized UTF-16 units before the documented terminator are read.
        String::from_utf16(unsafe { std::slice::from_raw_parts(text, units) }).unwrap()
    }

    fn native_dacl(path: &Path) -> String {
        native_policy(path, DACL_SECURITY_INFORMATION)
    }

    #[test]
    fn writable_file_symlink_publication_follows_the_native_target() {
        use std::io::Read;
        let root = tempfile::tempdir().unwrap();
        let target = root.path().join("target");
        let alias = root.path().join("alias");
        std::fs::write(&target, b"old").unwrap();
        symlink_file(&target, &alias).expect("Windows namespace controls require native file-symlink creation; no machine policy is changed");
        assert!(
            std::fs::symlink_metadata(&alias)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        let mut old = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&alias)
            .unwrap();
        assert_eq!(
            identity(&old).unwrap(),
            identity(&File::open(&target).unwrap()).unwrap()
        );
        let link = std::fs::read_link(&alias).unwrap();
        publish(b"complete-linked-result", Some(&alias)).unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"complete-linked-result");
        assert_eq!(std::fs::read(&alias).unwrap(), b"complete-linked-result");
        assert_eq!(std::fs::read_link(&alias).unwrap(), link);
        let mut retained = Vec::new();
        old.read_to_end(&mut retained).unwrap();
        assert_eq!(retained, b"old");
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 2);
    }

    #[test]
    fn missing_native_directory_spellings_do_not_create_regular_files() {
        for suffix in ["/", "/.", "\\", "\\."] {
            for through_link in [false, true] {
                let root = tempfile::tempdir().unwrap();
                let target = root.path().join("missing-directory");
                let path = if through_link {
                    let alias = root.path().join("alias");
                    symlink_file(format!("missing-directory{suffix}"), &alias)
                        .expect("Windows suffix controls require native file-symlink creation");
                    alias
                } else {
                    std::path::PathBuf::from(format!("{}{suffix}", target.display()))
                };
                let link = through_link.then(|| std::fs::read_link(&path).unwrap());
                let result = publish(b"must-not-create", Some(&path));
                assert!(
                    result.is_err(),
                    "{suffix:?} link={through_link} admitted a regular file"
                );
                let error = result.unwrap_err();
                assert_eq!(error.error, ErrorCode::OutputUnwritable);
                assert!(!target.exists());
                if let Some(link) = link {
                    assert_eq!(std::fs::read_link(&path).unwrap(), link);
                }
                assert_eq!(
                    std::fs::read_dir(root.path()).unwrap().count(),
                    usize::from(through_link)
                );
                eprintln!("completed native backslash suffix {suffix:?} link={through_link}");
            }
        }
    }

    #[test]
    fn changed_output_namespace_or_authorization_refuses_publication() {
        for change in [
            "alias-parent",
            "alias-leaf",
            "appeared-dacl",
            "identity",
            "policy",
        ] {
            let mut root = tempfile::tempdir().unwrap();
            let parent = root.path().join("parent");
            let other_parent = root.path().join("other-parent");
            std::fs::create_dir(&parent).unwrap();
            std::fs::create_dir(&other_parent).unwrap();
            let target = parent.join("target");
            let alternate = if change == "alias-parent" {
                other_parent.join("target")
            } else {
                parent.join("alternate")
            };
            if change != "appeared-dacl" {
                std::fs::write(&target, b"old").unwrap();
            }
            std::fs::write(&alternate, b"foreign-alternate").unwrap();
            let alias = root.path().join("alias");
            let destination = match change {
                "alias-parent" => {
                    symlink_dir(&parent, &alias).expect(
                        "Windows namespace controls require native directory-symlink creation",
                    );
                    alias.join("target")
                }
                "alias-leaf" => {
                    symlink_file(&target, &alias)
                        .expect("Windows namespace controls require native file-symlink creation");
                    alias.clone()
                }
                _ => target.clone(),
            };
            if change != "appeared-dacl" {
                assert_eq!(
                    identity(&OpenOptions::new().write(true).open(&destination).unwrap()).unwrap(),
                    identity(&File::open(&target).unwrap()).unwrap()
                );
            }
            let ready = root.path().join("ready");
            let release = root.path().join("release");
            let child = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "local_io::tests::publication_revalidation_child",
                    "--nocapture",
                ])
                .env("HTMLCUT_REVALIDATION_DESTINATION", &destination)
                .env("HTMLCUT_PUBLICATION_PHASE", "policy")
                .env("HTMLCUT_PUBLICATION_READY", &ready)
                .env("HTMLCUT_PUBLICATION_RELEASE", &release)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .unwrap();
            let mut child = TestChild {
                child: Some(child),
                fixture: Some(&mut root),
            };
            let deadline = Instant::now() + Duration::from_secs(10);
            while !ready.exists() && Instant::now() < deadline {
                if child.try_wait().unwrap().is_some() {
                    let output = child.child.take().unwrap().wait_with_output().unwrap();
                    panic!("{change} exited before completed staging: {output:?}");
                }
                std::thread::sleep(Duration::from_millis(1));
            }
            assert!(ready.exists(), "{change} never reached completed staging");
            let stage = std::path::PathBuf::from(std::fs::read_to_string(&ready).unwrap());
            assert_eq!(std::fs::read(&stage).unwrap(), b"complete-private-result");
            let displaced = parent.join("displaced");
            let target_dacl = if change == "identity" {
                let information = OWNER_SECURITY_INFORMATION
                    | GROUP_SECURITY_INFORMATION
                    | DACL_SECURITY_INFORMATION
                    | LABEL_SECURITY_INFORMATION;
                let before = native_policy(&target, information);
                let old_id = identity(&File::open(&target).unwrap()).unwrap();
                std::fs::rename(&target, &displaced).unwrap();
                std::fs::write(&target, b"foreign-replacement").unwrap();
                assert_ne!(identity(&File::open(&target).unwrap()).unwrap(), old_id);
                assert_eq!(native_policy(&target, information), before);
                drop(OpenOptions::new().write(true).open(&target).unwrap());
                Some(native_dacl(&target))
            } else if change == "policy" {
                let old_id = identity(&File::open(&target).unwrap()).unwrap();
                let before = native_dacl(&target);
                let information = OWNER_SECURITY_INFORMATION
                    | GROUP_SECURITY_INFORMATION
                    | LABEL_SECURITY_INFORMATION;
                let retained = native_policy(&target, information);
                author_native_dacl(&target, "D:P(A;;FA;;;OW)");
                assert_ne!(native_dacl(&target), before);
                assert_eq!(native_policy(&target, information), retained);
                assert_eq!(identity(&File::open(&target).unwrap()).unwrap(), old_id);
                drop(OpenOptions::new().write(true).open(&target).unwrap());
                Some(native_dacl(&target))
            } else if change == "appeared-dacl" {
                std::fs::write(&target, b"foreign-appeared").unwrap();
                deny_data_writes(&target);
                assert!(!target.metadata().unwrap().permissions().readonly());
                let denied = OpenOptions::new().write(true).open(&target).unwrap_err();
                assert_eq!(denied.raw_os_error(), Some(ERROR_ACCESS_DENIED as i32));
                let peer = parent.join("replacement-control");
                let replaceable = parent.join("denied-control");
                std::fs::write(&peer, b"native-replaced").unwrap();
                std::fs::write(&replaceable, b"native-old").unwrap();
                deny_data_writes(&replaceable);
                let policy = native_dacl(&target);
                assert_eq!(native_dacl(&replaceable), policy);
                assert_eq!(
                    OpenOptions::new()
                        .write(true)
                        .open(&replaceable)
                        .unwrap_err()
                        .raw_os_error(),
                    Some(ERROR_ACCESS_DENIED as i32)
                );
                std::fs::rename(&peer, &replaceable).unwrap();
                assert_eq!(std::fs::read(&replaceable).unwrap(), b"native-replaced");
                Some(policy)
            } else {
                if change == "alias-parent" {
                    std::fs::remove_dir(&alias).unwrap();
                    symlink_dir(&other_parent, &alias).unwrap();
                    assert_eq!(
                        std::fs::canonicalize(&alias).unwrap(),
                        std::fs::canonicalize(&other_parent).unwrap()
                    );
                } else {
                    std::fs::remove_file(&alias).unwrap();
                    symlink_file(&alternate, &alias).unwrap();
                    assert_eq!(
                        identity(&File::open(&alias).unwrap()).unwrap(),
                        identity(&File::open(&alternate).unwrap()).unwrap()
                    );
                }
                None
            };
            std::fs::write(&release, b"continue").unwrap();
            let output = wait_for_test_child(child, "Windows publication revalidation", deadline);
            assert_eq!(output.status.code(), Some(5), "{change}: {output:?}");
            assert_eq!(
                std::fs::read(&target).unwrap(),
                if change == "appeared-dacl" {
                    b"foreign-appeared".as_slice()
                } else if change == "identity" {
                    b"foreign-replacement".as_slice()
                } else {
                    b"old".as_slice()
                }
            );
            assert_eq!(std::fs::read(&alternate).unwrap(), b"foreign-alternate");
            if change == "identity" {
                assert_eq!(std::fs::read(&displaced).unwrap(), b"old");
            }
            if let Some(policy) = target_dacl {
                assert_eq!(native_dacl(&target), policy);
            }
            assert!(!stage.exists(), "{change} left owned result staging");
            assert_eq!(
                parent.read_dir().unwrap().count(),
                match change {
                    "alias-parent" => 1,
                    "alias-leaf" | "policy" => 2,
                    _ => 3,
                },
                "{change} left an unexpected parent entry"
            );
            assert_eq!(
                other_parent.read_dir().unwrap().count(),
                usize::from(change == "alias-parent")
            );
            eprintln!("completed native publication revalidation {change}");
        }
    }
}
