use super::*;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

fn write_toolchain_contract(repo_root: &Path) {
    fs::write(
        repo_root.join("rust-toolchain.toml"),
        "[toolchain]\nchannel = \"stable\"\ncomponents = [\"clippy\", \"rustfmt\"]\n",
    )
    .expect("write rust-toolchain.toml");
}

fn write_tracked_source(repo_root: &Path, relative_path: &str) -> PathBuf {
    let path = repo_root.join(relative_path);
    fs::create_dir_all(path.parent().expect("tracked source parent")).expect("create src dir");
    fs::write(&path, "pub fn covered() {}\n").expect("write tracked source");
    path
}

fn write_coverage_report(
    repo_root: &Path,
    tracked_file: &Path,
    line_count: u64,
    branch_count: u64,
    covered_branches: u64,
    uncovered_branches: u64,
) {
    let coverage_path = coverage_output_path(repo_root);
    fs::create_dir_all(coverage_path.parent().expect("coverage dir")).expect("create coverage dir");
    fs::write(
        coverage_path,
        serde_json::json!({
            "data": [{
                "files": [{
                    "filename": tracked_file,
                    "segments": [[7, 0, line_count, false, true, false]],
                    "branches": [],
                    "summary": {
                        "branches": {
                            "count": branch_count,
                            "covered": covered_branches,
                            "notcovered": uncovered_branches,
                        }
                    }
                }]
            }]
        })
        .to_string(),
    )
    .expect("write coverage report");
}

fn with_ready_preflight<T>(operation: impl FnOnce() -> T) -> T {
    crate::command_exec::with_capture_command_output_override(
        |_, spec| {
            let args = spec.args.iter().map(String::as_str).collect::<Vec<_>>();
            if spec.program == Path::new("rustup") && args == ["toolchain", "list"] {
                return Some(Ok(
                    b"stable-aarch64-apple-darwin (default)\nnightly-aarch64-apple-darwin\n"
                        .to_vec(),
                ));
            }
            if spec.program == Path::new("rustup")
                && args == ["component", "list", "--toolchain", "stable", "--installed"]
            {
                return Some(Ok(
                    b"clippy-aarch64-apple-darwin\nrustfmt-aarch64-apple-darwin\n".to_vec(),
                ));
            }
            if spec.program == Path::new("rustup")
                && args == ["component", "list", "--toolchain", "nightly", "--installed"]
            {
                return Some(Ok(b"llvm-tools-preview-aarch64-apple-darwin\n".to_vec()));
            }
            if spec.program == Path::new("rustup")
                && args == ["run", "stable", "cargo-clippy", "-V"]
            {
                return Some(Ok(b"clippy 0.1.0\n".to_vec()));
            }
            if spec.program == Path::new("rustup")
                && args == ["run", "stable", "rustfmt", "--version"]
            {
                return Some(Ok(b"rustfmt 1.0.0\n".to_vec()));
            }
            if spec.program == Path::new("cargo") && args == ["fuzz", "--help"] {
                return Some(Ok(b"cargo-fuzz 0.12.0\n".to_vec()));
            }
            if (spec.program == Path::new("clang") || spec.program == Path::new("clang++"))
                && args == ["--version"]
            {
                return Some(Ok(b"clang version 18.0.0\n".to_vec()));
            }

            None
        },
        operation,
    )
}

#[test]
fn main_entry_with_runs_the_full_check_flow_and_cleans_semver_scratch() {
    let repo_root = tempdir().expect("repo tempdir");
    write_repo_scaffold(repo_root.path());
    write_toolchain_contract(repo_root.path());
    let tracked_file = write_tracked_source(repo_root.path(), "xtask/src/runner.rs");
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
                if *spec == coverage_command(current_root) {
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
    assert_eq!(
        calls
            .borrow()
            .iter()
            .filter(|spec| **spec == coverage_clean_command())
            .count(),
        2,
        "coverage cleanup should run before and after measurement"
    );
}

#[test]
fn main_entry_with_runs_only_the_semver_step_for_semver_check() {
    let repo_root = tempdir().expect("repo tempdir");
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
fn main_entry_with_reports_coverage_failures_and_runs_cleanup() {
    let repo_root = tempdir().expect("repo tempdir");
    let tracked_file = write_tracked_source(repo_root.path(), "xtask/src/uncovered.rs");
    let calls = Rc::new(RefCell::new(Vec::new()));
    let calls_for_override = Rc::clone(&calls);

    let error = with_ready_preflight(|| {
        crate::command_exec::with_run_spec_override(
            move |current_root, spec| {
                calls_for_override.borrow_mut().push(spec.clone());
                if *spec == coverage_command(current_root) {
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
}

#[test]
fn run_coverage_for_tests_reports_branch_only_failures() {
    let repo_root = tempdir().expect("repo tempdir");
    let tracked_file = write_tracked_source(repo_root.path(), "xtask/src/branch_only.rs");

    let error = with_ready_preflight(|| {
        crate::command_exec::with_run_spec_override(
            move |current_root, spec| {
                if *spec == coverage_command(current_root) {
                    write_coverage_report(current_root, &tracked_file, 1, 1, 0, 1);
                }
                Some(Ok(()))
            },
            || run_coverage_for_tests(repo_root.path()),
        )
    })
    .expect_err("branch-only coverage drift should fail");

    assert!(error.to_string().contains("coverage gate failed"));
}

#[test]
fn run_coverage_for_tests_reports_line_only_failures() {
    let repo_root = tempdir().expect("repo tempdir");
    let tracked_file = write_tracked_source(repo_root.path(), "xtask/src/line_only.rs");

    let error = with_ready_preflight(|| {
        crate::command_exec::with_run_spec_override(
            move |current_root, spec| {
                if *spec == coverage_command(current_root) {
                    write_coverage_report(current_root, &tracked_file, 0, 0, 0, 0);
                }
                Some(Ok(()))
            },
            || run_coverage_for_tests(repo_root.path()),
        )
    })
    .expect_err("line-only coverage drift should fail");

    assert!(error.to_string().contains("coverage gate failed"));
}

#[test]
fn main_entry_with_runs_one_targeted_fuzz_smoke_command() {
    let repo_root = tempdir().expect("repo tempdir");
    let checked_in_corpus = repo_root.path().join("fuzz/corpus/selector_parsing");
    fs::create_dir_all(&checked_in_corpus).expect("create corpus dir");
    fs::write(checked_in_corpus.join("seed"), "alpha").expect("write seed");

    let calls = Rc::new(RefCell::new(Vec::new()));
    let calls_for_override = Rc::clone(&calls);

    with_ready_preflight(|| {
        crate::command_exec::with_run_spec_override(
            move |_, spec| {
                calls_for_override.borrow_mut().push(spec.clone());
                Some(Ok(()))
            },
            || {
                main_entry_with(
                    repo_root.path(),
                    [
                        "xtask",
                        "fuzz-smoke",
                        "--target",
                        "selector_parsing",
                        "--runs",
                        "13",
                    ],
                )
            },
        )
    })
    .expect("targeted fuzz-smoke should pass");

    let calls = calls.borrow();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].program, PathBuf::from("cargo"));
    assert!(calls[0].args.iter().any(|arg| arg == "selector_parsing"));
    assert!(calls[0].args.iter().any(|arg| arg == "-runs=13"));
}

#[test]
fn main_entry_with_runs_the_full_fuzz_smoke_inventory() {
    let repo_root = tempdir().expect("repo tempdir");
    for target in fuzz_smoke_targets() {
        let checked_in_corpus = repo_root.path().join("fuzz/corpus").join(target);
        fs::create_dir_all(&checked_in_corpus).expect("create corpus dir");
        fs::write(checked_in_corpus.join("seed"), target).expect("write seed");
    }

    let call_count = Rc::new(RefCell::new(0usize));
    let call_count_for_override = Rc::clone(&call_count);

    with_ready_preflight(|| {
        crate::command_exec::with_run_spec_override(
            move |_, _| {
                *call_count_for_override.borrow_mut() += 1;
                Some(Ok(()))
            },
            || main_entry_with(repo_root.path(), ["xtask", "fuzz-smoke", "--runs", "5"]),
        )
    })
    .expect("full fuzz-smoke inventory should pass");

    assert_eq!(*call_count.borrow(), fuzz_smoke_targets().len());
}

#[test]
fn main_entry_with_refreshes_the_semver_baseline_snapshot() {
    let repo_root = tempdir().expect("repo tempdir");
    fs::write(
        repo_root.path().join("Cargo.toml"),
        "[workspace.package]\nversion = \"3.0.0\"\n",
    )
    .expect("write Cargo.toml");
    let baseline_parent = repo_root.path().join("semver-baseline");
    let baseline_dir = baseline_parent.join("htmlcut-core");
    let stale_extracted_dir = baseline_parent.join("htmlcut-core-4.2.0");
    fs::create_dir_all(&baseline_dir).expect("create baseline dir");
    fs::create_dir_all(&stale_extracted_dir).expect("create stale extracted dir");
    fs::write(baseline_dir.join("Cargo.toml"), "old baseline\n").expect("write old baseline");
    fs::write(stale_extracted_dir.join("Cargo.toml"), "stale\n").expect("write stale manifest");

    let calls = Rc::new(RefCell::new(Vec::new()));
    let calls_for_override = Rc::clone(&calls);
    let packaged_manifest = Rc::new(RefCell::new(String::new()));
    let packaged_manifest_for_override = Rc::clone(&packaged_manifest);
    let repo_root_path = repo_root.path().to_path_buf();

    crate::command_exec::with_run_spec_override(
        move |current_root, spec| {
            calls_for_override.borrow_mut().push(spec.clone());
            let args = spec.args.iter().map(String::as_str).collect::<Vec<_>>();

            if spec.program == Path::new("tar")
                && args.first() == Some(&"-xf")
                && args.get(2) == Some(&"-C")
            {
                let snapshot_root = PathBuf::from(args[3]);
                fs::create_dir_all(snapshot_root.join("crates/htmlcut-core"))
                    .expect("create snapshot crate dir");
                fs::write(
                    snapshot_root.join("Cargo.toml"),
                    "[workspace.package]\nversion = \"4.2.0\"\n",
                )
                .expect("write snapshot workspace Cargo.toml");
                fs::write(
                    snapshot_root.join("crates/htmlcut-core/Cargo.toml"),
                    "[package]\nname = \"htmlcut-core\"\nversion = \"4.2.0\"\n\n[dev-dependencies]\ninsta = \"1\"\n",
                )
                .expect("write snapshot crate manifest");
                return Some(Ok(()));
            }

            if spec.program == Path::new("cargo") && args[..5] == ["package", "--allow-dirty", "--no-verify", "-p", "htmlcut-core"] {
                let manifest = fs::read_to_string(current_root.join("crates/htmlcut-core/Cargo.toml"))
                    .expect("read stripped snapshot manifest");
                assert!(
                    !manifest.contains("[dev-dependencies]"),
                    "snapshot manifest should be stripped before packaging"
                );
                *packaged_manifest_for_override.borrow_mut() = manifest;
                let archive = current_root.join("target/package/htmlcut-core-4.2.0.crate");
                fs::create_dir_all(archive.parent().expect("archive parent"))
                    .expect("create archive parent");
                fs::write(&archive, "crate archive").expect("write crate archive");
                return Some(Ok(()));
            }

            if spec.program == Path::new("tar")
                && args.first() == Some(&"-xzf")
                && args.get(2) == Some(&"-C")
            {
                let extracted_dir = repo_root_path.join("semver-baseline/htmlcut-core-4.2.0");
                fs::create_dir_all(&extracted_dir).expect("create extracted dir");
                fs::write(
                    extracted_dir.join("Cargo.toml"),
                    packaged_manifest_for_override.borrow().as_str(),
                )
                    .expect("write extracted manifest");
                return Some(Ok(()));
            }

            Some(Ok(()))
        },
        || {
            main_entry_with(
                repo_root.path(),
                ["xtask", "refresh-semver-baseline", "--git-ref", "v4.2.0"],
            )
        },
    )
    .expect("refresh-semver-baseline should pass");

    let refreshed_manifest = fs::read_to_string(baseline_dir.join("Cargo.toml"))
        .expect("read refreshed baseline manifest");
    assert!(refreshed_manifest.contains("version = \"4.2.0\""));
    assert!(!refreshed_manifest.contains("[dev-dependencies]"));
    assert!(refreshed_manifest.contains("\n[workspace]\n"));
    assert!(
        !stale_extracted_dir.exists(),
        "stale extracted dir should be replaced"
    );
    assert!(
        calls
            .borrow()
            .iter()
            .any(|spec| spec.program == Path::new("git")),
        "refresh flow should archive the requested git ref"
    );
}

#[test]
fn refresh_semver_baseline_for_tests_bootstraps_missing_baseline_dirs() {
    let repo_root = tempdir().expect("repo tempdir");
    fs::write(
        repo_root.path().join("Cargo.toml"),
        "[workspace.package]\nversion = \"3.0.0\"\n",
    )
    .expect("write Cargo.toml");
    let packaged_manifest = Rc::new(RefCell::new(String::new()));
    let packaged_manifest_for_override = Rc::clone(&packaged_manifest);
    let repo_root_path = repo_root.path().to_path_buf();

    crate::command_exec::with_run_spec_override(
        move |current_root, spec| {
            let args = spec.args.iter().map(String::as_str).collect::<Vec<_>>();

            if spec.program == Path::new("tar")
                && args.first() == Some(&"-xf")
                && args.get(2) == Some(&"-C")
            {
                let snapshot_root = PathBuf::from(args[3]);
                fs::create_dir_all(snapshot_root.join("crates/htmlcut-core"))
                    .expect("create snapshot crate dir");
                fs::write(
                    snapshot_root.join("Cargo.toml"),
                    "[workspace.package]\nversion = \"4.2.0\"\n",
                )
                .expect("write snapshot workspace Cargo.toml");
                fs::write(
                    snapshot_root.join("crates/htmlcut-core/Cargo.toml"),
                    "[package]\nname = \"htmlcut-core\"\nversion = \"4.2.0\"\n",
                )
                .expect("write snapshot crate manifest");
                return Some(Ok(()));
            }

            if spec.program == Path::new("cargo")
                && args[..5]
                    == [
                        "package",
                        "--allow-dirty",
                        "--no-verify",
                        "-p",
                        "htmlcut-core",
                    ]
            {
                *packaged_manifest_for_override.borrow_mut() =
                    fs::read_to_string(current_root.join("crates/htmlcut-core/Cargo.toml"))
                        .expect("read packaged manifest");
                let archive = current_root.join("target/package/htmlcut-core-4.2.0.crate");
                fs::create_dir_all(archive.parent().expect("archive parent"))
                    .expect("create archive parent");
                fs::write(&archive, "crate archive").expect("write crate archive");
                return Some(Ok(()));
            }

            if spec.program == Path::new("tar")
                && args.first() == Some(&"-xzf")
                && args.get(2) == Some(&"-C")
            {
                let extracted_dir = repo_root_path.join("semver-baseline/htmlcut-core-4.2.0");
                fs::create_dir_all(&extracted_dir).expect("create extracted dir");
                fs::write(
                    extracted_dir.join("Cargo.toml"),
                    packaged_manifest_for_override.borrow().as_str(),
                )
                .expect("write extracted manifest");
                return Some(Ok(()));
            }

            Some(Ok(()))
        },
        || refresh_semver_baseline_for_tests(repo_root.path(), "v4.2.0"),
    )
    .expect("refresh-semver-baseline should create missing baseline dirs");

    let refreshed_manifest = fs::read_to_string(
        repo_root
            .path()
            .join("semver-baseline/htmlcut-core/Cargo.toml"),
    )
    .expect("read refreshed baseline manifest");
    assert!(refreshed_manifest.contains("version = \"4.2.0\""));
    assert!(refreshed_manifest.contains("\n[workspace]\n"));
}
