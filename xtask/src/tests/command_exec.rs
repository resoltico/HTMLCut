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
