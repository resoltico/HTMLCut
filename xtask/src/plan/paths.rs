use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::model::DynResult;

/// Resolves the Cargo target directory, honoring `CARGO_TARGET_DIR` when present.
pub fn cargo_target_dir(repo_root: &Path) -> PathBuf {
    cargo_target_dir_from(
        repo_root,
        env::var_os("CARGO_TARGET_DIR").map(PathBuf::from),
    )
}

/// Canonicalizes a repo-relative or absolute path against the repository root.
pub fn normalize_path(repo_root: &Path, path: &Path) -> DynResult<PathBuf> {
    let candidate = if path.is_absolute() {
        path.to_path_buf()
    } else {
        repo_root.join(path)
    };

    Ok(fs::canonicalize(candidate)?)
}

/// Returns the distribution-profile binary path used by smoke checks.
pub fn release_binary_path(repo_root: &Path) -> PathBuf {
    cargo_target_dir(repo_root).join("dist").join(binary_name())
}

/// Returns the manifest path for the public `htmlcut-core` crate.
pub fn core_manifest_path(repo_root: &Path) -> PathBuf {
    repo_root
        .join("crates")
        .join("htmlcut-core")
        .join("Cargo.toml")
}

/// Returns the unpacked semver baseline directory for `htmlcut-core`.
pub fn semver_baseline_path(repo_root: &Path) -> PathBuf {
    repo_root.join("semver-baseline").join("htmlcut-core")
}

/// Returns the semver-checks scratch directory under the Cargo target tree.
pub fn semver_scratch_dir(repo_root: &Path) -> PathBuf {
    cargo_target_dir(repo_root).join("semver-checks")
}

#[cfg(windows)]
/// Returns the platform-specific HTMLCut binary name.
pub fn binary_name() -> &'static str {
    "htmlcut.exe"
}

#[cfg(not(windows))]
/// Returns the platform-specific HTMLCut binary name.
pub fn binary_name() -> &'static str {
    "htmlcut"
}

fn cargo_target_dir_from(repo_root: &Path, target_dir: Option<PathBuf>) -> PathBuf {
    match target_dir {
        Some(target_dir) if target_dir.is_absolute() => target_dir,
        Some(target_dir) => repo_root.join(target_dir),
        None => repo_root.join("target"),
    }
}

#[cfg(test)]
pub(crate) fn cargo_target_dir_for_tests(repo_root: &Path, target_dir: Option<&Path>) -> PathBuf {
    cargo_target_dir_from(repo_root, target_dir.map(Path::to_path_buf))
}

#[cfg(test)]
pub(crate) fn semver_scratch_dir_for_tests(repo_root: &Path, target_dir: Option<&Path>) -> PathBuf {
    cargo_target_dir_for_tests(repo_root, target_dir).join("semver-checks")
}

#[cfg(test)]
pub(crate) fn release_binary_path_for_tests(
    repo_root: &Path,
    target_dir: Option<&Path>,
) -> PathBuf {
    cargo_target_dir_for_tests(repo_root, target_dir)
        .join("dist")
        .join(binary_name())
}
