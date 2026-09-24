//! Narrow test-only adapters for private hygiene operations.

use std::path::{Path, PathBuf};

use crate::DynResult;

use super::*;

pub(crate) fn looks_like_cargo_target_dir_for_tests(path: &Path) -> bool {
    looks_like_cargo_target_dir(path)
}

pub(crate) fn format_bytes_for_tests(bytes: u64) -> String {
    format_bytes(bytes)
}

#[cfg(unix)]
pub(crate) fn dir_size_bytes_for_tests(path: &Path) -> u64 {
    dir_size_bytes(path).expect("dir size bytes")
}

#[cfg(unix)]
pub(crate) fn dir_size_bytes_result_for_tests(path: &Path) -> DynResult<u64> {
    dir_size_bytes(path)
}

#[cfg(unix)]
pub(crate) fn dir_size_bytes_excluding_roots_for_tests(
    path: &Path,
    skipped_roots: &[PathBuf],
) -> DynResult<u64> {
    dir_size_bytes_excluding_roots(path, skipped_roots)
}

#[cfg(unix)]
pub(crate) fn aggregate_entry_for_tests(path: &Path, roots: &[PathBuf]) -> DynResult<HygieneEntry> {
    aggregate_entry(
        "test-aggregate",
        "test-aggregate",
        path,
        roots,
        Some(0),
        false,
        true,
    )
}

pub(crate) fn checked_aggregate_bytes_for_tests(
    left: u64,
    right: u64,
    root: &Path,
) -> DynResult<u64> {
    checked_aggregate_bytes(left, right, root)
}

pub(crate) fn managed_artifact_container_entry_for_tests(
    container: &Path,
) -> DynResult<HygieneEntry> {
    managed_artifact_container_entry(container)
}

pub(crate) fn unmanaged_artifact_container_entry_for_tests(
    container: &Path,
    managed_roots: &[PathBuf],
) -> DynResult<HygieneEntry> {
    unmanaged_artifact_container_entry(container, managed_roots)
}

pub(crate) fn remove_artifact_path_if_exists_for_tests(path: &Path) -> DynResult<()> {
    remove_artifact_path_if_exists(path)
}

pub(crate) fn report_violations_for_tests(entries: &[HygieneEntry]) -> Vec<HygieneViolation> {
    report_violations(entries)
}
