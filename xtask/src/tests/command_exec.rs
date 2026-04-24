use super::*;

#[test]
fn run_spec_executes_successfully_with_and_without_clang_override() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");

    run_spec(
        repo_root,
        &CommandSpec::new("cargo", ["--version"], true, false),
    )
    .expect("plain command");
    run_spec(
        repo_root,
        &CommandSpec::new("cargo", ["--version"], true, true),
    )
    .expect("clang-forced command");
    run_spec(
        repo_root,
        &CommandSpec::new("cargo", ["--version"], false, false),
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
        &CommandSpec::new(
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
        &CommandSpec::new("cargo", ["--version"], false, true),
    )
    .expect("stdout");
    assert!(String::from_utf8(output).expect("utf8").contains("cargo"));

    let error = capture_command_output(
        repo_root,
        &CommandSpec::new(
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
