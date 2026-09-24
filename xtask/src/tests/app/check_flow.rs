use super::*;

#[test]
fn main_entry_with_runs_the_full_check_flow_and_cleans_semver_scratch() {
    let repo_root = tempdir().expect("repo tempdir");
    with_isolated_target_dir(repo_root.path(), || {
        write_repo_scaffold(repo_root.path());
        write_toolchain_contract(repo_root.path());
        let tracked_file = write_tracked_source(repo_root.path(), "xtask/src/runner.rs");
        add_xtask_source_rule_to_repo_scaffold(repo_root.path());
        let semver_scratch = semver_scratch_dir(repo_root.path());
        fs::create_dir_all(semver_scratch.join("before")).expect("create initial semver scratch");

        let calls = Rc::new(RefCell::new(Vec::new()));
        let calls_for_override = Rc::clone(&calls);

        with_ready_preflight(|| {
            crate::command_exec::with_run_spec_override(
                move |current_root, spec| {
                    calls_for_override.borrow_mut().push(spec.clone());
                    if is_semver_check_spec(spec) {
                        fs::create_dir_all(semver_scratch_dir(current_root).join("during"))
                            .expect("recreate semver scratch");
                    }
                    if *spec == coverage_report_command(current_root) {
                        write_coverage_report(current_root, &tracked_file, 1, 1, 1, 0);
                    }
                    Some(Ok(()))
                },
                || main_entry_with(repo_root.path(), ["xtask", "check"]),
            )
        })
        .expect("xtask check should pass");

        assert!(!semver_scratch.exists(), "semver scratch should be cleaned");
        assert!(
            calls.borrow().iter().any(is_semver_check_spec),
            "check flow should include the semver step"
        );
        assert!(
            calls
                .borrow()
                .iter()
                .any(|spec| *spec == miri_contract_command()),
            "check flow should include the strict-provenance selector-and-slice Miri proof"
        );
        assert_eq!(
            calls
                .borrow()
                .iter()
                .filter(|spec| **spec == coverage_clean_command())
                .count(),
            2,
            "coverage cleanup should run before and after measurement"
        );
    });
}

#[test]
fn main_entry_with_runs_only_the_semver_step_for_semver_check() {
    let repo_root = tempdir().expect("repo tempdir");
    with_isolated_target_dir(repo_root.path(), || {
        write_repo_scaffold(repo_root.path());
        write_toolchain_contract(repo_root.path());
        let semver_scratch = semver_scratch_dir(repo_root.path());
        fs::create_dir_all(semver_scratch.join("before")).expect("create initial semver scratch");

        let calls = Rc::new(RefCell::new(Vec::new()));
        let calls_for_override = Rc::clone(&calls);

        with_ready_preflight(|| {
            crate::command_exec::with_run_spec_override(
                move |current_root, spec| {
                    calls_for_override.borrow_mut().push(spec.clone());
                    if is_semver_check_spec(spec) {
                        fs::create_dir_all(semver_scratch_dir(current_root).join("during"))
                            .expect("recreate semver scratch");
                    }
                    Some(Ok(()))
                },
                || main_entry_with(repo_root.path(), ["xtask", "semver-check"]),
            )
        })
        .expect("xtask semver-check should pass");

        assert!(!semver_scratch.exists(), "semver scratch should be cleaned");
        assert_eq!(
            calls.borrow().len(),
            1,
            "semver-check should run one command"
        );
        assert!(is_semver_check_spec(&calls.borrow()[0]));
    });
}

#[test]
fn main_entry_with_runs_the_ci_rust_gate_without_coverage() {
    let repo_root = tempdir().expect("repo tempdir");
    with_isolated_target_dir(repo_root.path(), || {
        write_repo_scaffold(repo_root.path());
        write_toolchain_contract(repo_root.path());
        let semver_scratch = semver_scratch_dir(repo_root.path());
        fs::create_dir_all(semver_scratch.join("before")).expect("create initial semver scratch");

        let calls = Rc::new(RefCell::new(Vec::new()));
        let calls_for_override = Rc::clone(&calls);

        with_ready_preflight(|| {
            crate::command_exec::with_run_spec_override(
                move |current_root, spec| {
                    calls_for_override.borrow_mut().push(spec.clone());
                    if is_semver_check_spec(spec) {
                        fs::create_dir_all(semver_scratch_dir(current_root).join("during"))
                            .expect("recreate semver scratch");
                    }
                    Some(Ok(()))
                },
                || main_entry_with(repo_root.path(), ["xtask", "ci-rust-gate"]),
            )
        })
        .expect("xtask ci-rust-gate should pass");

        assert!(!semver_scratch.exists(), "semver scratch should be cleaned");
        assert!(calls.borrow().iter().any(is_semver_check_spec));
        assert!(
            calls
                .borrow()
                .iter()
                .all(|spec| *spec != coverage_command(repo_root.path())),
            "ci rust gate should not run coverage"
        );
    });
}

#[test]
fn semver_check_spec_requires_the_semver_gate_step() {
    let error = crate::semver_check_spec_for_tests(Vec::new())
        .expect_err("missing semver step should fail");

    assert!(
        error
            .to_string()
            .contains("semver gate step is missing from cargo xtask check")
    );
}

#[test]
fn main_entry_with_supports_hygiene_report_and_verify() {
    let repo_root = tempdir().expect("repo tempdir");
    let cargo_config_dir = repo_root.path().join(".cargo");
    fs::create_dir_all(&cargo_config_dir).expect("create .cargo dir");
    fs::write(
        cargo_config_dir.join("config.toml"),
        "[build]\ntarget-dir = \".htmlcut-artifacts/target\"\nbuild-dir = \".htmlcut-artifacts/build\"\n",
    )
    .expect("write cargo config");

    crate::plan::with_cargo_artifact_dir_overrides_for_tests(
        repo_root.path().join(".htmlcut-artifacts/target"),
        repo_root.path().join(".htmlcut-artifacts/build"),
        || {
            main_entry_with(repo_root.path(), ["xtask", "hygiene", "report"])
                .expect("text hygiene report");
            main_entry_with(
                repo_root.path(),
                ["xtask", "hygiene", "report", "--format", "json"],
            )
            .expect("json hygiene report");
            main_entry_with(repo_root.path(), ["xtask", "hygiene", "verify"])
                .expect("hygiene verify");
        },
    );
}

#[test]
fn main_entry_with_hygiene_clean_removes_repo_tmp_cargo_roots() {
    let repo_root = tempdir().expect("repo tempdir");
    let tmp_cargo_root = repo_root.path().join("tmp").join("cargo-target-debug");
    fs::create_dir_all(tmp_cargo_root.join("debug")).expect("create repo tmp cargo root");
    fs::write(tmp_cargo_root.join("debug").join("artifact"), "artifact")
        .expect("write repo tmp artifact");

    main_entry_with(repo_root.path(), ["xtask", "hygiene", "clean"]).expect("hygiene clean");

    assert!(
        !tmp_cargo_root.exists(),
        "repo tmp cargo root should be removed"
    );
}

#[test]
fn main_entry_with_reports_coverage_failures_and_runs_cleanup() {
    let repo_root = tempdir().expect("repo tempdir");
    with_isolated_target_dir(repo_root.path(), || {
        let tracked_file = write_tracked_source(repo_root.path(), "xtask/src/uncovered.rs");
        let calls = Rc::new(RefCell::new(Vec::new()));
        let calls_for_override = Rc::clone(&calls);

        let error = with_ready_preflight(|| {
            crate::command_exec::with_run_spec_override(
                move |current_root, spec| {
                    calls_for_override.borrow_mut().push(spec.clone());
                    if *spec == coverage_report_command(current_root) {
                        write_coverage_report(current_root, &tracked_file, 0, 1, 0, 1);
                    }
                    Some(Ok(()))
                },
                || main_entry_with(repo_root.path(), ["xtask", "coverage"]),
            )
        })
        .expect_err("xtask coverage should fail");

        assert!(error.to_string().contains("coverage gate failed"));
        assert_eq!(
            calls
                .borrow()
                .iter()
                .filter(|spec| **spec == coverage_clean_command())
                .count(),
            2,
            "coverage cleanup should run on failure as well"
        );
    });
}

#[test]
fn run_coverage_for_tests_reports_branch_only_failures() {
    let repo_root = tempdir().expect("repo tempdir");
    with_isolated_target_dir(repo_root.path(), || {
        let tracked_file = write_tracked_source(repo_root.path(), "xtask/src/branch_only.rs");

        let error = with_ready_preflight(|| {
            crate::command_exec::with_run_spec_override(
                move |current_root, spec| {
                    if *spec == coverage_report_command(current_root) {
                        write_coverage_report(current_root, &tracked_file, 1, 1, 0, 1);
                    }
                    Some(Ok(()))
                },
                || run_coverage_for_tests(repo_root.path()),
            )
        })
        .expect_err("branch-only coverage drift should fail");

        assert!(error.to_string().contains("coverage gate failed"));
    });
}

#[test]
fn run_coverage_for_tests_reports_line_only_failures() {
    let repo_root = tempdir().expect("repo tempdir");
    with_isolated_target_dir(repo_root.path(), || {
        let tracked_file = write_tracked_source(repo_root.path(), "xtask/src/line_only.rs");

        let error = with_ready_preflight(|| {
            crate::command_exec::with_run_spec_override(
                move |current_root, spec| {
                    if *spec == coverage_report_command(current_root) {
                        write_coverage_report(current_root, &tracked_file, 0, 0, 0, 0);
                    }
                    Some(Ok(()))
                },
                || run_coverage_for_tests(repo_root.path()),
            )
        })
        .expect_err("line-only coverage drift should fail");

        assert!(error.to_string().contains("coverage gate failed"));
    });
}

#[test]
fn run_coverage_for_tests_reports_success_when_every_tracked_counter_is_covered() {
    let repo_root = tempdir().expect("repo tempdir");
    with_isolated_target_dir(repo_root.path(), || {
        let tracked_file = write_tracked_source(repo_root.path(), "xtask/src/covered.rs");

        with_ready_preflight(|| {
            crate::command_exec::with_run_spec_override(
                move |current_root, spec| {
                    if *spec == coverage_report_command(current_root) {
                        write_coverage_report(current_root, &tracked_file, 1, 1, 1, 0);
                    }
                    Some(Ok(()))
                },
                || run_coverage_for_tests(repo_root.path()),
            )
        })
        .expect("fully covered fixture should pass");
    });
}

#[test]
fn coverage_success_is_retained_as_a_gate_report_step() {
    let repo_root = tempdir().expect("repo tempdir");
    with_isolated_target_dir(repo_root.path(), || {
        let tracked_file = write_tracked_source(repo_root.path(), "xtask/src/covered.rs");

        with_ready_preflight(|| {
            crate::command_exec::with_run_spec_override(
                move |current_root, spec| {
                    if *spec == coverage_report_command(current_root) {
                        write_coverage_report(current_root, &tracked_file, 1, 1, 1, 0);
                    }
                    Some(Ok(()))
                },
                || {
                    crate::gate_report::with_gate_report(
                        repo_root.path(),
                        "coverage-success-evidence",
                        crate::gate_report::GateOutputOptions {
                            format: crate::gate_report::GateOutputFormat::Json,
                            verbose: false,
                        },
                        || run_coverage_for_tests(repo_root.path()),
                    )
                },
            )
        })
        .expect("fully covered fixture should retain success evidence");

        let report = retained_gate_report(repo_root.path());
        assert!(
            report["steps"]
                .as_array()
                .is_some_and(|steps| steps.iter().any(|step| {
                    step["label"] == "Rust coverage ledger" && step["outcome"] == "passed"
                })),
            "coverage success must be recorded as an internal gate step: {report}"
        );
    });
}

#[test]
fn coverage_branch_failures_are_retained_as_gate_report_evidence() {
    let repo_root = tempdir().expect("repo tempdir");
    with_isolated_target_dir(repo_root.path(), || {
        let tracked_file = write_tracked_source(repo_root.path(), "xtask/src/branch_only.rs");

        let error = with_ready_preflight(|| {
            crate::command_exec::with_run_spec_override(
                move |current_root, spec| {
                    if *spec == coverage_report_command(current_root) {
                        write_coverage_report(current_root, &tracked_file, 1, 1, 0, 1);
                    }
                    Some(Ok(()))
                },
                || {
                    crate::gate_report::with_gate_report(
                        repo_root.path(),
                        "coverage-failure-evidence",
                        crate::gate_report::GateOutputOptions {
                            format: crate::gate_report::GateOutputFormat::Json,
                            verbose: false,
                        },
                        || run_coverage_for_tests(repo_root.path()),
                    )
                },
            )
        })
        .expect_err("branch-only coverage fixture must fail");
        assert!(error.to_string().contains("coverage gate failed"));

        let report = retained_gate_report(repo_root.path());
        assert!(
            report["steps"]
                .as_array()
                .is_some_and(|steps| steps.iter().any(|step| {
                    step["label"] == "Rust coverage ledger"
                        && step["outcome"] == "failed"
                        && step["failure_tail"]
                            .as_str()
                            .is_some_and(|detail| detail.contains("branches: 1 uncovered"))
                })),
            "branch-only coverage failure must retain its exact diagnostic: {report}"
        );
    });
}

fn retained_gate_report(repo_root: &Path) -> serde_json::Value {
    let reports = fs::read_dir(crate::gate_report_dir(repo_root))
        .expect("read gate-report root")
        .filter_map(Result::ok)
        .map(|entry| entry.path().join("report.json"))
        .filter(|path| path.is_file())
        .collect::<Vec<_>>();
    assert_eq!(reports.len(), 1, "expected one retained gate report");
    serde_json::from_slice(&fs::read(&reports[0]).expect("read retained gate report"))
        .expect("parse retained gate report")
}
