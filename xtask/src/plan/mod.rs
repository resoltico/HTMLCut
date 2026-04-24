use std::path::{Path, PathBuf};

mod check;
mod paths;
mod semver;

use crate::model::{CommandSpec, DynResult};

/// Builds the ordered command plan for `cargo xtask check`.
pub fn check_plan(repo_root: &Path) -> DynResult<Vec<CommandSpec>> {
    check::check_plan(repo_root)
}

/// Returns whether a command spec is the semver-checks gate step.
pub fn is_semver_check_spec(spec: &CommandSpec) -> bool {
    check::is_semver_check_spec(spec)
}

/// Lists shell scripts that should be syntax-checked and linted by the maintainer gate.
pub fn shell_script_paths(repo_root: &Path) -> DynResult<Vec<PathBuf>> {
    check::shell_script_paths(repo_root)
}

/// Resolves the Cargo target directory, honoring `CARGO_TARGET_DIR` when present.
pub fn cargo_target_dir(repo_root: &Path) -> PathBuf {
    paths::cargo_target_dir(repo_root)
}

/// Canonicalizes a repo-relative or absolute path against the repository root.
pub fn normalize_path(repo_root: &Path, path: &Path) -> DynResult<PathBuf> {
    paths::normalize_path(repo_root, path)
}

/// Returns the distribution-profile binary path used by smoke checks.
pub fn release_binary_path(repo_root: &Path) -> PathBuf {
    paths::release_binary_path(repo_root)
}

/// Returns the manifest path for the public `htmlcut-core` crate.
pub fn core_manifest_path(repo_root: &Path) -> PathBuf {
    paths::core_manifest_path(repo_root)
}

/// Returns the unpacked semver baseline directory for `htmlcut-core`.
pub fn semver_baseline_path(repo_root: &Path) -> PathBuf {
    paths::semver_baseline_path(repo_root)
}

/// Returns the semver-checks scratch directory under the Cargo target tree.
pub fn semver_scratch_dir(repo_root: &Path) -> PathBuf {
    paths::semver_scratch_dir(repo_root)
}

/// Returns the platform-specific HTMLCut binary name.
pub fn binary_name() -> &'static str {
    paths::binary_name()
}

/// Infers the semver release type that `cargo semver-checks` should enforce.
pub fn semver_release_type(repo_root: &Path) -> DynResult<String> {
    semver::semver_release_type(repo_root)
}

/// Maps the workspace and baseline versions to the semver release type checked in CI.
pub fn semver_release_type_from_versions(
    workspace_version: &str,
    baseline_version: &str,
) -> String {
    semver::semver_release_type_from_versions(workspace_version, baseline_version)
}

/// Adds a minimal workspace stub to isolated manifests used by the semver baseline flow.
pub fn with_workspace_stub(cargo_toml: &str) -> String {
    semver::with_workspace_stub(cargo_toml)
}

/// Removes dev-dependency tables from a manifest used only for semver-baseline packaging.
pub fn strip_dev_dependency_tables(cargo_toml: &str) -> String {
    semver::strip_dev_dependency_tables(cargo_toml)
}

#[cfg(test)]
pub(crate) fn cargo_target_dir_for_tests(repo_root: &Path, target_dir: Option<&Path>) -> PathBuf {
    paths::cargo_target_dir_for_tests(repo_root, target_dir)
}

#[cfg(test)]
pub(crate) fn release_binary_path_for_tests(
    repo_root: &Path,
    target_dir: Option<&Path>,
) -> PathBuf {
    paths::release_binary_path_for_tests(repo_root, target_dir)
}

#[cfg(test)]
pub(crate) fn semver_scratch_dir_for_tests(repo_root: &Path, target_dir: Option<&Path>) -> PathBuf {
    paths::semver_scratch_dir_for_tests(repo_root, target_dir)
}

#[cfg(test)]
pub(crate) fn is_maintained_shell_script_for_tests(repo_root: &Path, path: &Path) -> bool {
    check::is_maintained_shell_script_for_tests(repo_root, path)
}
