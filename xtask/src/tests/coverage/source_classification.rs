use super::*;

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

    let macro_invocation = core_src.join("macro_invocation.rs");
    fs::write(&macro_invocation, "tracked_tokens!();\n").expect("write macro invocation");
    assert_eq!(
        crate::coverage::coverage_source_kind_for_tests(&macro_invocation)
            .expect("macro invocation kind"),
        CoverageSourceKind::DeclarativeOnly
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
