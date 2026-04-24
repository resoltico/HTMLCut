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
        CommandSpec::new(
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
        CommandSpec::new(
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
        CommandSpec::new("cargo", ["fmt", "--check"], false, false)
    );
    assert!(
        plan.iter()
            .any(|spec| { spec.args == ["test", "-p", "xtask", "--lib", "--locked"] })
    );
    assert!(plan.iter().any(|spec| {
        spec.args
            == [
                "test",
                "-p",
                "htmlcut-core",
                "--lib",
                "--locked",
                "contract_lint",
            ]
    }));
    assert!(plan.iter().any(|spec| {
        spec.args
            == [
                "test",
                "-p",
                "htmlcut-cli",
                "--lib",
                "--locked",
                "contract_lint",
            ]
    }));
    assert!(plan.iter().any(|spec| spec.args
        == [
            "outdated",
            "--workspace",
            "--root-deps-only",
            "--exit-code",
            "1"
        ]));
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
    assert!(plan.iter().any(|spec| {
        spec.args
            == [
                "nextest",
                "run",
                "--workspace",
                "--lib",
                "--tests",
                "--all-features",
                "--locked",
            ]
    }));
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
        &CommandSpec::new(
            release_binary_path(repo_root.path()),
            ["--version"],
            true,
            false
        )
    );
}

#[test]
fn check_plan_lints_the_canonical_release_script_even_without_extra_shell_files() {
    let repo_root = tempdir().expect("tempdir");
    write_repo_scaffold(repo_root.path());

    let plan = check_plan(repo_root.path()).expect("check plan");

    assert_eq!(
        plan[0],
        CommandSpec::new(
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
        CommandSpec::new(
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
        CommandSpec::new("cargo", ["fmt", "--check"], false, false)
    );
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
    assert!(command.force_clang);
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
    assert!(!clean.force_clang);
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

    assert_eq!(
        crate::coverage::coverage_output_path_for_tests(repo_root.path(), None),
        repo_root.path().join("target").join("coverage.json")
    );
    assert_eq!(
        crate::coverage::coverage_output_path_for_tests(
            repo_root.path(),
            Some(Path::new("/tmp/htmlcut-gate")),
        ),
        Path::new("/tmp/htmlcut-gate").join("coverage.json")
    );
    assert_eq!(
        crate::coverage::coverage_output_path_for_tests(
            repo_root.path(),
            Some(Path::new("custom-target")),
        ),
        repo_root.path().join("custom-target").join("coverage.json")
    );
}

#[test]
fn semver_scratch_dir_uses_target_tree() {
    let repo_root = tempdir().expect("tempdir");
    let expected_live_target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .map(|target_dir| {
            if target_dir.is_absolute() {
                target_dir
            } else {
                repo_root.path().join(target_dir)
            }
        })
        .unwrap_or_else(|| repo_root.path().join("target"));

    assert_eq!(
        semver_scratch_dir(repo_root.path()),
        expected_live_target.join("semver-checks")
    );
    assert_eq!(
        crate::plan::semver_scratch_dir_for_tests(repo_root.path(), None),
        repo_root.path().join("target").join("semver-checks")
    );
    assert_eq!(
        crate::plan::semver_scratch_dir_for_tests(
            repo_root.path(),
            Some(Path::new("/tmp/htmlcut-gate"))
        ),
        Path::new("/tmp/htmlcut-gate").join("semver-checks")
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

    assert_eq!(
        crate::plan::release_binary_path_for_tests(repo_root.path(), None),
        repo_root
            .path()
            .join("target")
            .join("dist")
            .join(binary_name())
    );
    assert_eq!(
        crate::plan::release_binary_path_for_tests(
            repo_root.path(),
            Some(Path::new("/tmp/htmlcut-gate"))
        ),
        Path::new("/tmp/htmlcut-gate")
            .join("dist")
            .join(binary_name())
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
    assert!(is_semver_check_spec(&CommandSpec::new(
        "cargo",
        ["semver-checks", "--all-features"],
        false,
        true
    )));
    assert!(!is_semver_check_spec(&CommandSpec::new(
        "cargo",
        ["nextest", "run"],
        false,
        true
    )));
    assert!(!is_semver_check_spec(&CommandSpec::new(
        "bash",
        ["check.sh"],
        false,
        false
    )));
}
