use super::*;

#[test]
fn main_entry_with_runs_the_structure_report_and_check_commands() {
    let repo_root = tempdir().expect("repository fixture");
    write_repo_scaffold(repo_root.path());

    main_entry_with(repo_root.path(), ["xtask", "structure", "check"])
        .expect("structure check succeeds");
    main_entry_with(repo_root.path(), ["xtask", "structure", "report"])
        .expect("structure report succeeds");
}

#[test]
fn structure_check_records_a_failed_internal_step_when_gate_reporting_is_active() {
    let repo_root = tempdir().expect("repository fixture");
    write_repo_scaffold(repo_root.path());
    let unowned_source = repo_root.path().join("crates/htmlcut-core/src/unowned.rs");
    fs::create_dir_all(unowned_source.parent().expect("source parent")).expect("create source dir");
    fs::write(&unowned_source, "pub(crate) fn unowned() {}\n").expect("write unowned source");

    let error = main_entry_with(
        repo_root.path(),
        ["xtask", "structure", "check", "--format", "json"],
    )
    .expect_err("unowned source must fail structure gate");
    assert!(
        error
            .to_string()
            .contains("source-structure contract failed")
    );
}
