use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};

use crate::model::{
    BranchCoverageByFile, COVERAGE_EXCLUDED_RELATIVE_PATHS, COVERAGE_SOURCE_ROOTS,
    COVERAGE_TOOLCHAIN, COVERAGE_TOOLCHAIN_NAME, CommandSpec, CoverageCounter, CoverageFailure,
    CoveragePreflightFailure, CoverageReport, CoverageSummary, DynResult,
};
use crate::plan::{cargo_target_dir, normalize_path};

/// Builds the `cargo llvm-cov` command used by the one-ring coverage gate.
pub fn coverage_command(repo_root: &Path) -> CommandSpec {
    CommandSpec::new(
        "cargo",
        [
            COVERAGE_TOOLCHAIN,
            "llvm-cov",
            "--branch",
            "--workspace",
            "--all-targets",
            "--all-features",
            "--locked",
            "--json",
            "--output-path",
            coverage_output_path(repo_root).to_string_lossy().as_ref(),
        ],
        false,
        true,
    )
}

/// Builds the cleanup command that clears stale `llvm-cov` state before measurement.
pub fn coverage_clean_command() -> CommandSpec {
    CommandSpec::new(
        "cargo",
        [COVERAGE_TOOLCHAIN, "llvm-cov", "clean", "--workspace"],
        false,
        false,
    )
}

/// Returns missing nightly prerequisites for the branch-coverage gate.
pub fn coverage_preflight_failures(
    toolchains_output: &str,
    installed_components_output: &str,
) -> Vec<CoveragePreflightFailure> {
    let has_nightly_toolchain = toolchains_output
        .lines()
        .map(str::trim)
        .any(|line| line.starts_with(COVERAGE_TOOLCHAIN_NAME));
    if !has_nightly_toolchain {
        return vec![
            CoveragePreflightFailure::MissingNightlyToolchain,
            CoveragePreflightFailure::MissingNightlyLlvmTools,
        ];
    }

    let has_llvm_tools = installed_components_output
        .lines()
        .map(str::trim)
        .any(|line| line.starts_with("llvm-tools"));

    if has_llvm_tools {
        Vec::new()
    } else {
        vec![CoveragePreflightFailure::MissingNightlyLlvmTools]
    }
}

/// Formats the actionable preflight error shown before coverage work starts.
pub fn coverage_preflight_message(failures: &[CoveragePreflightFailure]) -> String {
    let missing_nightly = failures.contains(&CoveragePreflightFailure::MissingNightlyToolchain);
    let missing_llvm_tools = failures.contains(&CoveragePreflightFailure::MissingNightlyLlvmTools);

    let mut message = String::from(
        "Rust coverage preflight failed. HTMLCut keeps stable as the default toolchain, but the coverage gate still requires `cargo +nightly llvm-cov --branch` for true branch coverage.\n",
    );

    if missing_nightly {
        message.push_str(
            "\nInstall the nightly coverage toolchain first:\n  rustup toolchain install nightly --profile minimal --component llvm-tools-preview\n",
        );
        return message;
    }

    if missing_llvm_tools {
        message.push_str(
            "\nNightly is installed, but `llvm-tools-preview` is missing:\n  rustup component add llvm-tools-preview --toolchain nightly\n",
        );
    }

    message
}

/// Returns the JSON file that `cargo llvm-cov` writes for later scoring.
pub fn coverage_output_path(repo_root: &Path) -> PathBuf {
    coverage_target_dir(repo_root).join("coverage.json")
}

/// Ensures the target directory that will receive `coverage.json` already exists.
pub fn ensure_coverage_output_dir(repo_root: &Path) -> DynResult<()> {
    let output_path = coverage_output_path(repo_root);
    let parent = output_path
        .parent()
        .expect("coverage output path should have a parent directory");
    fs::create_dir_all(parent)?;
    Ok(())
}

/// Loads the curated set of production files that the coverage gate tracks.
pub fn tracked_files(repo_root: &Path) -> DynResult<BTreeMap<PathBuf, String>> {
    let mut tracked_files = BTreeMap::new();
    let excluded_paths = coverage_excluded_paths();

    for relative_root in COVERAGE_SOURCE_ROOTS {
        collect_tracked_files(
            repo_root,
            &repo_root.join(relative_root),
            &excluded_paths,
            &mut tracked_files,
        )?;
    }

    Ok(tracked_files)
}

/// Scores one `llvm-cov` report against the tracked-file coverage policy.
pub fn evaluate_coverage_report(
    repo_root: &Path,
    tracked_files: &BTreeMap<PathBuf, String>,
    report: CoverageReport,
) -> DynResult<CoverageSummary> {
    let mut coverage_by_file: BTreeMap<PathBuf, BTreeMap<u64, u64>> = BTreeMap::new();
    let mut branch_records_by_file: BranchCoverageByFile = BTreeMap::new();
    let mut branch_summary_by_file: BTreeMap<PathBuf, CoverageCounter> = BTreeMap::new();

    for data_set in report.data {
        for file in data_set.files {
            let normalized_filename = normalize_path(repo_root, &file.filename)?;
            if !tracked_files.contains_key(&normalized_filename) {
                continue;
            }

            let line_counts = coverage_by_file
                .entry(normalized_filename.clone())
                .or_default();
            for (line, _, count, _, has_count, _) in file.segments {
                if !has_count {
                    continue;
                }

                let current = line_counts.entry(line).or_insert(0);
                *current = (*current).max(count);
            }

            if !file.branches.is_empty() {
                let branch_records = branch_records_by_file
                    .entry(normalized_filename.clone())
                    .or_default();
                for (
                    start_line,
                    start_column,
                    end_line,
                    end_column,
                    first_count,
                    second_count,
                    ..,
                ) in file.branches
                {
                    let entry = branch_records
                        .entry((start_line, start_column, end_line, end_column))
                        .or_insert((0, 0));
                    entry.0 = entry.0.max(first_count);
                    entry.1 = entry.1.max(second_count);
                }
            }

            let current_branch_summary = branch_summary_by_file
                .entry(normalized_filename)
                .or_default();
            current_branch_summary.count = current_branch_summary
                .count
                .max(file.summary.branches.count);
            current_branch_summary.covered = current_branch_summary
                .covered
                .max(file.summary.branches.covered);
            current_branch_summary.not_covered = current_branch_summary
                .not_covered
                .max(file.summary.branches.not_covered);
        }
    }

    let mut failures = Vec::new();
    let mut tracked_line_count = 0usize;
    let mut tracked_branch_count = 0usize;

    for (tracked_file, display_path) in tracked_files {
        let Some(line_counts) = coverage_by_file.get(tracked_file) else {
            failures.push(CoverageFailure {
                file: display_path.clone(),
                uncovered_lines: vec!["<no executable lines found>".to_owned()],
                uncovered_branch_count: 0,
            });
            continue;
        };

        tracked_line_count += line_counts.len();
        let uncovered_lines: Vec<String> = line_counts
            .iter()
            .filter_map(|(line, count)| (*count == 0).then_some(line.to_string()))
            .collect();
        let (branch_count, uncovered_branch_count) =
            if let Some(branch_records) = branch_records_by_file.get(tracked_file) {
                let branch_count = branch_records.len() * 2;
                let uncovered_branch_count = branch_records
                    .values()
                    .map(|(first_count, second_count)| {
                        usize::from(*first_count == 0) + usize::from(*second_count == 0)
                    })
                    .sum();
                (branch_count, uncovered_branch_count)
            } else {
                let branch_summary = branch_summary_by_file
                    .get(tracked_file)
                    .copied()
                    .unwrap_or_default();
                (
                    branch_summary.count as usize,
                    branch_summary.not_covered as usize,
                )
            };
        tracked_branch_count += branch_count;

        if !uncovered_lines.is_empty() || uncovered_branch_count > 0 {
            failures.push(CoverageFailure {
                file: display_path.clone(),
                uncovered_lines,
                uncovered_branch_count,
            });
        }
    }

    Ok(CoverageSummary {
        tracked_line_count,
        tracked_branch_count,
        failures,
    })
}

/// Reads and deserializes the `llvm-cov` JSON report from disk.
pub fn read_coverage_report(path: &Path) -> DynResult<CoverageReport> {
    Ok(serde_json::from_reader(File::open(path)?)?)
}

fn coverage_target_dir(repo_root: &Path) -> PathBuf {
    cargo_target_dir(repo_root)
}

fn coverage_excluded_paths() -> BTreeSet<&'static str> {
    COVERAGE_EXCLUDED_RELATIVE_PATHS.iter().copied().collect()
}

fn collect_tracked_files(
    repo_root: &Path,
    current_path: &Path,
    excluded_paths: &BTreeSet<&str>,
    tracked_files: &mut BTreeMap<PathBuf, String>,
) -> DynResult<()> {
    if !current_path.is_dir() {
        return Ok(());
    }

    for entry in fs::read_dir(current_path)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;

        if file_type.is_dir() {
            collect_tracked_files(repo_root, &path, excluded_paths, tracked_files)?;
            continue;
        }

        if path.extension() != Some(OsStr::new("rs")) {
            continue;
        }

        let absolute_path = normalize_path(repo_root, &path)?;
        let relative_path = repo_relative_source_path(repo_root, &absolute_path)?;
        if should_skip_coverage_path(&relative_path, excluded_paths) {
            continue;
        }

        tracked_files.insert(absolute_path, relative_path);
    }

    Ok(())
}

fn repo_relative_source_path(repo_root: &Path, absolute_path: &Path) -> DynResult<String> {
    let normalized_repo_root = normalize_path(repo_root, repo_root)?;
    let relative = absolute_path
        .strip_prefix(&normalized_repo_root)
        .map_err(|error| {
            format!(
                "coverage source {} does not live under repo root {}: {error}",
                absolute_path.display(),
                normalized_repo_root.display()
            )
        })?;
    Ok(relative.to_string_lossy().replace('\\', "/"))
}

fn should_skip_coverage_path(relative_path: &str, excluded_paths: &BTreeSet<&str>) -> bool {
    relative_path.ends_with("/main.rs")
        || relative_path.contains("/tests/")
        || excluded_paths.contains(relative_path)
}

#[cfg(test)]
pub(crate) fn coverage_output_path_for_tests(
    repo_root: &Path,
    target_dir: Option<&Path>,
) -> PathBuf {
    crate::plan::cargo_target_dir_for_tests(repo_root, target_dir).join("coverage.json")
}

#[cfg(test)]
pub(crate) fn coverage_target_dir_for_tests(
    repo_root: &Path,
    target_dir: Option<&Path>,
) -> PathBuf {
    crate::plan::cargo_target_dir_for_tests(repo_root, target_dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use htmlcut_tempdir::tempdir;

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
        let tracked_paths = tracked.values().cloned().collect::<Vec<_>>();

        assert!(tracked_paths.contains(&"crates/htmlcut-cli/src/lookup.rs".to_owned()));
        assert!(tracked_paths.contains(&"crates/htmlcut-cli/src/nested/report.rs".to_owned()));
        assert!(!tracked_paths.contains(&"crates/htmlcut-cli/src/main.rs".to_owned()));
        assert!(!tracked_paths.contains(&"crates/htmlcut-cli/src/tests/helper.rs".to_owned()));
        assert!(!tracked_paths.contains(&"crates/htmlcut-cli/src/model/catalog.rs".to_owned()));
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

        let error = repo_relative_source_path(repo_root.path(), &absolute_outside_file)
            .expect_err("outside paths should fail");

        assert!(error.to_string().contains("does not live under repo root"));
    }
}
