use super::*;

#[test]
fn mutation_diff_staging_covers_relative_absolute_and_failure_paths() {
    let repo_root = tempdir().expect("repo root");
    let relative_diff = Path::new("changes.diff");
    fs::write(repo_root.path().join(relative_diff), "diff evidence").expect("write diff");

    assert_eq!(
        read_mutation_diff(repo_root.path(), None).expect("no diff"),
        None
    );
    assert_eq!(
        read_mutation_diff(repo_root.path(), Some(relative_diff)).expect("relative diff"),
        Some(b"diff evidence".to_vec())
    );
    assert_eq!(
        read_mutation_diff(
            repo_root.path(),
            Some(&repo_root.path().join(relative_diff)),
        )
        .expect("absolute diff"),
        Some(b"diff evidence".to_vec())
    );
    assert!(
        read_mutation_diff(repo_root.path(), Some(Path::new("missing.diff")))
            .expect_err("missing diff")
            .to_string()
            .contains("failed to read mutation diff")
    );

    let output_dir = repo_root.path().join("mutation-runs");
    fs::create_dir_all(&output_dir).expect("create output dir");
    assert_eq!(
        stage_mutation_diff(&output_dir, None).expect("no staged diff"),
        None
    );
    let staged = stage_mutation_diff(&output_dir, Some(b"staged evidence"))
        .expect("stage diff")
        .expect("staged path");
    assert_eq!(
        fs::read_to_string(&staged).expect("read staged diff"),
        "staged evidence"
    );
    remove_staged_mutation_diff(Some(&staged)).expect("remove staged diff");
    remove_staged_mutation_diff(None).expect("remove no diff");

    assert!(
        stage_mutation_diff(&repo_root.path().join("missing-parent"), Some(b"diff"))
            .expect_err("missing stage parent")
            .to_string()
            .contains("failed to stage mutation diff")
    );
    assert!(
        remove_staged_mutation_diff(Some(&staged))
            .expect_err("missing staged file")
            .to_string()
            .contains("failed to remove staged mutation diff")
    );
}
