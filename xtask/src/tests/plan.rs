use super::*;

#[test]
fn shell_script_paths_returns_sorted_shell_scripts_only() {
    let repo_root = tempdir().expect("tempdir");
    let scripts_dir = repo_root.path().join("scripts");
    fs::create_dir_all(&scripts_dir).expect("create scripts dir");
    fs::write(repo_root.path().join("check.sh"), "#!/usr/bin/env bash\n").expect("write check.sh");
    fs::write(scripts_dir.join("b.sh"), "#!/usr/bin/env bash\n").expect("write b.sh");
    fs::write(scripts_dir.join("a.sh"), "#!/usr/bin/env bash\n").expect("write a.sh");
    fs::write(scripts_dir.join("note.txt"), "ignore").expect("write note.txt");

    let scripts = shell_script_paths(repo_root.path()).expect("script paths");

    assert_eq!(
        scripts,
        vec![
            repo_root.path().join("check.sh"),
            scripts_dir.join("a.sh"),
            scripts_dir.join("b.sh"),
        ]
    );
}

#[test]
fn shell_script_paths_returns_empty_when_scripts_dir_is_missing() {
    let repo_root = tempdir().expect("tempdir");

    let scripts = shell_script_paths(repo_root.path()).expect("script paths");

    assert!(scripts.is_empty());
}

#[test]
fn cargo_target_dir_prefers_explicit_environment_over_repo_config() {
    let repo_root = tempdir().expect("tempdir");
    let env_target = Path::new("tmp/managed-target");
    let config_target = Path::new("../.managed-artifacts/target");

    let resolved = crate::plan::cargo_target_dir_from_sources_for_tests(
        repo_root.path(),
        Some(env_target),
        Some(config_target),
    );

    assert_eq!(resolved, repo_root.path().join(env_target));
}

#[test]
fn cargo_build_dir_prefers_explicit_environment_over_repo_config() {
    let repo_root = tempdir().expect("tempdir");
    let env_target = Path::new("tmp/managed-target");
    let config_target = Path::new("../.managed-artifacts/target");
    let env_build = Path::new("tmp/managed-build");
    let config_build = Path::new("../.managed-artifacts/build");

    let resolved = crate::plan::cargo_build_dir_from_sources_for_tests(
        repo_root.path(),
        Some(env_target),
        Some(config_target),
        Some(env_build),
        Some(config_build),
    );

    assert_eq!(resolved, repo_root.path().join(env_build));
}

#[test]
fn cargo_build_dir_follows_environment_target_when_no_build_dir_override_exists() {
    let repo_root = tempdir().expect("tempdir");
    let env_target = Path::new("tmp/managed-target");
    let config_target = Path::new("../.managed-artifacts/target");
    let config_build = Path::new("../.managed-artifacts/build");

    let resolved = crate::plan::cargo_build_dir_from_sources_for_tests(
        repo_root.path(),
        Some(env_target),
        Some(config_target),
        None,
        Some(config_build),
    );

    assert_eq!(resolved, repo_root.path().join(config_build));
}

#[test]
fn cargo_path_helpers_can_opt_into_process_env_lookup_without_changing_defaults() {
    let repo_root = tempdir().expect("tempdir");
    let expected_target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_root.path().join("target"));
    let expected_build = std::env::var_os("CARGO_BUILD_BUILD_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| expected_target.clone());

    crate::plan::with_process_env_passthrough_for_tests(|| {
        assert_eq!(cargo_target_dir(repo_root.path()), expected_target);
        assert_eq!(cargo_build_dir(repo_root.path()), expected_build);
    });
}

#[test]
fn shell_script_paths_use_git_inventory_when_available() {
    let repo_root = tempdir().expect("tempdir");
    fs::write(repo_root.path().join(".git"), "gitdir: /tmp/htmlcut.git\n").expect("write .git");
    fs::write(repo_root.path().join("check.sh"), "#!/usr/bin/env bash\n").expect("write check.sh");
    let scripts_dir = repo_root.path().join("scripts");
    fs::create_dir_all(&scripts_dir).expect("create scripts dir");
    fs::write(
        scripts_dir.join("release-targets.sh"),
        "#!/usr/bin/env bash\n",
    )
    .expect("write release-targets.sh");
    fs::write(scripts_dir.join("local-only.sh"), "#!/usr/bin/env bash\n")
        .expect("write local-only.sh");

    let scripts = crate::command_exec::with_capture_command_output_override(
        |_, spec| {
            (spec.program == std::path::Path::new("git"))
                .then(|| Ok(b"check.sh\0scripts/release-targets.sh\0".to_vec()))
        },
        || shell_script_paths(repo_root.path()),
    )
    .expect("script paths");

    assert_eq!(
        scripts,
        vec![
            repo_root.path().join("check.sh"),
            scripts_dir.join("release-targets.sh"),
        ]
    );
}

#[test]
fn is_maintained_shell_script_rejects_paths_outside_the_repo_root() {
    let repo_root = tempdir().expect("repo tempdir");
    let outside_root = tempdir().expect("outside tempdir");
    let inside_check = repo_root.path().join("check.sh");
    let inside_script = repo_root.path().join("scripts").join("release-targets.sh");
    let nested_script = repo_root
        .path()
        .join("scripts")
        .join("nested")
        .join("release-targets.sh");
    let non_shell_note = repo_root.path().join("scripts").join("notes.txt");
    let outside_script = outside_root.path().join("check.sh");
    fs::create_dir_all(inside_script.parent().expect("parent")).expect("create scripts dir");
    fs::create_dir_all(nested_script.parent().expect("nested parent"))
        .expect("create nested scripts dir");
    fs::write(&inside_check, "#!/usr/bin/env bash\n").expect("write inside check");
    fs::write(&inside_script, "#!/usr/bin/env bash\n").expect("write inside script");
    fs::write(&nested_script, "#!/usr/bin/env bash\n").expect("write nested script");
    fs::write(&non_shell_note, "ignore").expect("write note");
    fs::write(&outside_script, "#!/usr/bin/env bash\n").expect("write outside script");

    assert!(crate::plan::is_maintained_shell_script_for_tests(
        repo_root.path(),
        &inside_check
    ));
    assert!(crate::plan::is_maintained_shell_script_for_tests(
        repo_root.path(),
        &inside_script
    ));
    assert!(!crate::plan::is_maintained_shell_script_for_tests(
        repo_root.path(),
        &nested_script
    ));
    assert!(!crate::plan::is_maintained_shell_script_for_tests(
        repo_root.path(),
        &non_shell_note
    ));
    assert!(!crate::plan::is_maintained_shell_script_for_tests(
        repo_root.path(),
        &outside_script
    ));
}

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
            && spec.args.iter().any(|arg| arg == "--target")
            && spec.args.iter().any(|arg| arg == "x86_64-pc-windows-msvc")
            && spec.args.iter().any(|arg| arg == "check")
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
    assert!(plan.iter().any(|spec| *spec == miri_selector_command()));
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

#[test]
fn ci_rust_gate_plan_builds_the_curated_cross_platform_gate() {
    let repo_root = tempdir().expect("tempdir");
    write_repo_scaffold(repo_root.path());

    let plan = crate::ci_rust_gate_plan(repo_root.path()).expect("ci rust gate plan");

    assert_eq!(
        &plan[..8],
        vec![
            test_command_spec("cargo", ["fmt", "--check"], false, false),
            test_command_spec(
                "cargo",
                [
                    "clippy",
                    "--workspace",
                    "--all-targets",
                    "--all-features",
                    "--locked",
                    "--",
                    "-D",
                    "warnings",
                ],
                false,
                true,
            ),
            test_command_spec(
                "cargo",
                [
                    "test",
                    "-p",
                    "htmlcut-core",
                    "--lib",
                    "--all-features",
                    "--locked",
                ],
                false,
                true,
            ),
            test_command_spec(
                "cargo",
                [
                    "test",
                    "-p",
                    "htmlcut-cli",
                    "--lib",
                    "--tests",
                    "--all-features",
                    "--locked",
                ],
                false,
                true,
            ),
            test_command_spec(
                "cargo",
                [
                    "nextest",
                    "run",
                    "-p",
                    "htmlcut-tempdir",
                    "--lib",
                    "--tests",
                    "--locked",
                ],
                false,
                true,
            ),
            test_command_spec(
                "cargo",
                ["run", "-p", "xtask", "--", "outdated-check",],
                false,
                false,
            ),
            test_command_spec("cargo", ["audit", "-D", "warnings"], false, false),
            deny_check_command(repo_root.path()).expect("deny check"),
        ]
    );
    assert!(is_semver_check_spec(plan.last().expect("semver command")));
    assert!(
        plan.last()
            .expect("semver command")
            .args
            .windows(2)
            .any(|window| window == ["--release-type", "major"])
    );
}

#[test]
fn check_plan_rejects_dirty_semver_baseline() {
    let repo_root = tempdir().expect("tempdir");
    write_repo_scaffold(repo_root.path());
    fs::write(repo_root.path().join(".git"), "gitdir: /tmp/htmlcut.git\n").expect("write .git");
    let baseline_arg = semver_baseline_path(repo_root.path())
        .strip_prefix(repo_root.path())
        .expect("baseline relative to repo root")
        .to_string_lossy()
        .into_owned();
    let expected_message = format!("semver baseline {} is dirty", baseline_arg);

    let error = crate::command_exec::with_capture_command_output_override(
        move |_, spec| {
            (spec.program == std::path::Path::new("git")
                && spec.args
                    == [
                        "status",
                        "--porcelain=1",
                        "--untracked-files=all",
                        "--",
                        baseline_arg.as_str(),
                    ])
            .then(|| Ok(b" M semver-baseline/htmlcut-core/src/contracts/results.rs\n".to_vec()))
        },
        || check_plan(repo_root.path()).expect_err("dirty baseline should fail"),
    );

    let message = error.to_string();
    assert!(message.contains(&expected_message));
    assert!(message.contains("src/contracts/results.rs"));
}

#[test]
fn check_plan_accepts_a_clean_semver_baseline() {
    let repo_root = tempdir().expect("tempdir");
    write_repo_scaffold(repo_root.path());
    fs::write(repo_root.path().join(".git"), "gitdir: /tmp/htmlcut.git\n").expect("write .git");
    let baseline_arg = semver_baseline_path(repo_root.path())
        .strip_prefix(repo_root.path())
        .expect("baseline relative to repo root")
        .to_string_lossy()
        .into_owned();

    let plan = crate::command_exec::with_capture_command_output_override(
        move |_, spec| {
            if spec.program != std::path::Path::new("git") {
                return None;
            }
            if spec.args
                == [
                    "status",
                    "--porcelain=1",
                    "--untracked-files=all",
                    "--",
                    baseline_arg.as_str(),
                ]
            {
                return Some(Ok(Vec::new()));
            }
            Some(Ok(b"check.sh\0scripts/release-targets.sh\0".to_vec()))
        },
        || check_plan(repo_root.path()),
    )
    .expect("clean baseline should pass");

    assert!(!plan.is_empty());
}

#[test]
fn check_plan_lints_the_canonical_release_script_even_without_extra_shell_files() {
    let repo_root = tempdir().expect("tempdir");
    write_repo_scaffold(repo_root.path());

    let plan = check_plan(repo_root.path()).expect("check plan");

    assert_eq!(
        plan[0],
        test_command_spec(
            "bash",
            [
                "-n".to_owned(),
                repo_root
                    .path()
                    .join("scripts")
                    .join("release-targets.sh")
                    .to_string_lossy()
                    .into_owned(),
            ],
            false,
            false,
        )
    );
    assert_eq!(
        plan[1],
        test_command_spec(
            "shellcheck",
            [repo_root
                .path()
                .join("scripts")
                .join("release-targets.sh")
                .to_string_lossy()
                .into_owned()],
            false,
            false,
        )
    );
    assert_eq!(
        plan[2],
        test_command_spec("cargo", ["fmt", "--check"], false, false)
    );
}

#[test]
fn check_plan_keeps_the_cli_lib_gate_when_cli_test_targets_are_missing() {
    let repo_root = tempdir().expect("tempdir");
    write_repo_scaffold(repo_root.path());
    fs::remove_dir_all(
        repo_root
            .path()
            .join("crates")
            .join("htmlcut-cli")
            .join("tests"),
    )
    .expect("remove htmlcut-cli tests dir");

    let plan = check_plan(repo_root.path()).expect("check plan");

    assert!(plan.iter().any(|spec| {
        spec.args
            == [
                "build",
                "--profile",
                "dist",
                "-p",
                "htmlcut-cli",
                "--bin",
                "htmlcut",
                "--locked",
            ]
    }));
    assert!(!plan.iter().any(|spec| {
        spec.args.first().map(String::as_str) == Some("nextest")
            && spec.args.iter().any(|arg| arg == "--test")
    }));
}

#[test]
fn check_plan_still_builds_core_steps_before_missing_release_registry_errors() {
    let repo_root = tempdir().expect("tempdir");
    fs::write(
        repo_root.path().join("Cargo.toml"),
        "[workspace.package]\nversion = \"3.0.0\"\n",
    )
    .expect("write Cargo.toml");
    fs::write(repo_root.path().join("changelog.md"), "## [Unreleased]\n").expect("write changelog");
    let baseline_dir = repo_root
        .path()
        .join("semver-baseline")
        .join("htmlcut-core");
    fs::create_dir_all(&baseline_dir).expect("create semver baseline");
    fs::write(
        baseline_dir.join("Cargo.toml"),
        "[package]\nname = \"htmlcut-core\"\nversion = \"2.0.0\"\n",
    )
    .expect("write baseline manifest");

    let error = check_plan(repo_root.path()).expect_err("missing release registry should fail");

    assert!(
        error
            .to_string()
            .contains("missing canonical release target script")
    );
}

#[test]
fn coverage_command_targets_repo_coverage_file() {
    let repo_root = tempdir().expect("tempdir");

    let command = coverage_command(repo_root.path());
    let clean = coverage_clean_command();
    let expected_output_path = coverage_output_path(repo_root.path());

    assert_eq!(command.program, PathBuf::from("cargo"));
    assert!(command_forces_clang(&command));
    assert!(command_uses_managed_coverage_artifacts(&command));
    assert_eq!(
        command.args,
        vec![
            "+nightly".to_owned(),
            "llvm-cov".to_owned(),
            "--branch".to_owned(),
            "-p".to_owned(),
            "htmlcut-core".to_owned(),
            "-p".to_owned(),
            "htmlcut-cli".to_owned(),
            "-p".to_owned(),
            "htmlcut-tempdir".to_owned(),
            "-p".to_owned(),
            "xtask".to_owned(),
            "--all-targets".to_owned(),
            "--all-features".to_owned(),
            "--locked".to_owned(),
            "--json".to_owned(),
            "--output-path".to_owned(),
            expected_output_path.to_string_lossy().into_owned(),
        ]
    );
    assert_eq!(clean.program, PathBuf::from("cargo"));
    assert!(!command_forces_clang(&clean));
    assert!(command_uses_managed_coverage_artifacts(&clean));
    assert_eq!(
        clean.args,
        vec![
            "+nightly".to_owned(),
            "llvm-cov".to_owned(),
            "clean".to_owned(),
            "--workspace".to_owned(),
        ]
    );
}

#[test]
fn coverage_output_path_uses_the_configured_target_dir_layout() {
    let repo_root = tempdir().expect("tempdir");
    let absolute_target_root = tempdir().expect("absolute target tempdir");
    let absolute_target_dir = absolute_target_root.path().join("htmlcut-gate");

    assert_eq!(
        crate::coverage::coverage_output_path_for_tests(repo_root.path(), None),
        repo_root
            .path()
            .join("coverage-target")
            .join("coverage.json")
    );
    assert_eq!(
        crate::coverage::coverage_output_path_for_tests(
            repo_root.path(),
            Some(&absolute_target_dir),
        ),
        absolute_target_dir
            .parent()
            .expect("absolute target parent")
            .join("coverage-target")
            .join("coverage.json")
    );
    assert_eq!(
        crate::coverage::coverage_output_path_for_tests(
            repo_root.path(),
            Some(Path::new("custom-target")),
        ),
        repo_root
            .path()
            .join("coverage-target")
            .join("coverage.json")
    );
}

#[test]
fn semver_scratch_dir_uses_target_tree() {
    let repo_root = tempdir().expect("tempdir");
    let absolute_target_root = tempdir().expect("absolute target tempdir");
    let absolute_target_dir = absolute_target_root.path().join("htmlcut-gate");

    assert_eq!(
        semver_scratch_dir(repo_root.path()),
        repo_root.path().join("target").join("semver-checks")
    );
    assert_eq!(
        crate::plan::semver_scratch_dir_for_tests(repo_root.path(), None),
        repo_root.path().join("target").join("semver-checks")
    );
    assert_eq!(
        crate::plan::semver_scratch_dir_for_tests(repo_root.path(), Some(&absolute_target_dir)),
        absolute_target_dir.join("semver-checks")
    );
    assert_eq!(
        crate::plan::semver_scratch_dir_for_tests(
            repo_root.path(),
            Some(Path::new("custom-target"))
        ),
        repo_root.path().join("custom-target").join("semver-checks")
    );
}

#[test]
fn release_binary_path_uses_the_configured_target_dir_layout() {
    let repo_root = tempdir().expect("tempdir");
    let absolute_target_root = tempdir().expect("absolute target tempdir");
    let absolute_target_dir = absolute_target_root.path().join("htmlcut-gate");

    assert_eq!(
        crate::plan::release_binary_path_for_tests(repo_root.path(), None),
        repo_root
            .path()
            .join("target")
            .join("dist")
            .join(binary_name())
    );
    assert_eq!(
        crate::plan::release_binary_path_for_tests(repo_root.path(), Some(&absolute_target_dir)),
        absolute_target_dir.join("dist").join(binary_name())
    );
    assert_eq!(
        crate::plan::release_binary_path_for_tests(
            repo_root.path(),
            Some(Path::new("custom-target"))
        ),
        repo_root
            .path()
            .join("custom-target")
            .join("dist")
            .join(binary_name())
    );
}

#[test]
fn plan_path_helpers_expose_canonical_workspace_layout() {
    let repo_root = tempdir().expect("tempdir");

    assert_eq!(
        core_manifest_path(repo_root.path()),
        repo_root
            .path()
            .join("crates")
            .join("htmlcut-core")
            .join("Cargo.toml")
    );
    assert_eq!(
        semver_baseline_path(repo_root.path()),
        repo_root
            .path()
            .join("semver-baseline")
            .join("htmlcut-core")
    );
}

#[test]
fn is_semver_check_spec_matches_only_the_semver_gate() {
    assert!(is_semver_check_spec(&test_command_spec(
        "cargo",
        ["semver-checks", "--all-features"],
        false,
        true
    )));
    assert!(!is_semver_check_spec(&test_command_spec(
        "cargo",
        ["nextest", "run"],
        false,
        true
    )));
    assert!(!is_semver_check_spec(&test_command_spec(
        "bash",
        ["check.sh"],
        false,
        false
    )));
}
