use std::path::{Path, PathBuf};

mod check;
mod paths;
mod semver;

use crate::model::{CommandSpec, DynResult};
pub use check::ci_rust_gate_plan;

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

/// Resolves the Cargo target directory owned by this repository.
pub fn cargo_target_dir(repo_root: &Path) -> PathBuf {
    paths::cargo_target_dir(repo_root)
}

/// Resolves the Cargo build directory owned by this repository.
pub fn cargo_build_dir(repo_root: &Path) -> PathBuf {
    paths::cargo_build_dir(repo_root)
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

/// Returns the isolated Cargo target directory used by the coverage gate.
pub fn coverage_target_dir(repo_root: &Path) -> PathBuf {
    paths::coverage_target_dir(repo_root)
}

/// Returns the isolated Cargo build directory used by the coverage gate.
pub fn coverage_build_dir(repo_root: &Path) -> PathBuf {
    paths::coverage_build_dir(repo_root)
}

/// Returns the managed evidence root retained for completed maintainer-gate runs.
pub fn gate_report_dir(repo_root: &Path) -> PathBuf {
    paths::gate_report_dir(repo_root)
}

/// Returns the nested Cargo target directory created by `cargo llvm-cov`.
pub(crate) fn coverage_cargo_target_dir(repo_root: &Path) -> PathBuf {
    paths::coverage_cargo_target_dir(repo_root)
}

/// Returns the nested Cargo build directory created by `cargo llvm-cov`.
pub(crate) fn coverage_cargo_build_dir(repo_root: &Path) -> PathBuf {
    paths::coverage_cargo_build_dir(repo_root)
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

/// Rewrites vendored workspace dependencies back to registry coordinates for semver packaging.
pub fn sanitize_snapshot_workspace_manifest_for_baseline(cargo_toml: &str) -> DynResult<String> {
    semver::sanitize_snapshot_workspace_manifest_for_baseline(cargo_toml)
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
pub(crate) fn cargo_build_dir_for_tests(repo_root: &Path, build_dir: Option<&Path>) -> PathBuf {
    paths::cargo_build_dir_for_tests(repo_root, build_dir)
}

#[cfg(test)]
pub(crate) fn cargo_target_dir_from_sources_for_tests(
    repo_root: &Path,
    env_target_dir: Option<&Path>,
    config_target_dir: Option<&Path>,
) -> PathBuf {
    paths::cargo_target_dir_from_sources_for_tests(repo_root, env_target_dir, config_target_dir)
}

#[cfg(test)]
pub(crate) fn cargo_build_dir_from_sources_for_tests(
    repo_root: &Path,
    env_target_dir: Option<&Path>,
    config_target_dir: Option<&Path>,
    env_build_dir: Option<&Path>,
    config_build_dir: Option<&Path>,
) -> PathBuf {
    paths::cargo_build_dir_from_sources_for_tests(
        repo_root,
        env_target_dir,
        config_target_dir,
        env_build_dir,
        config_build_dir,
    )
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
pub(crate) fn coverage_target_dir_for_tests(
    repo_root: &Path,
    target_dir: Option<&Path>,
) -> PathBuf {
    paths::coverage_target_dir_for_tests(repo_root, target_dir)
}

#[cfg(test)]
pub(crate) fn coverage_build_dir_for_tests(repo_root: &Path, build_dir: Option<&Path>) -> PathBuf {
    paths::coverage_build_dir_for_tests(repo_root, build_dir)
}

#[cfg(test)]
pub(crate) fn is_maintained_shell_script_for_tests(repo_root: &Path, path: &Path) -> bool {
    check::is_maintained_shell_script_for_tests(repo_root, path)
}

#[cfg(test)]
pub(crate) fn devcontainer_validation_command_for_tests(repo_root: &Path) -> CommandSpec {
    check::devcontainer_validation_command_for_tests(repo_root)
}

#[cfg(test)]
pub(crate) fn should_run_devcontainer_validation_for_tests(repo_root: &Path) -> DynResult<bool> {
    check::should_run_devcontainer_validation_for_tests(repo_root)
}

#[cfg(test)]
pub(crate) fn devcontainer_changed_file_args_for_tests(repo_root: &Path) -> DynResult<Vec<String>> {
    check::devcontainer_changed_file_args_for_tests(repo_root)
}

#[cfg(test)]
pub(crate) fn devcontainer_untracked_file_args_for_tests() -> Vec<String> {
    check::devcontainer_untracked_file_args_for_tests()
}

#[cfg(test)]
pub(crate) fn with_cargo_artifact_dir_overrides_for_tests<T>(
    target_dir: PathBuf,
    build_dir: PathBuf,
    operation: impl FnOnce() -> T,
) -> T {
    paths::with_cargo_artifact_dir_overrides(target_dir, build_dir, operation)
}

#[cfg(test)]
pub(crate) fn with_process_env_passthrough_for_tests<T>(operation: impl FnOnce() -> T) -> T {
    paths::with_process_env_passthrough_for_tests(operation)
}

#[cfg(test)]
pub(crate) fn coverage_cargo_target_dir_for_tests(
    repo_root: &Path,
    target_dir: Option<&Path>,
) -> PathBuf {
    paths::coverage_cargo_target_dir_for_tests(repo_root, target_dir)
}

#[cfg(test)]
pub(crate) fn coverage_cargo_build_dir_for_tests(
    repo_root: &Path,
    build_dir: Option<&Path>,
) -> PathBuf {
    paths::coverage_cargo_build_dir_for_tests(repo_root, build_dir)
}

#[cfg(test)]
pub(crate) fn sibling_artifact_dir_for_tests(path: &Path, sibling_name: &str) -> PathBuf {
    paths::sibling_artifact_dir_for_tests(path, sibling_name)
}
