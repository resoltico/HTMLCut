use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

mod commands;
mod report;
mod tracking;

use crate::model::{
    CommandSpec, CoveragePreflightFailure, CoverageReport, CoverageSummary, DynResult,
};

/// Builds the cleanup command that clears stale `llvm-cov` state before measurement.
pub fn coverage_clean_command() -> CommandSpec {
    commands::coverage_clean_command()
}

/// Builds the `cargo llvm-cov` command used by the one-ring coverage gate.
pub fn coverage_command(repo_root: &Path) -> CommandSpec {
    commands::coverage_command(repo_root)
}

/// Returns the JSON file that `cargo llvm-cov` writes for later scoring.
pub fn coverage_output_path(repo_root: &Path) -> PathBuf {
    commands::coverage_output_path(repo_root)
}

/// Returns missing nightly prerequisites for the branch-coverage gate.
pub fn coverage_preflight_failures(
    toolchains_output: &str,
    installed_components_output: &str,
) -> Vec<CoveragePreflightFailure> {
    commands::coverage_preflight_failures(toolchains_output, installed_components_output)
}

/// Formats the actionable preflight error shown before coverage work starts.
pub fn coverage_preflight_message(failures: &[CoveragePreflightFailure]) -> String {
    commands::coverage_preflight_message(failures)
}

/// Ensures the target directory that will receive `coverage.json` already exists.
pub fn ensure_coverage_output_dir(repo_root: &Path) -> DynResult<()> {
    commands::ensure_coverage_output_dir(repo_root)
}

/// Scores one `llvm-cov` report against the tracked-file coverage policy.
pub fn evaluate_coverage_report(
    repo_root: &Path,
    tracked_files: &BTreeMap<PathBuf, String>,
    report: CoverageReport,
) -> DynResult<CoverageSummary> {
    report::evaluate_coverage_report(repo_root, tracked_files, report)
}

/// Reads and deserializes the `llvm-cov` JSON report from disk.
pub fn read_coverage_report(path: &Path) -> DynResult<CoverageReport> {
    report::read_coverage_report(path)
}

/// Loads the curated set of production files that the coverage gate tracks.
pub fn tracked_files(repo_root: &Path) -> DynResult<BTreeMap<PathBuf, String>> {
    tracking::tracked_files(repo_root)
}

#[cfg(test)]
pub(crate) fn coverage_output_path_for_tests(
    repo_root: &Path,
    target_dir: Option<&Path>,
) -> PathBuf {
    commands::coverage_output_path_for_tests(repo_root, target_dir)
}

#[cfg(test)]
pub(crate) fn coverage_target_dir_for_tests(
    repo_root: &Path,
    target_dir: Option<&Path>,
) -> PathBuf {
    commands::coverage_target_dir_for_tests(repo_root, target_dir)
}

#[cfg(test)]
pub(crate) fn repo_relative_source_path_for_tests(
    repo_root: &Path,
    absolute_path: &Path,
) -> DynResult<String> {
    tracking::repo_relative_source_path_for_tests(repo_root, absolute_path)
}
