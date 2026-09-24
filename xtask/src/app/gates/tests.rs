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

#[test]
fn local_mutation_summary_uses_timeout_precedence_and_reports_exact_totals() {
    let missed_only = LocalMutationSummary {
        total_mutants: 1,
        caught: 0,
        missed: 1,
        timed_out: 0,
        unviable: 0,
    };
    let missed_error = ensure_local_mutation_summary_is_clean(missed_only)
        .expect_err("missed mutant must fail the gate")
        .to_string();
    assert!(missed_error.contains("missed mutants: 1 missed, 0 timed out"));

    let timeout_with_miss = LocalMutationSummary {
        total_mutants: 2,
        caught: 0,
        missed: 1,
        timed_out: 1,
        unviable: 0,
    };
    let timeout_error = ensure_local_mutation_summary_is_clean(timeout_with_miss)
        .expect_err("timed-out mutant must take precedence")
        .to_string();
    assert!(timeout_error.contains("timed-out mutants: 1 missed, 1 timed out"));
}

#[test]
fn coverage_failure_rendering_retains_lines_and_branches_without_empty_sections() {
    let rendered = render_coverage_failures(&[
        CoverageFailure {
            file: "crates/htmlcut-core/src/lib.rs".to_owned(),
            uncovered_lines: vec!["12".to_owned(), "18".to_owned()],
            uncovered_branch_count: 0,
        },
        CoverageFailure {
            file: "xtask/src/lib.rs".to_owned(),
            uncovered_lines: Vec::new(),
            uncovered_branch_count: 2,
        },
    ]);

    assert_eq!(
        rendered,
        "crates/htmlcut-core/src/lib.rs lines: 12, 18\nxtask/src/lib.rs branches: 2 uncovered"
    );
}
