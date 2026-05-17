use super::*;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[test]
fn run_spec_executes_successfully_with_and_without_clang_override() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");

    run_spec(
        repo_root,
        &test_command_spec("cargo", ["--version"], true, false),
    )
    .expect("plain command");
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
}

#[test]
fn run_spec_reports_non_zero_status() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");
    let error = run_spec(
        repo_root,
        &test_command_spec(
            "cargo",
            ["__definitely_not_a_real_subcommand__"],
            true,
            false,
        ),
    )
    .expect_err("failing command");

    assert!(error.to_string().contains("command failed with status"));
}

#[test]
fn capture_command_output_returns_stdout_and_reports_failure() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");

    let output = capture_command_output(
        repo_root,
        &test_command_spec("cargo", ["--version"], false, true),
    )
    .expect("stdout");
    assert!(String::from_utf8(output).expect("utf8").contains("cargo"));

    let error = capture_command_output(
        repo_root,
        &test_command_spec(
            "cargo",
            ["__definitely_not_a_real_subcommand__"],
            false,
            false,
        ),
    )
    .expect_err("failing command");
    assert!(error.to_string().contains("command failed with status"));
}

#[test]
fn command_environment_for_tests_applies_explicit_overrides() {
    let spec = test_command_spec("cargo", ["--version"], true, false)
        .with_env("MIRIFLAGS", "-Zmiri-strict-provenance");

    let env_pairs = crate::command_exec::command_environment_for_tests(&spec);
    let miriflags = env_pairs
        .into_iter()
        .find(|(key, _)| key == "MIRIFLAGS")
        .and_then(|(_, value)| value)
        .expect("MIRIFLAGS override");
    assert_eq!(miriflags, "-Zmiri-strict-provenance");
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

#[test]
fn detached_launcher_is_required_when_xtask_runs_from_the_managed_target_root() {
    let repo_root = tempdir().expect("repo tempdir");
    fs::create_dir_all(repo_root.path().join(".cargo")).expect("create .cargo dir");
    fs::write(
        repo_root.path().join(".cargo").join("config.toml"),
        "[build]\ntarget-dir = \"../.htmlcut-artifacts/target\"\nbuild-dir = \"../.htmlcut-artifacts/build\"\n",
    )
    .expect("write cargo config");
    let target_executable = repo_root
        .path()
        .join("../.htmlcut-artifacts/target/debug/xtask");
    let build_executable = repo_root
        .path()
        .join("../.htmlcut-artifacts/build/debug/xtask");
    let detached_executable = repo_root.path().join("tmp/detached/xtask");

    assert!(crate::command_exec::launcher_requires_detach_for_tests(
        repo_root.path(),
        &target_executable
    ));
    assert!(crate::command_exec::launcher_requires_detach_for_tests(
        repo_root.path(),
        &build_executable
    ));
    assert!(!crate::command_exec::launcher_requires_detach_for_tests(
        repo_root.path(),
        &detached_executable
    ));
}

#[cfg(unix)]
#[test]
fn detached_launcher_reexecutes_from_a_temp_copy_and_marks_the_child_environment() {
    let repo_root = tempdir().expect("repo tempdir");
    fs::create_dir_all(repo_root.path().join(".cargo")).expect("create .cargo dir");
    fs::write(
        repo_root.path().join(".cargo").join("config.toml"),
        "[build]\ntarget-dir = \"../.htmlcut-artifacts/target\"\nbuild-dir = \"../.htmlcut-artifacts/build\"\n",
    )
    .expect("write cargo config");
    let managed_target_root = repo_root.path().join("../.htmlcut-artifacts/target");
    let script_path = managed_target_root.join("debug").join("xtask");
    fs::create_dir_all(script_path.parent().expect("script parent")).expect("create script dir");
    fs::write(
        &script_path,
        format!(
            "#!/usr/bin/env bash\nset -euo pipefail\n[[ \"${{HTMLCUT_XTASK_DETACHED_LAUNCHER:-}}\" == \"1\" ]]\n[[ \"$1\" == \"ci-rust-gate\" ]]\ncase \"$0\" in\n  \"{managed_root}\"/*) exit 42 ;;\nesac\n",
            managed_root = managed_target_root.display(),
        ),
    )
    .expect("write detached launcher script");
    let mut permissions = fs::metadata(&script_path)
        .expect("script metadata")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&script_path, permissions).expect("make script executable");

    let handed_off = crate::command_exec::with_detached_launcher_env_override(false, || {
        crate::command_exec::with_current_executable_override(script_path, || {
            crate::command_exec::run_from_detached_launcher_if_needed(
                repo_root.path(),
                &["xtask".into(), "ci-rust-gate".into()],
            )
        })
    })
    .expect("detached launcher re-exec should succeed");

    assert!(handed_off, "launcher should hand off to the detached copy");
}

#[cfg(unix)]
#[test]
fn detached_launcher_returns_false_when_already_detached_or_outside_artifact_roots() {
    let repo_root = tempdir().expect("repo tempdir");
    fs::create_dir_all(repo_root.path().join(".cargo")).expect("create .cargo dir");
    fs::write(
        repo_root.path().join(".cargo").join("config.toml"),
        "[build]\ntarget-dir = \"../.htmlcut-artifacts/target\"\nbuild-dir = \"../.htmlcut-artifacts/build\"\n",
    )
    .expect("write cargo config");
    let standalone_script = repo_root.path().join("tmp").join("standalone-xtask");
    fs::create_dir_all(standalone_script.parent().expect("script parent"))
        .expect("create script dir");
    fs::write(&standalone_script, "#!/usr/bin/env bash\nexit 0\n").expect("write script");
    let mut permissions = fs::metadata(&standalone_script)
        .expect("script metadata")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&standalone_script, permissions).expect("make script executable");

    let without_detach =
        crate::command_exec::with_current_executable_override(standalone_script.clone(), || {
            crate::command_exec::run_from_detached_launcher_if_needed(
                repo_root.path(),
                &["xtask".into()],
            )
        })
        .expect("non-artifact-root launcher should succeed");
    assert!(!without_detach);

    let already_detached = crate::command_exec::with_detached_launcher_env_override(true, || {
        crate::command_exec::with_current_executable_override(standalone_script, || {
            crate::command_exec::run_from_detached_launcher_if_needed(
                repo_root.path(),
                &["xtask".into()],
            )
        })
    })
    .expect("already detached launcher should succeed");
    assert!(!already_detached);
}

#[test]
fn detached_launcher_returns_false_for_the_real_test_binary_outside_managed_roots() {
    let repo_root = tempdir().expect("repo tempdir");
    fs::create_dir_all(repo_root.path().join(".cargo")).expect("create .cargo dir");
    fs::write(
        repo_root.path().join(".cargo").join("config.toml"),
        "[build]\ntarget-dir = \"../.htmlcut-artifacts/target\"\nbuild-dir = \"../.htmlcut-artifacts/build\"\n",
    )
    .expect("write cargo config");

    let without_detach = crate::command_exec::with_detached_launcher_env_override(false, || {
        crate::command_exec::run_from_detached_launcher_if_needed(
            repo_root.path(),
            &["xtask".into()],
        )
    })
    .expect("real test binary should not require detached relaunch");

    assert!(!without_detach);
}

#[cfg(unix)]
#[test]
fn current_executable_path_uses_the_real_process_when_no_override_is_installed() {
    let current_executable =
        crate::command_exec::current_executable_path_for_tests().expect("current executable");
    assert!(
        current_executable.exists(),
        "current executable path should exist: {}",
        current_executable.display()
    );
}

#[cfg(unix)]
#[test]
fn prepare_detached_executable_copy_clones_the_source_binary_and_permissions() {
    let workspace = tempdir().expect("workspace tempdir");
    let source_path = workspace.path().join("tmp").join("xtask-source");
    fs::create_dir_all(source_path.parent().expect("source parent")).expect("create source dir");
    fs::write(&source_path, "#!/usr/bin/env bash\nexit 0\n").expect("write source script");
    let mut source_permissions = fs::metadata(&source_path)
        .expect("source metadata")
        .permissions();
    source_permissions.set_mode(0o751);
    fs::set_permissions(&source_path, source_permissions).expect("set source permissions");

    let detached_root = workspace.path().join("tmp").join("detached");
    fs::create_dir_all(&detached_root).expect("create detached root");
    let detached_path = crate::command_exec::prepare_detached_executable_copy_for_tests(
        &source_path,
        &detached_root,
    )
    .expect("prepare detached copy");

    assert_eq!(
        fs::read_to_string(&detached_path).expect("read detached copy"),
        "#!/usr/bin/env bash\nexit 0\n"
    );
    assert_eq!(
        detached_path.file_name(),
        source_path.file_name(),
        "detached copy should preserve the executable file name"
    );
    assert_eq!(
        fs::metadata(&detached_path)
            .expect("detached metadata")
            .permissions()
            .mode()
            & 0o777,
        0o751,
        "detached copy should preserve executable permissions"
    );
}

#[cfg(unix)]
#[test]
fn detached_launcher_reports_non_zero_child_exit_statuses() {
    let repo_root = tempdir().expect("repo tempdir");
    fs::create_dir_all(repo_root.path().join(".cargo")).expect("create .cargo dir");
    fs::write(
        repo_root.path().join(".cargo").join("config.toml"),
        "[build]\ntarget-dir = \"../.htmlcut-artifacts/target\"\nbuild-dir = \"../.htmlcut-artifacts/build\"\n",
    )
    .expect("write cargo config");
    let managed_target_root = repo_root.path().join("../.htmlcut-artifacts/target");
    let script_path = managed_target_root.join("debug").join("xtask");
    fs::create_dir_all(script_path.parent().expect("script parent")).expect("create script dir");
    fs::write(&script_path, "#!/usr/bin/env bash\nexit 7\n").expect("write failing script");
    let mut permissions = fs::metadata(&script_path)
        .expect("script metadata")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&script_path, permissions).expect("make script executable");

    let error = crate::command_exec::with_detached_launcher_env_override(false, || {
        crate::command_exec::with_current_executable_override(script_path, || {
            crate::command_exec::run_from_detached_launcher_if_needed(
                repo_root.path(),
                &["xtask".into()],
            )
        })
    })
    .expect_err("failing detached launcher should surface");
    assert!(
        error
            .to_string()
            .contains("detached xtask launcher exited with status"),
        "unexpected error: {error}"
    );
}
