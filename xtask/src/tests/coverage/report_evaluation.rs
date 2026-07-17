use super::*;

#[test]
fn read_coverage_report_loads_json_from_disk() {
    let repo_root = tempdir().expect("tempdir");
    let coverage_path = repo_root.path().join("coverage.json");
    fs::write(
            &coverage_path,
            r#"{"data":[{"files":[{"filename":"tracked.rs","segments":[[7,0,1,false,true,false]],"summary":{"branches":{"count":1,"covered":1,"notcovered":0}}}]}]}"#,
        )
        .expect("write coverage report");

    let report = read_coverage_report(&coverage_path).expect("read coverage report");

    assert_eq!(report.data.len(), 1);
    assert_eq!(report.data[0].files.len(), 1);
    assert_eq!(
        report.data[0].files[0].filename,
        PathBuf::from("tracked.rs")
    );
}

#[test]
fn evaluate_coverage_report_merges_duplicate_segments_and_ignores_untracked_files() {
    let repo_root = tempdir().expect("tempdir");
    let tracked = tracked_subset(
        repo_root.path(),
        &[
            "crates/htmlcut-core/src/lib.rs",
            "crates/htmlcut-cli/src/lib.rs",
            "crates/htmlcut-cli/src/main.rs",
            "xtask/src/lib.rs",
        ],
    );
    let extra_file = repo_root.path().join("notes.txt");
    fs::write(&extra_file, "ignore").expect("write extra file");

    let report = CoverageReport {
        data: vec![
            CoverageDataSet {
                files: vec![
                    CoverageFile {
                        filename: repo_root.path().join("crates/htmlcut-core/src/lib.rs"),
                        segments: vec![
                            (10, 0, 0, false, true, false),
                            (11, 0, 0, false, false, false),
                        ],
                        branches: Vec::new(),
                        summary: CoverageFileSummary {
                            branches: CoverageCounter {
                                count: 1,
                                covered: 1,
                                not_covered: 0,
                            },
                        },
                    },
                    CoverageFile {
                        filename: extra_file,
                        segments: vec![(99, 0, 1, false, true, false)],
                        branches: Vec::new(),
                        summary: CoverageFileSummary::default(),
                    },
                ],
            },
            CoverageDataSet {
                files: vec![
                    CoverageFile {
                        filename: repo_root.path().join("crates/htmlcut-core/src/lib.rs"),
                        segments: vec![(10, 0, 2, false, true, false)],
                        branches: Vec::new(),
                        summary: CoverageFileSummary {
                            branches: CoverageCounter {
                                count: 1,
                                covered: 1,
                                not_covered: 0,
                            },
                        },
                    },
                    CoverageFile {
                        filename: repo_root.path().join("crates/htmlcut-cli/src/lib.rs"),
                        segments: vec![(20, 0, 1, false, true, false)],
                        branches: Vec::new(),
                        summary: CoverageFileSummary {
                            branches: CoverageCounter {
                                count: 2,
                                covered: 2,
                                not_covered: 0,
                            },
                        },
                    },
                    CoverageFile {
                        filename: repo_root.path().join("crates/htmlcut-cli/src/main.rs"),
                        segments: vec![(30, 0, 1, false, true, false)],
                        branches: Vec::new(),
                        summary: CoverageFileSummary::default(),
                    },
                    CoverageFile {
                        filename: repo_root.path().join("xtask/src/lib.rs"),
                        segments: vec![(40, 0, 1, false, true, false)],
                        branches: Vec::new(),
                        summary: CoverageFileSummary {
                            branches: CoverageCounter {
                                count: 3,
                                covered: 3,
                                not_covered: 0,
                            },
                        },
                    },
                ],
            },
        ],
    };

    let summary =
        evaluate_coverage_report(repo_root.path(), &tracked, report).expect("coverage summary");

    assert_eq!(summary.tracked_line_count, 4);
    assert_eq!(summary.tracked_branch_count, 6);
    assert!(summary.failures.is_empty());
}

#[test]
fn evaluate_coverage_report_deduplicates_duplicate_branch_spans() {
    let repo_root = tempdir().expect("tempdir");
    let tracked = tracked_subset(
        repo_root.path(),
        &[
            "crates/htmlcut-core/src/lib.rs",
            "crates/htmlcut-cli/src/lib.rs",
            "crates/htmlcut-cli/src/main.rs",
            "xtask/src/lib.rs",
        ],
    );

    let report = CoverageReport {
        data: vec![CoverageDataSet {
            files: vec![
                CoverageFile {
                    filename: repo_root.path().join("crates/htmlcut-core/src/lib.rs"),
                    segments: vec![(7, 0, 1, false, true, false)],
                    branches: Vec::new(),
                    summary: CoverageFileSummary::default(),
                },
                CoverageFile {
                    filename: repo_root.path().join("crates/htmlcut-cli/src/lib.rs"),
                    segments: vec![(9, 0, 1, false, true, false)],
                    branches: vec![
                        (12, 0, 12, 24, 0, 0, 0, 0, 4),
                        (12, 0, 12, 24, 3, 2, 0, 0, 4),
                    ],
                    summary: CoverageFileSummary {
                        branches: CoverageCounter {
                            count: 2,
                            covered: 0,
                            not_covered: 2,
                        },
                    },
                },
                CoverageFile {
                    filename: repo_root.path().join("crates/htmlcut-cli/src/main.rs"),
                    segments: vec![(11, 0, 1, false, true, false)],
                    branches: Vec::new(),
                    summary: CoverageFileSummary::default(),
                },
                CoverageFile {
                    filename: repo_root.path().join("xtask/src/lib.rs"),
                    segments: vec![(13, 0, 1, false, true, false)],
                    branches: Vec::new(),
                    summary: CoverageFileSummary::default(),
                },
            ],
        }],
    };

    let summary =
        evaluate_coverage_report(repo_root.path(), &tracked, report).expect("coverage summary");

    assert_eq!(summary.tracked_line_count, 4);
    assert_eq!(summary.tracked_branch_count, 2);
    assert!(summary.failures.is_empty());
}

#[test]
fn evaluate_coverage_report_reports_uncovered_and_missing_files() {
    let repo_root = tempdir().expect("tempdir");
    let tracked = seed_tracked_files(repo_root.path());

    let report = CoverageReport {
        data: vec![CoverageDataSet {
            files: vec![CoverageFile {
                filename: repo_root
                    .path()
                    .join("crates/htmlcut-core/src/contracts/mod.rs"),
                segments: vec![(7, 0, 0, false, true, false)],
                branches: Vec::new(),
                summary: CoverageFileSummary {
                    branches: CoverageCounter {
                        count: 2,
                        covered: 1,
                        not_covered: 1,
                    },
                },
            }],
        }],
    };

    let summary =
        evaluate_coverage_report(repo_root.path(), &tracked, report).expect("coverage summary");

    assert_eq!(summary.tracked_line_count, 1);
    assert_eq!(summary.tracked_branch_count, 2);
    let core_failure = summary
        .failures
        .iter()
        .find(|failure| failure.file == "crates/htmlcut-core/src/contracts/mod.rs")
        .expect("core failure");
    assert_eq!(core_failure.uncovered_lines, vec!["7".to_owned()]);
    assert_eq!(core_failure.uncovered_branch_count, 1);
    assert!(
        summary.failures.iter().any(
            |failure| failure.uncovered_lines == vec!["<no executable lines found>".to_owned()]
        )
    );
}

#[test]
fn tracked_files_classify_declarative_only_sources_without_path_special_cases() {
    let repo_root = tempdir().expect("tempdir");
    let text_dir = repo_root
        .path()
        .join("crates/htmlcut-core/src/document/text");
    fs::create_dir_all(&text_dir).expect("create text dir");
    fs::write(
        text_dir.join("mod.rs"),
        "mod render;\npub(crate) use render::render_document_body_as_text;\n",
    )
    .expect("write text mod");
    fs::write(
        text_dir.join("vocabulary.rs"),
        "pub(super) const TOKENS: [&str; 2] = [\"content\", \"main\"];\n",
    )
    .expect("write vocabulary file");
    fs::write(
        text_dir.join("render.rs"),
        "pub(crate) fn render_document_body_as_text() -> &'static str {\n    \"ok\"\n}\n",
    )
    .expect("write render file");

    let tracked = tracked_files(repo_root.path()).expect("tracked files");

    assert_eq!(
        tracked
            .get(&normalize_path(repo_root.path(), &text_dir.join("mod.rs")).expect("path"))
            .expect("tracked mod file")
            .kind,
        CoverageSourceKind::DeclarativeOnly
    );
    assert_eq!(
        tracked
            .get(&normalize_path(repo_root.path(), &text_dir.join("vocabulary.rs")).expect("path"))
            .expect("tracked vocabulary file")
            .kind,
        CoverageSourceKind::DeclarativeOnly
    );
    assert_eq!(
        tracked
            .get(&normalize_path(repo_root.path(), &text_dir.join("render.rs")).expect("path"))
            .expect("tracked render file")
            .kind,
        CoverageSourceKind::Executable
    );
}
