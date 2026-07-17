use super::*;

#[test]
fn run_spec_executes_successfully_with_and_without_clang_override() {
    with_isolated_managed_workspace_artifacts(|repo_root, target_dir, build_dir| {
        run_spec(
            repo_root,
            &test_command_spec("cargo", ["--version"], true, false),
        )
        .expect("plain command");
        run_spec(
            repo_root,
            &test_command_spec("cargo", ["--version"], true, false)
                .with_stderr(CommandStderr::Quiet),
        )
        .expect("stderr-quiet command");
        run_spec(
            repo_root,
            &test_command_spec("cargo", ["--version"], true, true),
        )
        .expect("clang-forced command");
        run_spec(
            repo_root,
            &test_command_spec("cargo", ["--version"], false, false),
        )
        .expect("stdout inherited command");

        for managed_root in [target_dir, build_dir] {
            assert!(
                managed_root.is_dir(),
                "{} should exist",
                managed_root.display()
            );
            assert!(managed_root.join("CACHEDIR.TAG").is_file());
            assert!(managed_root.join(".htmlcut-artifact.toml").is_file());
        }
    });
}

#[test]
fn run_spec_reports_test_owned_non_zero_status_without_leaking_a_fake_diagnostic() {
    with_isolated_managed_workspace_artifacts(|repo_root, _, _| {
        let error = crate::command_exec::with_run_spec_override(
            |_, _| Some(Err("expected probe failure".into())),
            || {
                run_spec(
                    repo_root,
                    &test_command_spec("cargo", ["--version"], true, false)
                        .with_stderr(CommandStderr::Quiet),
                )
            },
        )
        .expect_err("failing command");

        assert!(
            error.to_string().contains("expected probe failure"),
            "expected subprocess failure should stay test-owned: {error}"
        );
    });
}

#[test]
fn capture_command_output_returns_stdout_and_reports_test_owned_failure() {
    with_isolated_managed_workspace_artifacts(|repo_root, _, _| {
        let output = capture_command_output(
            repo_root,
            &test_command_spec("cargo", ["--version"], false, true),
        )
        .expect("stdout");
        assert!(String::from_utf8(output).expect("utf8").contains("cargo"));

        let error = crate::command_exec::with_capture_command_output_override(
            |_, _| Some(Err("expected capture failure".into())),
            || {
                capture_command_output(
                    repo_root,
                    &test_command_spec("cargo", ["--version"], false, false)
                        .with_stderr(CommandStderr::Quiet),
                )
            },
        )
        .expect_err("failing command");
        assert!(
            error.to_string().contains("expected capture failure"),
            "expected captured failure should stay test-owned: {error}"
        );
    });
}

#[test]
fn non_reporting_command_failures_retain_both_streams_without_test_runner_noise() {
    with_isolated_managed_workspace_artifacts(|repo_root, _, _| {
        let spec = test_command_spec(
            "cargo",
            ["__htmlcut_command_exec_test_failure__"],
            true,
            false,
        )
        .with_stderr(CommandStderr::Quiet);
        let captured_streams = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let captured_streams_for_override = std::rc::Rc::clone(&captured_streams);

        crate::command_exec::with_stream_write_override(
            move |stderr, bytes| {
                captured_streams_for_override
                    .borrow_mut()
                    .push((stderr, bytes.to_vec()));
                Ok(())
            },
            || {
                let run_error = run_spec(repo_root, &spec).expect_err("run command failure");
                assert!(run_error.to_string().contains("command failed with status"));
                assert!(run_error.to_string().contains("stderr:"));

                let capture_error =
                    capture_command_output(repo_root, &spec).expect_err("capture command failure");
                assert!(
                    capture_error
                        .to_string()
                        .contains("command failed with status")
                );
            },
        );

        assert!(
            captured_streams
                .borrow()
                .iter()
                .any(|(stderr, bytes)| *stderr && !bytes.is_empty()),
            "failing streams must be retained and offered to the caller-owned writer"
        );
    });
}

#[test]
fn command_failure_messages_bound_and_label_combined_streams() {
    let spec = test_command_spec("cargo", ["--version"], true, false);
    let failed = std::process::Command::new("cargo")
        .arg("__htmlcut_command_exec_test_failure__")
        .output()
        .expect("run failing cargo fixture");
    assert!(!failed.status.success(), "fixture command must fail");

    let detailed = crate::command_exec::command_failure_message_for_tests(
        &spec,
        failed.status,
        b"synthetic stdout",
        b"synthetic stderr",
    );
    assert!(detailed.contains("cargo --version"));
    assert!(detailed.contains("stdout:\nsynthetic stdout"));
    assert!(detailed.contains("stderr:\nsynthetic stderr"));

    let quiet =
        crate::command_exec::command_failure_message_for_tests(&spec, failed.status, b"", b"");
    assert!(!quiet.contains("stdout:"));
    assert!(!quiet.contains("stderr:"));
}

#[test]
fn command_environment_for_tests_applies_explicit_overrides() {
    let spec = test_command_spec("cargo", ["--version"], true, false)
        .with_stderr(CommandStderr::Quiet)
        .with_env("MIRIFLAGS", "-Zmiri-strict-provenance");

    let env_pairs = crate::command_exec::command_environment_for_tests(&spec);
    let miriflags = env_pairs
        .into_iter()
        .find(|(key, _)| key == "MIRIFLAGS")
        .and_then(|(_, value)| value)
        .expect("MIRIFLAGS override");
    assert_eq!(miriflags, "-Zmiri-strict-provenance");
    assert!(command_quiets_stderr(&spec));
}

#[test]
fn remove_dir_if_exists_is_idempotent_and_repo_root_points_at_workspace() {
    let tempdir = tempdir().expect("tempdir");
    let removable = tempdir.path().join("scratch");
    fs::create_dir_all(removable.join("nested")).expect("create dir");
    fs::write(removable.join("nested").join("file.txt"), "hello").expect("write file");

    remove_dir_if_exists(&removable).expect("remove existing dir");
    assert!(!removable.exists());
    remove_dir_if_exists(&removable).expect("remove missing dir");

    assert_eq!(
        repo_root(),
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace root")
    );
}

#[test]
fn repo_worktree_files_use_git_inventory_when_available() {
    let repo_root = tempdir().expect("tempdir");
    fs::write(repo_root.path().join(".git"), "gitdir: /tmp/htmlcut.git\n").expect("write .git");
    fs::write(repo_root.path().join("README.md"), "# readme\n").expect("write readme");
    fs::create_dir_all(repo_root.path().join("scripts")).expect("create scripts dir");
    fs::write(
        repo_root.path().join("scripts").join("release-targets.sh"),
        "#!/usr/bin/env bash\n",
    )
    .expect("write release script");

    let files = crate::command_exec::with_capture_command_output_override(
        |_, spec| {
            (spec.program == Path::new("git"))
                .then(|| Ok(b"README.md\0scripts/release-targets.sh\0missing.md\0".to_vec()))
        },
        || crate::command_exec::repo_worktree_files(repo_root.path()),
    )
    .expect("repo worktree files")
    .expect("git inventory");

    assert_eq!(
        files,
        vec![
            repo_root.path().join("README.md"),
            repo_root.path().join("scripts").join("release-targets.sh"),
        ]
    );
}

#[test]
fn repo_worktree_files_return_none_outside_git_worktrees() {
    let repo_root = tempdir().expect("tempdir");

    assert!(
        crate::command_exec::repo_worktree_files(repo_root.path())
            .expect("repo worktree files")
            .is_none()
    );
}

#[test]
fn repo_worktree_files_propagate_git_inventory_failures() {
    let repo_root = tempdir().expect("tempdir");
    fs::write(repo_root.path().join(".git"), "gitdir: /tmp/htmlcut.git\n").expect("write .git");

    let error = crate::command_exec::with_capture_command_output_override(
        |_, spec| (spec.program == Path::new("git")).then(|| Err("git failed".into())),
        || crate::command_exec::repo_worktree_files(repo_root.path()),
    )
    .expect_err("git failure should surface");

    assert!(error.to_string().contains("git failed"));
}
