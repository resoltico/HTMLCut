//! Executable quality-gate workflows behind the `xtask` command dispatcher.

use std::path::Path;

use htmlcut_tempdir::tempdir;

use crate::{
    CommandArtifactLayout, CommandSpec, CoverageFailure, DynResult, HygieneCleanMode,
    assert_known_fuzz_target, check_plan, check_source_structure, ci_rust_gate_plan, clean_hygiene,
    coverage_clean_command, coverage_command, coverage_output_path, ensure_coverage_output_dir,
    ensure_coverage_prerequisites, ensure_fuzz_smoke_prerequisites, ensure_hygiene,
    ensure_miri_prerequisites, ensure_repo_toolchain_prerequisites, evaluate_coverage_report,
    fuzz_smoke_command, fuzz_smoke_targets, is_semver_check_spec, miri_contract_command,
    prepare_artifact_layout, read_coverage_report, remove_dir_if_exists, run_spec,
    semver_scratch_dir, stage_fuzz_corpus, tracked_files,
};

/// Runs the complete maintainer quality gate.
pub(super) fn run_check(repo_root: &Path) -> DynResult<()> {
    ensure_repo_toolchain_prerequisites(repo_root)?;
    ensure_miri_prerequisites(repo_root)?;
    ensure_coverage_prerequisites(repo_root)?;
    clean_hygiene(repo_root, HygieneCleanMode::Safe)?;
    prepare_artifact_layout(repo_root, CommandArtifactLayout::ManagedWorkspace)?;
    ensure_hygiene(repo_root)?;
    check_source_structure(repo_root)?;
    for spec in check_plan(repo_root)? {
        if is_semver_check_spec(&spec) {
            run_semver_step(repo_root, spec)?;
        } else {
            run_spec(repo_root, &spec)?;
        }
    }

    run_coverage(repo_root)?;
    clean_hygiene(repo_root, HygieneCleanMode::Safe)?;
    ensure_hygiene(repo_root)
}

/// Runs the strict-provenance Miri proof.
pub(super) fn run_miri(repo_root: &Path) -> DynResult<()> {
    ensure_repo_toolchain_prerequisites(repo_root)?;
    ensure_miri_prerequisites(repo_root)?;
    clean_hygiene(repo_root, HygieneCleanMode::Safe)?;
    prepare_artifact_layout(repo_root, CommandArtifactLayout::ManagedWorkspace)?;
    ensure_hygiene(repo_root)?;
    run_spec(repo_root, &miri_contract_command())?;
    clean_hygiene(repo_root, HygieneCleanMode::Safe)?;
    ensure_hygiene(repo_root)
}

/// Runs the CI-compatible Rust gate without the coverage proof.
pub(super) fn run_ci_rust_gate(repo_root: &Path) -> DynResult<()> {
    ensure_repo_toolchain_prerequisites(repo_root)?;
    clean_hygiene(repo_root, HygieneCleanMode::Safe)?;
    prepare_artifact_layout(repo_root, CommandArtifactLayout::ManagedWorkspace)?;
    ensure_hygiene(repo_root)?;
    check_source_structure(repo_root)?;

    for spec in ci_rust_gate_plan(repo_root)? {
        if is_semver_check_spec(&spec) {
            run_semver_step(repo_root, spec)?;
        } else {
            run_spec(repo_root, &spec)?;
        }
    }

    clean_hygiene(repo_root, HygieneCleanMode::Safe)?;
    ensure_hygiene(repo_root)
}

/// Runs the semver check in isolation.
pub(super) fn run_semver_check(repo_root: &Path) -> DynResult<()> {
    ensure_repo_toolchain_prerequisites(repo_root)?;
    clean_hygiene(repo_root, HygieneCleanMode::Safe)?;
    prepare_artifact_layout(repo_root, CommandArtifactLayout::ManagedWorkspace)?;
    ensure_hygiene(repo_root)?;
    run_semver_step(repo_root, semver_check_spec(check_plan(repo_root)?)?)?;
    clean_hygiene(repo_root, HygieneCleanMode::Safe)?;
    ensure_hygiene(repo_root)
}

/// Returns the semver command from the complete maintainer plan.
pub(super) fn semver_check_spec(plan: Vec<CommandSpec>) -> DynResult<CommandSpec> {
    plan.into_iter()
        .find(is_semver_check_spec)
        .ok_or_else(|| "semver gate step is missing from cargo xtask check".into())
}

/// Runs the curated 100% line-and-branch coverage proof.
pub(super) fn run_coverage(repo_root: &Path) -> DynResult<()> {
    ensure_coverage_prerequisites(repo_root)?;
    clean_hygiene(repo_root, HygieneCleanMode::Safe)?;
    prepare_artifact_layout(repo_root, CommandArtifactLayout::ManagedCoverage)?;
    ensure_hygiene(repo_root)?;
    let coverage_clean_spec = coverage_clean_command();
    let coverage_spec = coverage_command(repo_root);
    run_spec(repo_root, &coverage_clean_spec)?;
    ensure_coverage_output_dir(repo_root)?;

    let result = (|| -> DynResult<()> {
        run_spec(repo_root, &coverage_spec)?;

        let tracked = tracked_files(repo_root)?;
        let report = read_coverage_report(&coverage_output_path(repo_root))?;
        let summary = evaluate_coverage_report(repo_root, &tracked, report)?;

        if !summary.failures.is_empty() {
            record_coverage_failure(
                &summary.failures,
                render_coverage_failures(&summary.failures),
            );
            return Err("coverage gate failed".into());
        }

        record_coverage_success(&format!(
            "Rust coverage: lines 100.00% ({0}/{0}) | branches 100.00% ({1}/{1})",
            summary.tracked_line_count, summary.tracked_branch_count
        ));
        Ok(())
    })();

    let cleanup = run_spec(repo_root, &coverage_clean_spec);
    result?;
    cleanup?;
    clean_hygiene(repo_root, HygieneCleanMode::Safe)?;
    ensure_hygiene(repo_root)
}

/// Runs the selected or complete libFuzzer smoke inventory.
pub(super) fn run_fuzz_smoke(repo_root: &Path, target: Option<&str>, runs: u32) -> DynResult<()> {
    if let Some(target) = target {
        assert_known_fuzz_target(target)?;
    }

    ensure_fuzz_smoke_prerequisites(repo_root)?;
    clean_hygiene(repo_root, HygieneCleanMode::Safe)?;
    prepare_artifact_layout(repo_root, CommandArtifactLayout::ManagedWorkspace)?;
    ensure_hygiene(repo_root)?;

    let targets = target
        .map(|target| vec![target])
        .unwrap_or_else(|| fuzz_smoke_targets().to_vec());
    for target in targets {
        let scratch_root = tempdir()?;
        let staged_corpus = stage_fuzz_corpus(repo_root, scratch_root.path(), target)?;
        run_spec(
            repo_root,
            &fuzz_smoke_command(target, &staged_corpus, runs)?,
        )?;
    }

    clean_hygiene(repo_root, HygieneCleanMode::Safe)?;
    ensure_hygiene(repo_root)
}

fn run_semver_step(repo_root: &Path, spec: CommandSpec) -> DynResult<()> {
    remove_dir_if_exists(&semver_scratch_dir(repo_root))?;
    let result = run_spec(repo_root, &spec);
    let cleanup = remove_dir_if_exists(&semver_scratch_dir(repo_root));
    result?;
    cleanup
}

fn record_coverage_success(message: &str) {
    if crate::gate_report::is_active() {
        crate::gate_report::record_internal_check(
            "Rust coverage ledger",
            Ok(()),
            std::time::Duration::ZERO,
        );
    } else {
        println!("{message}");
    }
}

fn record_coverage_failure(failures: &[CoverageFailure], message: String) {
    if crate::gate_report::is_active() {
        crate::gate_report::record_internal_check(
            "Rust coverage ledger",
            Err(message),
            std::time::Duration::ZERO,
        );
        return;
    }

    eprintln!("Rust coverage gate failed.");
    for failure in failures {
        if !failure.uncovered_lines.is_empty() {
            eprintln!(
                "- {} lines: {}",
                failure.file,
                failure.uncovered_lines.join(", ")
            );
        }
        if failure.uncovered_branch_count != 0 {
            eprintln!(
                "- {} branches: {} uncovered",
                failure.file, failure.uncovered_branch_count
            );
        }
    }
}

fn render_coverage_failures(failures: &[CoverageFailure]) -> String {
    failures
        .iter()
        .flat_map(|failure| {
            let mut lines = Vec::new();
            if !failure.uncovered_lines.is_empty() {
                lines.push(format!(
                    "{} lines: {}",
                    failure.file,
                    failure.uncovered_lines.join(", ")
                ));
            }
            if failure.uncovered_branch_count != 0 {
                lines.push(format!(
                    "{} branches: {} uncovered",
                    failure.file, failure.uncovered_branch_count
                ));
            }
            lines
        })
        .collect::<Vec<_>>()
        .join("\n")
}
