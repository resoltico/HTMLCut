use super::*;

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
                    "--no-deps",
                    "--",
                    "-D",
                    "warnings",
                ],
                false,
                false,
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
                false,
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
                false,
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
                false,
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
    assert!(plan.iter().all(|spec| !command_forces_clang(spec)));
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
