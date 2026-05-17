use super::*;

#[cfg(unix)]
fn symlink_file(source: &Path, link: &Path) {
    std::os::unix::fs::symlink(source, link).expect("create symlink");
}

#[test]
fn tracked_files_skip_missing_roots_non_rust_entries_and_explicit_exclusions() {
    let repo_root = tempdir().expect("tempdir");
    let cli_src = repo_root.path().join("crates/htmlcut-cli/src");
    fs::create_dir_all(cli_src.join("nested")).expect("create nested cli src");
    fs::create_dir_all(cli_src.join("tests")).expect("create cli test dir");
    fs::create_dir_all(cli_src.join("model")).expect("create cli model dir");

    fs::write(cli_src.join("lookup.rs"), "// tracked\n").expect("write lookup");
    fs::write(cli_src.join("nested/report.rs"), "// tracked\n").expect("write nested report");
    fs::write(cli_src.join("main.rs"), "// skipped main\n").expect("write main");
    fs::write(cli_src.join("notes.txt"), "ignore").expect("write note");
    fs::write(cli_src.join("tests/helper.rs"), "// skipped test module")
        .expect("write test helper");
    fs::write(cli_src.join("model/catalog.rs"), "// skipped declarative")
        .expect("write excluded catalog model");

    let tracked = tracked_files(repo_root.path()).expect("tracked files");
    let tracked_paths = tracked
        .values()
        .map(|tracked_file| tracked_file.display_path.clone())
        .collect::<Vec<_>>();

    assert!(tracked_paths.contains(&"crates/htmlcut-cli/src/lookup.rs".to_owned()));
    assert!(tracked_paths.contains(&"crates/htmlcut-cli/src/nested/report.rs".to_owned()));
    assert!(!tracked_paths.contains(&"crates/htmlcut-cli/src/main.rs".to_owned()));
    assert!(!tracked_paths.contains(&"crates/htmlcut-cli/src/tests/helper.rs".to_owned()));
    assert!(!tracked_paths.contains(&"crates/htmlcut-cli/src/model/catalog.rs".to_owned()));
}

#[test]
fn tracked_files_use_git_inventory_when_available() {
    let repo_root = tempdir().expect("tempdir");
    fs::write(repo_root.path().join(".git"), "gitdir: /tmp/htmlcut.git\n").expect("write .git");
    let tracked_file = repo_root.path().join("crates/htmlcut-cli/src/execute.rs");
    let ignored_local_file = repo_root
        .path()
        .join("crates/htmlcut-cli/src/local_only.rs");
    let skipped_main = repo_root.path().join("xtask/src/main.rs");
    let outside_root_rs = repo_root.path().join("scripts/helper.rs");
    let non_rust_file = repo_root.path().join("Cargo.toml");
    fs::create_dir_all(tracked_file.parent().expect("parent")).expect("create tracked parent");
    fs::write(&tracked_file, "// tracked\n").expect("write tracked file");
    fs::write(&ignored_local_file, "// local only\n").expect("write local-only file");
    fs::create_dir_all(skipped_main.parent().expect("parent")).expect("create xtask parent");
    fs::write(&skipped_main, "// main\n").expect("write skipped main");
    fs::create_dir_all(outside_root_rs.parent().expect("parent")).expect("create scripts dir");
    fs::write(&outside_root_rs, "// helper\n").expect("write outside-root helper");
    fs::write(&non_rust_file, "[workspace]\n").expect("write Cargo.toml");

    let tracked = crate::command_exec::with_capture_command_output_override(
        |_, spec| {
            (spec.program == Path::new("git"))
                .then(|| {
                    Ok(
                        b"Cargo.toml\0crates/htmlcut-cli/src/execute.rs\0scripts/helper.rs\0xtask/src/main.rs\0".to_vec(),
                    )
                })
        },
        || tracked_files(repo_root.path()),
    )
    .expect("tracked files");
    let tracked_paths = tracked
        .values()
        .map(|tracked_file| tracked_file.display_path.clone())
        .collect::<Vec<_>>();

    assert_eq!(
        tracked_paths,
        vec!["crates/htmlcut-cli/src/execute.rs".to_owned()]
    );
    assert!(crate::coverage::is_under_coverage_root_for_tests(
        "xtask/src"
    ));
    assert!(crate::coverage::is_under_coverage_root_for_tests(
        "xtask/src/lib.rs"
    ));
    assert!(!crate::coverage::is_under_coverage_root_for_tests(
        "scripts/helper.rs"
    ));
}

#[cfg(unix)]
#[test]
fn tracked_files_reject_entries_that_resolve_outside_the_repo_root() {
    let repo_root = tempdir().expect("repo tempdir");
    let outside_root = tempdir().expect("outside tempdir");
    let cli_src = repo_root.path().join("crates/htmlcut-cli/src");
    fs::create_dir_all(&cli_src).expect("create cli src");
    let outside_file = outside_root.path().join("escaped.rs");
    fs::write(&outside_file, "// outside\n").expect("write outside file");
    symlink_file(&outside_file, &cli_src.join("escaped.rs"));

    let error = tracked_files(repo_root.path()).expect_err("symlink should escape the repo");

    assert!(error.to_string().contains("does not live under repo root"));
}

#[test]
fn repo_relative_source_path_rejects_paths_outside_the_repo_root() {
    let repo_root = tempdir().expect("repo tempdir");
    let outside_root = tempdir().expect("outside tempdir");
    let outside_file = outside_root.path().join("lookup.rs");
    fs::write(&outside_file, "// outside\n").expect("write outside file");
    let absolute_outside_file =
        normalize_path(repo_root.path(), &outside_file).expect("normalize outside file");

    let error = crate::coverage::repo_relative_source_path_for_tests(
        repo_root.path(),
        &absolute_outside_file,
    )
    .expect_err("outside paths should fail");

    assert!(error.to_string().contains("does not live under repo root"));
}

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

#[test]
fn declarative_only_trait_signatures_are_not_scored_but_default_method_bodies_are() {
    let repo_root = tempdir().expect("tempdir");
    let core_src = repo_root.path().join("crates/htmlcut-core/src");
    fs::create_dir_all(&core_src).expect("create core src");
    let signature_only = core_src.join("trait_signature.rs");
    let default_body = core_src.join("trait_default.rs");
    fs::write(
        &signature_only,
        "pub trait RenderPolicy {\n    fn title(&self) -> &'static str;\n}\n",
    )
    .expect("write trait signature file");
    fs::write(
        &default_body,
        "pub trait RenderPolicy {\n    fn title(&self) -> &'static str {\n        \"ok\"\n    }\n}\n",
    )
    .expect("write trait default file");

    assert_eq!(
        crate::coverage::coverage_source_kind_for_tests(&signature_only)
            .expect("trait signature kind"),
        CoverageSourceKind::DeclarativeOnly
    );
    assert_eq!(
        crate::coverage::coverage_source_kind_for_tests(&default_body).expect("trait default kind"),
        CoverageSourceKind::Executable
    );
}

#[test]
fn coverage_source_classifier_covers_invalid_syntax_inline_modules_impls_and_macros() {
    let repo_root = tempdir().expect("tempdir");
    let core_src = repo_root.path().join("crates/htmlcut-core/src");
    fs::create_dir_all(&core_src).expect("create core src");

    let invalid_source = core_src.join("invalid.rs");
    fs::write(&invalid_source, "pub fn broken( {\n").expect("write invalid source");
    let invalid_error = crate::coverage::coverage_source_kind_for_tests(&invalid_source)
        .expect_err("invalid Rust source should fail classification");
    assert!(invalid_error.to_string().contains("invalid Rust source"));
    assert!(
        invalid_error
            .to_string()
            .contains(&invalid_source.display().to_string())
    );

    let declarative_inline_mod = core_src.join("inline_declarative.rs");
    fs::write(
        &declarative_inline_mod,
        "mod nested {\n    pub(super) const TOKENS: [&str; 1] = [\"content\"];\n}\n",
    )
    .expect("write declarative inline module");
    assert_eq!(
        crate::coverage::coverage_source_kind_for_tests(&declarative_inline_mod)
            .expect("declarative inline module kind"),
        CoverageSourceKind::DeclarativeOnly
    );

    let executable_inline_mod = core_src.join("inline_executable.rs");
    fs::write(
        &executable_inline_mod,
        "mod nested {\n    pub(super) fn render() -> &'static str {\n        \"ok\"\n    }\n}\n",
    )
    .expect("write executable inline module");
    assert_eq!(
        crate::coverage::coverage_source_kind_for_tests(&executable_inline_mod)
            .expect("executable inline module kind"),
        CoverageSourceKind::Executable
    );

    let declarative_impl = core_src.join("impl_declarative.rs");
    fs::write(
        &declarative_impl,
        "pub trait Shape {\n    const SIDES: usize;\n    type Output;\n}\npub struct Square;\nimpl Shape for Square {\n    const SIDES: usize = 4;\n    type Output = Square;\n}\n",
    )
    .expect("write declarative impl");
    assert_eq!(
        crate::coverage::coverage_source_kind_for_tests(&declarative_impl)
            .expect("declarative impl kind"),
        CoverageSourceKind::DeclarativeOnly
    );

    let executable_impl = core_src.join("impl_executable.rs");
    fs::write(
        &executable_impl,
        "pub struct Square;\nimpl Square {\n    pub fn area() -> usize {\n        4\n    }\n}\n",
    )
    .expect("write executable impl");
    assert_eq!(
        crate::coverage::coverage_source_kind_for_tests(&executable_impl)
            .expect("executable impl kind"),
        CoverageSourceKind::Executable
    );

    let declarative_macro = core_src.join("macro_definition.rs");
    fs::write(
        &declarative_macro,
        "macro_rules! tracked_tokens {\n    () => { \"ok\" };\n}\n",
    )
    .expect("write declarative macro");
    assert_eq!(
        crate::coverage::coverage_source_kind_for_tests(&declarative_macro)
            .expect("declarative macro kind"),
        CoverageSourceKind::DeclarativeOnly
    );

    let executable_macro = core_src.join("macro_invocation.rs");
    fs::write(&executable_macro, "tracked_tokens!();\n").expect("write macro invocation");
    assert_eq!(
        crate::coverage::coverage_source_kind_for_tests(&executable_macro)
            .expect("executable macro kind"),
        CoverageSourceKind::Executable
    );
}

#[test]
fn evaluate_coverage_report_accepts_declarative_only_tracked_files_without_segments() {
    let repo_root = tempdir().expect("tempdir");
    let contracts_path = repo_root
        .path()
        .join("crates/htmlcut-core/src/contracts/mod.rs");
    let catalog_path = repo_root.path().join("crates/htmlcut-core/src/catalog.rs");
    fs::create_dir_all(contracts_path.parent().expect("contracts parent"))
        .expect("create contracts parent");
    fs::write(&contracts_path, "mod request;\n").expect("write declarative contracts file");
    fs::write(
        &catalog_path,
        "pub(crate) fn tracked() -> usize {\n    1\n}\n",
    )
    .expect("write executable catalog file");
    let tracked = BTreeMap::from([
        (
            normalize_path(repo_root.path(), &contracts_path).expect("contracts path"),
            TrackedCoverageFile::declarative_only("crates/htmlcut-core/src/contracts/mod.rs"),
        ),
        (
            normalize_path(repo_root.path(), &catalog_path).expect("catalog path"),
            TrackedCoverageFile::executable("crates/htmlcut-core/src/catalog.rs"),
        ),
    ]);

    let report = CoverageReport {
        data: vec![CoverageDataSet {
            files: vec![CoverageFile {
                filename: catalog_path,
                segments: vec![(7, 0, 1, false, true, false)],
                branches: Vec::new(),
                summary: CoverageFileSummary::default(),
            }],
        }],
    };

    let summary =
        evaluate_coverage_report(repo_root.path(), &tracked, report).expect("coverage summary");

    assert_eq!(summary.tracked_line_count, 1);
    assert!(summary.failures.is_empty());
}

#[test]
fn evaluate_coverage_report_reports_branch_only_failures() {
    let repo_root = tempdir().expect("tempdir");
    let tracked = seed_tracked_files(repo_root.path());

    let report = CoverageReport {
        data: vec![CoverageDataSet {
            files: vec![
                CoverageFile {
                    filename: repo_root
                        .path()
                        .join("crates/htmlcut-core/src/contracts/mod.rs"),
                    segments: vec![(7, 0, 1, false, true, false)],
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
                    filename: repo_root.path().join("crates/htmlcut-cli/src/execute.rs"),
                    segments: vec![(9, 0, 1, false, true, false)],
                    branches: Vec::new(),
                    summary: CoverageFileSummary {
                        branches: CoverageCounter {
                            count: 3,
                            covered: 2,
                            not_covered: 1,
                        },
                    },
                },
                CoverageFile {
                    filename: repo_root.path().join("xtask/src/plan.rs"),
                    segments: vec![(11, 0, 1, false, true, false)],
                    branches: Vec::new(),
                    summary: CoverageFileSummary {
                        branches: CoverageCounter {
                            count: 1,
                            covered: 1,
                            not_covered: 0,
                        },
                    },
                },
            ],
        }],
    };

    let summary =
        evaluate_coverage_report(repo_root.path(), &tracked, report).expect("coverage summary");

    let cli_failure = summary
        .failures
        .iter()
        .find(|failure| failure.file == "crates/htmlcut-cli/src/execute.rs")
        .expect("cli branch-only failure");
    assert!(cli_failure.uncovered_lines.is_empty());
    assert_eq!(cli_failure.uncovered_branch_count, 1);
}

#[test]
fn coverage_output_helpers_create_directories_for_default_relative_and_absolute_targets() {
    let repo_root = tempdir().expect("tempdir");

    ensure_coverage_output_dir(repo_root.path()).expect("default coverage dir");
    let live_output_path = coverage_output_path(repo_root.path());
    assert!(
        live_output_path
            .parent()
            .expect("coverage output parent")
            .is_dir()
    );
    let absolute_target = repo_root.path().join("absolute-target");
    assert_eq!(
        crate::coverage::coverage_target_dir_for_tests(
            repo_root.path(),
            Some(Path::new("custom-target"))
        ),
        repo_root.path().join("coverage-target")
    );
    assert_eq!(
        crate::coverage::coverage_target_dir_for_tests(repo_root.path(), Some(&absolute_target)),
        repo_root.path().join("coverage-target")
    );
    assert_eq!(
        crate::coverage::coverage_target_dir_for_tests(repo_root.path(), None),
        repo_root.path().join("coverage-target")
    );
}

#[cfg(windows)]
#[test]
fn binary_name_matches_the_current_platform() {
    assert_eq!(binary_name(), "htmlcut.exe");
}

#[cfg(not(windows))]
#[test]
fn binary_name_matches_the_current_platform() {
    assert_eq!(binary_name(), "htmlcut");
}
