use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use crate::command_exec::repo_worktree_files;
use crate::model::{COVERAGE_EXCLUDED_RELATIVE_PATHS, COVERAGE_SOURCE_ROOTS, DynResult};
use crate::plan::normalize_path;

/// Loads the curated set of production files that the coverage gate tracks.
pub fn tracked_files(repo_root: &Path) -> DynResult<BTreeMap<PathBuf, String>> {
    let mut tracked_files = BTreeMap::new();
    let excluded_paths = coverage_excluded_paths();

    if let Some(paths) = repo_worktree_files(repo_root)? {
        collect_inventory_tracked_files(repo_root, &paths, &excluded_paths, &mut tracked_files)?;
    } else {
        for relative_root in COVERAGE_SOURCE_ROOTS {
            collect_tracked_files(
                repo_root,
                &repo_root.join(relative_root),
                &excluded_paths,
                &mut tracked_files,
            )?;
        }
    }

    Ok(tracked_files)
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

fn collect_inventory_tracked_files(
    repo_root: &Path,
    paths: &[PathBuf],
    excluded_paths: &BTreeSet<&str>,
    tracked_files: &mut BTreeMap<PathBuf, String>,
) -> DynResult<()> {
    for path in paths {
        if path.extension() != Some(OsStr::new("rs")) {
            continue;
        }

        let absolute_path = normalize_path(repo_root, path)?;
        let relative_path = repo_relative_source_path(repo_root, &absolute_path)?;
        if !is_under_coverage_root(&relative_path)
            || should_skip_coverage_path(&relative_path, excluded_paths)
        {
            continue;
        }

        tracked_files.insert(absolute_path, relative_path);
    }

    Ok(())
}

fn is_under_coverage_root(relative_path: &str) -> bool {
    COVERAGE_SOURCE_ROOTS
        .iter()
        .any(|root| relative_path == *root || relative_path.starts_with(&format!("{root}/")))
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
pub(crate) fn repo_relative_source_path_for_tests(
    repo_root: &Path,
    absolute_path: &Path,
) -> DynResult<String> {
    repo_relative_source_path(repo_root, absolute_path)
}

#[cfg(test)]
pub(crate) fn is_under_coverage_root_for_tests(relative_path: &str) -> bool {
    is_under_coverage_root(relative_path)
}
