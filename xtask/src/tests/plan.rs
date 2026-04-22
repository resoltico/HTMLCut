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
fn check_plan_includes_all_strict_gates() {
    let repo_root = tempdir().expect("tempdir");
    write_repo_scaffold(repo_root.path());
    fs::write(repo_root.path().join("check.sh"), "#!/usr/bin/env bash\n").expect("write check.sh");
    let scripts_dir = repo_root.path().join("scripts");
    fs::create_dir_all(&scripts_dir).expect("create scripts dir");
    fs::write(scripts_dir.join("z.sh"), "#!/usr/bin/env bash\n").expect("write z.sh");
    fs::write(scripts_dir.join("a.sh"), "#!/usr/bin/env bash\n").expect("write a.sh");

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
                scripts_dir.join("z.sh").to_string_lossy().into_owned(),
            ],
            false,
            false,
        )
    );
    assert!(plan.iter().any(|spec| {
        spec.args
            == [
                "fmt",
                "--check",
                "--manifest-path",
                repo_root
                    .path()
                    .join("fuzz")
                    .join("Cargo.toml")
                    .to_string_lossy()
                    .as_ref(),
            ]
    }));
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
    assert!(plan.iter().any(|spec| {
        spec.args
            == [
                "clippy",
                "--manifest-path",
                repo_root
                    .path()
                    .join("fuzz")
                    .join("Cargo.toml")
                    .to_string_lossy()
                    .as_ref(),
                "--bins",
                "--locked",
                "--",
                "-D",
                "warnings",
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
    assert!(plan.iter().any(|spec| {
        spec.args
            == [
                "outdated",
                "--manifest-path",
                repo_root
                    .path()
                    .join("fuzz")
                    .join("Cargo.toml")
                    .to_string_lossy()
                    .as_ref(),
                "--root-deps-only",
                "--exit-code",
                "1",
            ]
    }));
    assert!(
        plan.iter()
            .any(|spec| spec.args == ["audit", "-D", "warnings"])
    );
    assert!(plan.iter().any(|spec| {
        spec.args
            == [
                "audit",
                "-D",
                "warnings",
                "--file",
                repo_root
                    .path()
                    .join("fuzz")
                    .join("Cargo.lock")
                    .to_string_lossy()
                    .as_ref(),
            ]
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
                "--manifest-path",
                repo_root
                    .path()
                    .join("fuzz")
                    .join("Cargo.toml")
                    .to_string_lossy()
                    .as_ref(),
                "--bins",
                "--locked",
            ]
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
fn check_plan_skips_shell_gates_when_no_scripts_exist() {
    let repo_root = tempdir().expect("tempdir");
    write_repo_scaffold(repo_root.path());

    let plan = check_plan(repo_root.path()).expect("check plan");

    assert_eq!(
        plan[0],
        CommandSpec::new("cargo", ["fmt", "--check"], false, false)
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
            "--workspace".to_owned(),
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
