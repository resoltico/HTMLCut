//! Executable quality-gate workflows behind the `xtask` command dispatcher.

use std::fs;
use std::path::{Path, PathBuf};

use htmlcut_tempdir::tempdir;

use crate::plan::materialize_semver_baseline;
use crate::{
    CommandArtifactLayout, CommandSpec, CoverageFailure, DynResult, HygieneCleanMode,
    assert_known_fuzz_target, check_plan, check_source_structure, ci_rust_gate_plan, clean_hygiene,
    coverage_clean_command, coverage_command, coverage_output_path, ensure_coverage_output_dir,
    ensure_coverage_prerequisites, ensure_fuzz_smoke_prerequisites, ensure_hygiene,
    ensure_miri_prerequisites, ensure_mutants_prerequisites, ensure_repo_toolchain_prerequisites,
    evaluate_coverage_report, fuzz_smoke_command, fuzz_smoke_targets, is_semver_check_spec,
    miri_contract_command, mutants_command, mutants_output_dir, prepare_artifact_layout,
    prepare_mutation_report_root, read_coverage_report, remove_dir_if_exists, run_spec,
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

/// Runs cargo-mutants with the checked-in first-party mutation scope.
pub(super) fn run_mutants(
    repo_root: &Path,
    in_place: bool,
    shard: Option<&str>,
    in_diff: Option<&Path>,
) -> DynResult<()> {
    ensure_repo_toolchain_prerequisites(repo_root)?;
    ensure_mutants_prerequisites(repo_root)?;
    let diff_contents = read_mutation_diff(repo_root, in_diff)?;
    clean_hygiene(repo_root, HygieneCleanMode::Safe)?;
    if in_place {
        prepare_artifact_layout(repo_root, CommandArtifactLayout::ManagedWorkspace)?;
    }
    prepare_mutation_report_root(repo_root)?;
    ensure_hygiene(repo_root)?;

    let output_dir = mutants_output_dir(repo_root);
    remove_dir_if_exists(&output_dir.join("mutants.out"))?;
    remove_dir_if_exists(&output_dir.join("mutants.out.old"))?;
    let staged_diff = stage_mutation_diff(&output_dir, diff_contents.as_deref())?;
    let local_scratch = (!in_place).then(tempdir).transpose()?;
    let mut mutation_spec = mutants_command(&output_dir, in_place, shard, staged_diff.as_deref());
    if let Some(local_scratch) = &local_scratch {
        let temp_root = local_scratch.path().to_string_lossy().into_owned();
        mutation_spec = mutation_spec
            .with_env("TMPDIR", &temp_root)
            .with_env("TMP", &temp_root)
            .with_env("TEMP", temp_root);
    }
    let execution = run_spec(repo_root, &mutation_spec);
    drop(local_scratch);
    let cleanup = remove_staged_mutation_diff(staged_diff.as_deref());
    let hygiene = ensure_hygiene(repo_root);

    if let Err(error) = execution {
        cleanup?;
        return Err(mutation_execution_error(error));
    }
    cleanup?;
    hygiene
}

fn read_mutation_diff(repo_root: &Path, in_diff: Option<&Path>) -> DynResult<Option<Vec<u8>>> {
    in_diff
        .map(|path| {
            let path = if path.is_absolute() {
                path.to_owned()
            } else {
                repo_root.join(path)
            };
            fs::read(&path).map_err(|error| -> crate::XtaskError {
                format!("failed to read mutation diff {}: {error}", path.display()).into()
            })
        })
        .transpose()
}

fn stage_mutation_diff(output_dir: &Path, contents: Option<&[u8]>) -> DynResult<Option<PathBuf>> {
    contents
        .map(|contents| {
            let path = output_dir.join("input.diff");
            fs::write(&path, contents).map_err(|error| -> crate::XtaskError {
                format!("failed to stage mutation diff {}: {error}", path.display()).into()
            })?;
            Ok(path)
        })
        .transpose()
}

fn remove_staged_mutation_diff(path: Option<&Path>) -> DynResult<()> {
    let Some(path) = path else {
        return Ok(());
    };
    fs::remove_file(path).map_err(|error| -> crate::XtaskError {
        format!(
            "failed to remove staged mutation diff {}: {error}",
            path.display()
        )
        .into()
    })
}

fn mutation_execution_error(error: crate::XtaskError) -> crate::XtaskError {
    let outcome = match error.exit_code() {
        Some(2) => {
            "cargo-mutants found missed mutants. Review `missed.txt` and the per-mutant logs in the retained mutation evidence."
        }
        Some(3) => {
            "cargo-mutants timed out while testing one or more mutants. Review `timeout.txt` and the retained per-mutant logs before changing timeouts."
        }
        Some(4) => {
            "cargo-mutants could not establish a passing unmutated baseline. Repair the baseline test failure before interpreting mutation results."
        }
        _ => return error,
    };
    let message = error.to_string();
    format!("Mutation-testing run did not complete successfully: {outcome}\n\n{message}").into()
}

fn run_semver_step(repo_root: &Path, spec: CommandSpec) -> DynResult<()> {
    let scratch = semver_scratch_dir(repo_root);
    remove_dir_if_exists(&scratch)?;
    let result = (|| {
        let materialized_baseline =
            materialize_semver_baseline(repo_root, &scratch.join("baseline"))?;
        let spec = with_materialized_baseline(spec, &materialized_baseline)?;
        run_spec(repo_root, &spec)
    })();
    let cleanup = remove_dir_if_exists(&scratch);
    result?;
    cleanup
}

pub(super) fn with_materialized_baseline(
    mut spec: CommandSpec,
    materialized_baseline: &Path,
) -> DynResult<CommandSpec> {
    let flag_index = spec
        .args
        .iter()
        .position(|argument| argument == "--baseline-root")
        .ok_or("semver gate command is missing --baseline-root")?;
    let baseline_argument = spec
        .args
        .get_mut(flag_index + 1)
        .ok_or("semver gate command is missing the --baseline-root value")?;
    *baseline_argument = materialized_baseline.to_string_lossy().into_owned();
    Ok(spec)
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

#[cfg(test)]
mod tests;
