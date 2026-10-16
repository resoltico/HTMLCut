#[cfg(test)]
mod tests {
    use super::*;

    const REPORT_GRACE: Duration = Duration::from_millis(50);
    fn exit_status(code: ErrorCode) -> i32 {
        match code { ErrorCode::LimitExceeded => 4, ErrorCode::InvalidRequest => 2, ErrorCode::Internal => 6, ErrorCode::InputUnreadable | ErrorCode::OutputUnwritable => 5, _ => panic!("unexpected native test diagnostic {code:?}") }
    }
    fn report(error: &Error) { let _ = io::stderr().lock().write_all(&error.to_json_line()); }
    fn guarded<T>(action: impl FnOnce() -> Result<T, Error>) -> Result<T, Error> {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(action)).unwrap_or_else(|_| Err(Error::new(ErrorCode::Internal, "Internal test interruption.")))
    }
    struct Watchdog(crate::Job);
    impl std::ops::Deref for Watchdog { type Target = crate::Job; fn deref(&self) -> &Self::Target { &self.0 } }
    impl Watchdog {
        fn new(duration: Duration) -> Self {
            let job = crate::Job::new(Some(duration)).unwrap();
            let control = job.control();
            std::thread::spawn(move || {
                std::thread::sleep(duration);
                if let crate::Supervision::Claimed(error) = control.supervise() {
                    control.cleanup(REPORT_GRACE); report(&error); std::process::exit(exit_status(error.error));
                }
            }); Self(job)
        }
        fn finish<T>(&self, result: Result<T, Error>) -> Result<T, Error> {
            match self.0.finish(result) { crate::Completion::Finished(result) => result, crate::Completion::Supervised(error) => { std::thread::sleep(REPORT_GRACE*3); std::process::exit(exit_status(error.error)); } }
        }
    }
fn expire(duration: Duration, staging: &Arc<std::sync::Mutex<Vec<Arc<StageOwnership>>>>) -> ! {
    let diagnostic = staging
        .try_lock()
        .ok()
        .and_then(|records| records.last().cloned());
    let cleanup = Arc::clone(staging);
    let started = Instant::now();
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let records = cleanup.lock().unwrap().clone();
        cleanup_owned_stages(&records, started, REPORT_GRACE);
        let _ = sender.send(());
    });
    let mut error = Error::limit("time", duration.as_secs_f64());
    if let Some(owned) = diagnostic {
        error.detail = Some(format!(
            "Staging candidate {}; extraordinary blocked filesystem calls can leave residue.",
            owned.path.display()
        ));
    }
    report(&error);
    let _ = receiver.recv_timeout(REPORT_GRACE.saturating_sub(started.elapsed()));
    std::process::exit(4);
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
                Ok(None) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(1))
                }
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
    fn expiry_retries_cleanup_when_reserved_creation_finishes_during_grace() {
        if let Some(root) = std::env::var_os(PENDING_CREATION_CHILD) {
            let owned = Arc::new(StageOwnership {
                path: Path::new(&root).join("pending-stage"),
                created_identity: std::sync::OnceLock::new(),
                creation_finished: std::sync::atomic::AtomicBool::new(false),
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
            // Real expiry still establishes the process exit and diagnostic.
            expire(
                Duration::from_millis(1),
                &Arc::new(std::sync::Mutex::new(records)),
            );
        }
        let mut root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("destination"), b"old").unwrap();
        std::fs::write(root.path().join("foreign"), b"foreign").unwrap();
        let child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "local_io::tests::expiry_retries_cleanup_when_reserved_creation_finishes_during_grace",
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
        assert_eq!(output.status.code(), Some(4), "{output:?}");
        let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(error["error"], "limit_exceeded");
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
            let request = crate::Request { operation: crate::Operation::Extract { contract: crate::ContractInput::Stdin, format: crate::ExportFormat::Json }, input: if source == "-" { crate::Input::Stdin { allow_terminal: true } } else { crate::Input::File(source.into()) }, encoding: None, base_url: None, limits: crate::Limits::default(), destination: crate::Destination::Return };
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
            while !ready.exists()
                && matches!(child.try_wait(), Ok(None))
                && Instant::now() < deadline
            {
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
                    std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o644))
                        .unwrap();
                }
                "policy" => {
                    std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o600))
                        .unwrap();
                    assert!(OpenOptions::new().write(true).open(&target).is_ok());
                }
                "appeared-readonly" => {
                    std::fs::write(&target, b"foreign").unwrap();
                    std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o444))
                        .unwrap();
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
        let duration = if std::env::var("HTMLCUT_PUBLICATION_PHASE")
            .is_ok_and(|phase| phase == "policy-probe")
        {
            Duration::from_secs(2)
        } else {
            Duration::from_millis(500)
        };
        let watchdog = Watchdog::new(duration);
        let result = publish_timed(
            b"complete-private-result",
            Some(Path::new(&destination)),
            Some(&watchdog),
        );
        let result = watchdog.finish(result);
        if let Err(error) = result {
            report(&error);
            std::process::exit(exit_status(error.error));
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
                    "local_io::tests::publication_phase_child",
                    "--nocapture",
                ])
                .env("HTMLCUT_PUBLICATION_PHASE", phase)
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
            while !ready.exists() && started.elapsed() < Duration::from_secs(3) {
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
                started + Duration::from_secs(3),
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
    fn cleanup_never_unlinks_unconfirmed_or_replaced_objects() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("collision");
        std::fs::write(&path, b"foreign").unwrap();
        let unconfirmed = StageOwnership {
            path: path.clone(),
            created_identity: std::sync::OnceLock::new(),
            creation_finished: std::sync::atomic::AtomicBool::new(false),
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
            assert_eq!(resolve_output_links(&child).unwrap(), child);
        } else {
            let error = resolve_output_links(&child).unwrap_err();
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
            assert_eq!(write_output(&mut writer, b"complete", path, None).unwrap(),crate::Delivery::Disconnected);
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

    #[test]
    fn panic_unwinding_finishes_cleanup_before_delayed_reporting() {
        const CHILD: &str = "HTMLCUT_WATCHDOG_PANIC_CHILD";
        if std::env::var_os(CHILD).is_some() {
            std::panic::set_hook(Box::new(|_| {}));
            let error = guarded(|| -> Result<(), Error> {
                let _watchdog = Watchdog::new(Duration::from_millis(250));
                panic!("extraction interrupted before completion");
            })
            .unwrap_err();
            // The supervisor observes completed cleanup while error reporting is delayed.
            std::thread::sleep(Duration::from_millis(750));
            io::stderr()
                .lock()
                .write_all(&error.to_json_line())
                .unwrap();
            std::process::exit(exit_status(error.error));
        }

        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "local_io::tests::panic_unwinding_finishes_cleanup_before_delayed_reporting",
                "--nocapture",
            ])
            .env(CHILD, "1")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if child.try_wait().unwrap().is_some() {
                break;
            }
            if Instant::now() >= deadline {
                child.kill().unwrap();
                let output = child.wait_with_output().unwrap();
                panic!("panic unwinding did not finish: {output:?}");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let output = child.wait_with_output().unwrap();
        assert_eq!(output.status.code(), Some(6), "{output:?}");
        assert_eq!(output.stderr.iter().filter(|c| **c == b'\n').count(), 1);
        let diagnostic: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(diagnostic["error"], "internal");
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
        assert_eq!(resolve_output_links(&link).unwrap(), target);
        std::fs::write(&target, b"existing").unwrap();
        assert_eq!(resolve_output_links(&link).unwrap(), target);
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
        let error = resolve_output_links(&link).unwrap_err();
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
        struct Reader {control:crate::JobControl,calls:usize}
        impl Read for Reader {fn read(&mut self,bytes:&mut [u8])->io::Result<usize>{self.calls+=1;bytes[0]=b'x';self.control.cancel();Ok(1)}}
        let job=crate::Job::new(None).unwrap();let mut reader=Reader {control:job.control(),calls:0};
        let error=read(&mut reader,8,Path::new("input"),false,&job.context()).unwrap_err();assert_eq!(error.error,ErrorCode::Cancelled);assert_eq!(reader.calls,1);
        struct Writer {control:crate::JobControl,calls:usize}
        impl Write for Writer {fn write(&mut self,_:&[u8])->io::Result<usize>{self.calls+=1;self.control.cancel();Ok(1)}fn flush(&mut self)->io::Result<()>{panic!("cancelled write must not flush")}}
        let job=crate::Job::new(None).unwrap();let mut writer=Writer {control:job.control(),calls:0};let error=write_output(&mut writer,b"complete",Path::new("output"),Some(&job)).unwrap_err();assert_eq!(error.error,ErrorCode::Cancelled);assert_eq!(writer.calls,1);assert!(error.limit.is_none());assert!(error.bound.is_none());
    }

}
