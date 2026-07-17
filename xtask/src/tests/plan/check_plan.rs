use super::*;

#[test]
fn check_plan_includes_all_strict_gates() {
    let repo_root = tempdir().expect("tempdir");
    write_repo_scaffold(repo_root.path());
    fs::write(repo_root.path().join("check.sh"), "#!/usr/bin/env bash\n").expect("write check.sh");
    let scripts_dir = repo_root.path().join("scripts");
    fs::create_dir_all(&scripts_dir).expect("create scripts dir");
    fs::write(scripts_dir.join("z.sh"), "#!/usr/bin/env bash\n").expect("write z.sh");
    fs::write(scripts_dir.join("a.sh"), "#!/usr/bin/env bash\n").expect("write a.sh");
    let release_targets = scripts_dir.join("release-targets.sh");

    let plan = check_plan(repo_root.path()).expect("check plan");

    assert_eq!(
        plan[0],
        test_command_spec(
            "bash",
            [
                "-n".to_owned(),
                repo_root
                    .path()
                    .join("check.sh")
                    .to_string_lossy()
                    .into_owned(),
                scripts_dir.join("a.sh").to_string_lossy().into_owned(),
                release_targets.to_string_lossy().into_owned(),
                scripts_dir.join("z.sh").to_string_lossy().into_owned(),
            ],
            false,
            false,
        )
    );
    assert_eq!(
        plan[1],
        test_command_spec(
            "shellcheck",
            [
                repo_root
                    .path()
                    .join("check.sh")
                    .to_string_lossy()
                    .into_owned(),
                scripts_dir.join("a.sh").to_string_lossy().into_owned(),
                release_targets.to_string_lossy().into_owned(),
                scripts_dir.join("z.sh").to_string_lossy().into_owned(),
            ],
            false,
            false,
        )
    );
    assert_eq!(
        plan[2],
        test_command_spec("cargo", ["fmt", "--check"], false, false)
    );
    assert!(
        plan.iter()
            .any(|spec| spec.args == ["run", "-p", "xtask", "--", "outdated-check",])
    );
    assert!(
        plan.iter()
            .any(|spec| spec.args == ["audit", "-D", "warnings"])
    );
    assert!(plan.iter().any(|spec| {
        spec.program == std::path::Path::new("cargo")
            && spec.args.first().map(String::as_str) == Some("deny")
            && spec.args.iter().any(|arg| arg == "check")
            && !spec.args.iter().any(|arg| arg == "--target")
    }));
    assert!(plan.iter().any(|spec| {
        spec.args
            .windows(2)
            .any(|window| window == ["--release-type", "major"])
    }));
    assert!(plan.iter().any(|spec| {
        spec.args
            == [
                "check",
                "-p",
                "htmlcut-fuzz",
                "--bins",
                "--features",
                "fuzzing",
                "--locked",
            ]
    }));
    assert!(plan.iter().any(|spec| *spec == miri_contract_command()));
    assert!(
        !plan
            .iter()
            .any(|spec| { spec.args == ["nextest", "run", "-p", "xtask", "--tests", "--locked"] })
    );
    assert!(!plan.iter().any(|spec| {
        spec.args
            == [
                "nextest",
                "run",
                "-p",
                "htmlcut-core",
                "--lib",
                "--locked",
                "contract_lint",
            ]
    }));
    assert!(!plan.iter().any(|spec| {
        spec.args
            == [
                "nextest",
                "run",
                "-p",
                "htmlcut-cli",
                "--lib",
                "--locked",
                "contract_lint",
            ]
    }));
    assert!(!plan.iter().any(|spec| {
        spec.args
            == [
                "nextest",
                "run",
                "-p",
                "htmlcut-tempdir",
                "--lib",
                "--tests",
                "--locked",
            ]
    }));
    assert!(!plan.iter().any(|spec| {
        spec.args
            == [
                "test",
                "-p",
                "htmlcut-core",
                "--lib",
                "--all-features",
                "--locked",
            ]
    }));
    assert!(!plan.iter().any(|spec| {
        spec.args
            == [
                "test",
                "-p",
                "htmlcut-cli",
                "--lib",
                "--tests",
                "--all-features",
                "--locked",
            ]
    }));
    assert!(!plan.iter().any(|spec| {
        spec.args.first().map(String::as_str) == Some("nextest")
            && spec.args.iter().any(|arg| arg == "--test")
    }));
    assert!(
        plan.iter()
            .any(|spec| { spec.args == ["doc", "--workspace", "--no-deps", "--locked"] })
    );
    assert!(plan.iter().all(|spec| {
        !spec.args.iter().any(|arg| {
            arg == repo_root
                .path()
                .join("fuzz")
                .join("Cargo.toml")
                .to_string_lossy()
                .as_ref()
                || arg
                    == repo_root
                        .path()
                        .join("fuzz")
                        .join("Cargo.lock")
                        .to_string_lossy()
                        .as_ref()
        })
    }));
    assert_eq!(
        plan.last().expect("release smoke"),
        &test_command_spec(
            release_binary_path(repo_root.path()),
            ["--version"],
            true,
            false
        )
    );
}

#[test]
fn check_plan_runs_devcontainer_validation_when_branch_diff_touches_gate_inputs() {
    let repo_root = tempdir().expect("tempdir");
    write_repo_scaffold(repo_root.path());
    fs::write(repo_root.path().join(".git"), "gitdir: /tmp/htmlcut.git\n").expect("write .git");
    let validator = repo_root
        .path()
        .join("scripts")
        .join("validate-devcontainer.sh")
        .to_string_lossy()
        .into_owned();

    let plan = crate::command_exec::with_capture_command_output_override(
        move |_, spec| {
            if spec.program != std::path::Path::new("git") {
                return None;
            }
            if spec.args == ["merge-base", "HEAD", "origin/main"] {
                return Some(Ok(b"abc123\n".to_vec()));
            }
            if spec.args
                == [
                    "diff",
                    "--name-only",
                    "-z",
                    "abc123",
                    "--",
                    ".devcontainer",
                    "scripts/validate-devcontainer.sh",
                    "scripts/devcontainer-check.sh",
                    "scripts/devcontainer-prepare-user-home.sh",
                    "scripts/devcontainer-bootstrap.sh",
                    "scripts/devcontainer-cli-helper.Dockerfile",
                    "scripts/common.sh",
                    "scripts/xtask.sh",
                    "check.sh",
                ]
            {
                return Some(Ok(b"scripts/devcontainer-bootstrap.sh\0".to_vec()));
            }
            if spec.args
                == [
                    "status",
                    "--porcelain=1",
                    "--untracked-files=all",
                    "--",
                    "semver-baseline/htmlcut-core",
                ]
            {
                return Some(Ok(Vec::new()));
            }
            Some(Ok(b"check.sh\0scripts/release-targets.sh\0".to_vec()))
        },
        || check_plan(repo_root.path()),
    )
    .expect("check plan");

    assert!(
        plan.iter().any(|spec| {
            *spec == crate::plan::devcontainer_validation_command_for_tests(repo_root.path())
        }),
        "expected devcontainer validation to be part of the local maintainer gate when relevant files changed"
    );
    assert_eq!(
        plan[2],
        crate::plan::devcontainer_validation_command_for_tests(repo_root.path())
    );
    assert_eq!(
        plan[3],
        test_command_spec("cargo", ["fmt", "--check"], false, false)
    );
    assert_eq!(
        plan[2],
        test_command_spec("bash", [validator], false, false)
    );
}

#[test]
fn check_plan_skips_devcontainer_validation_when_branch_diff_is_clean() {
    let repo_root = tempdir().expect("tempdir");
    write_repo_scaffold(repo_root.path());
    fs::write(repo_root.path().join(".git"), "gitdir: /tmp/htmlcut.git\n").expect("write .git");

    let plan = crate::command_exec::with_capture_command_output_override(
        |_, spec| {
            if spec.program != std::path::Path::new("git") {
                return None;
            }
            if spec.args == ["merge-base", "HEAD", "origin/main"] {
                return Some(Ok(b"abc123\n".to_vec()));
            }
            if spec.args
                == [
                    "diff",
                    "--name-only",
                    "-z",
                    "abc123",
                    "--",
                    ".devcontainer",
                    "scripts/validate-devcontainer.sh",
                    "scripts/devcontainer-check.sh",
                    "scripts/devcontainer-prepare-user-home.sh",
                    "scripts/devcontainer-bootstrap.sh",
                    "scripts/devcontainer-cli-helper.Dockerfile",
                    "scripts/common.sh",
                    "scripts/xtask.sh",
                    "check.sh",
                ]
            {
                return Some(Ok(Vec::new()));
            }
            if spec.args
                == [
                    "ls-files",
                    "--others",
                    "--exclude-standard",
                    "-z",
                    "--",
                    ".devcontainer",
                    "scripts/validate-devcontainer.sh",
                    "scripts/devcontainer-check.sh",
                    "scripts/devcontainer-prepare-user-home.sh",
                    "scripts/devcontainer-bootstrap.sh",
                    "scripts/devcontainer-cli-helper.Dockerfile",
                    "scripts/common.sh",
                    "scripts/xtask.sh",
                    "check.sh",
                ]
            {
                return Some(Ok(Vec::new()));
            }
            if spec.args
                == [
                    "status",
                    "--porcelain=1",
                    "--untracked-files=all",
                    "--",
                    "semver-baseline/htmlcut-core",
                ]
            {
                return Some(Ok(Vec::new()));
            }
            Some(Ok(b"check.sh\0scripts/release-targets.sh\0".to_vec()))
        },
        || check_plan(repo_root.path()),
    )
    .expect("check plan");

    assert!(!plan.iter().any(|spec| {
        *spec == crate::plan::devcontainer_validation_command_for_tests(repo_root.path())
    }));
    assert_eq!(
        plan[2],
        test_command_spec("cargo", ["fmt", "--check"], false, false)
    );
}

#[test]
fn devcontainer_changed_file_args_fall_back_to_head_when_merge_base_is_unavailable() {
    let repo_root = tempdir().expect("tempdir");
    let seen_quiet_stderr = std::rc::Rc::new(std::cell::Cell::new(false));
    let seen_quiet_stderr_for_probe = std::rc::Rc::clone(&seen_quiet_stderr);

    let args = crate::command_exec::with_capture_command_output_override(
        move |_, spec| {
            if spec.program == std::path::Path::new("git")
                && spec.args == ["merge-base", "HEAD", "origin/main"]
            {
                seen_quiet_stderr_for_probe.set(command_quiets_stderr(spec));
                return Some(Err("merge-base unavailable".into()));
            }
            None
        },
        || crate::plan::devcontainer_changed_file_args_for_tests(repo_root.path()),
    )
    .expect("changed file args");

    assert!(seen_quiet_stderr.get());
    assert_eq!(
        args,
        vec![
            "diff".to_owned(),
            "--name-only".to_owned(),
            "-z".to_owned(),
            "HEAD".to_owned(),
            "--".to_owned(),
            ".devcontainer".to_owned(),
            "scripts/validate-devcontainer.sh".to_owned(),
            "scripts/devcontainer-check.sh".to_owned(),
            "scripts/devcontainer-prepare-user-home.sh".to_owned(),
            "scripts/devcontainer-bootstrap.sh".to_owned(),
            "scripts/devcontainer-cli-helper.Dockerfile".to_owned(),
            "scripts/common.sh".to_owned(),
            "scripts/xtask.sh".to_owned(),
            "check.sh".to_owned(),
        ]
    );
}

#[test]
fn should_run_devcontainer_validation_checks_untracked_gate_inputs_when_diff_is_clean() {
    let repo_root = tempdir().expect("tempdir");
    fs::write(repo_root.path().join(".git"), "gitdir: /tmp/htmlcut.git\n").expect("write .git");

    let should_run = crate::command_exec::with_capture_command_output_override(
        |_, spec| {
            if spec.program != std::path::Path::new("git") {
                return None;
            }
            if spec.args == ["merge-base", "HEAD", "origin/main"] {
                return Some(Ok(b"abc123\n".to_vec()));
            }
            if spec.args
                == [
                    "diff",
                    "--name-only",
                    "-z",
                    "abc123",
                    "--",
                    ".devcontainer",
                    "scripts/validate-devcontainer.sh",
                    "scripts/devcontainer-check.sh",
                    "scripts/devcontainer-prepare-user-home.sh",
                    "scripts/devcontainer-bootstrap.sh",
                    "scripts/devcontainer-cli-helper.Dockerfile",
                    "scripts/common.sh",
                    "scripts/xtask.sh",
                    "check.sh",
                ]
            {
                return Some(Ok(Vec::new()));
            }
            if spec.args == crate::plan::devcontainer_untracked_file_args_for_tests() {
                return Some(Ok(b".devcontainer/devcontainer.json\0".to_vec()));
            }
            None
        },
        || crate::plan::should_run_devcontainer_validation_for_tests(repo_root.path()),
    )
    .expect("devcontainer decision");

    assert!(should_run);
}

#[test]
fn should_run_devcontainer_validation_propagates_changed_scan_failures() {
    let repo_root = tempdir().expect("tempdir");
    fs::write(repo_root.path().join(".git"), "gitdir: /tmp/htmlcut.git\n").expect("write .git");

    let error = crate::command_exec::with_capture_command_output_override(
        |_, spec| {
            if spec.program != std::path::Path::new("git") {
                return None;
            }
            if spec.args == ["merge-base", "HEAD", "origin/main"] {
                return Some(Ok(b"abc123\n".to_vec()));
            }
            if spec.args
                == [
                    "diff",
                    "--name-only",
                    "-z",
                    "abc123",
                    "--",
                    ".devcontainer",
                    "scripts/validate-devcontainer.sh",
                    "scripts/devcontainer-check.sh",
                    "scripts/devcontainer-prepare-user-home.sh",
                    "scripts/devcontainer-bootstrap.sh",
                    "scripts/devcontainer-cli-helper.Dockerfile",
                    "scripts/common.sh",
                    "scripts/xtask.sh",
                    "check.sh",
                ]
            {
                return Some(Err("git diff failed".into()));
            }
            None
        },
        || crate::plan::should_run_devcontainer_validation_for_tests(repo_root.path()),
    )
    .expect_err("changed scan should surface failures");

    assert!(error.to_string().contains("git diff failed"));
}

#[test]
fn should_run_devcontainer_validation_propagates_untracked_scan_failures() {
    let repo_root = tempdir().expect("tempdir");
    fs::write(repo_root.path().join(".git"), "gitdir: /tmp/htmlcut.git\n").expect("write .git");

    let error = crate::command_exec::with_capture_command_output_override(
        |_, spec| {
            if spec.program != std::path::Path::new("git") {
                return None;
            }
            if spec.args == ["merge-base", "HEAD", "origin/main"] {
                return Some(Ok(b"abc123\n".to_vec()));
            }
            if spec.args
                == [
                    "diff",
                    "--name-only",
                    "-z",
                    "abc123",
                    "--",
                    ".devcontainer",
                    "scripts/validate-devcontainer.sh",
                    "scripts/devcontainer-check.sh",
                    "scripts/devcontainer-prepare-user-home.sh",
                    "scripts/devcontainer-bootstrap.sh",
                    "scripts/devcontainer-cli-helper.Dockerfile",
                    "scripts/common.sh",
                    "scripts/xtask.sh",
                    "check.sh",
                ]
            {
                return Some(Ok(Vec::new()));
            }
            if spec.args == crate::plan::devcontainer_untracked_file_args_for_tests() {
                return Some(Err("git ls-files failed".into()));
            }
            None
        },
        || crate::plan::should_run_devcontainer_validation_for_tests(repo_root.path()),
    )
    .expect_err("untracked scan should surface failures");

    assert!(error.to_string().contains("git ls-files failed"));
}
