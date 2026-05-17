use std::fs;
use std::path::{Path, PathBuf};

use crate::model::DynResult;
use serde::Deserialize;

#[cfg(test)]
use std::cell::RefCell;

#[cfg(test)]
thread_local! {
    static TEST_CARGO_TARGET_DIR_OVERRIDE: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
    static TEST_CARGO_BUILD_DIR_OVERRIDE: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
    static TEST_IGNORE_PROCESS_ENV: RefCell<bool> = const { RefCell::new(true) };
}

#[derive(Debug, Default, Deserialize)]
struct CargoConfigDocument {
    #[serde(default)]
    build: CargoBuildConfig,
}

#[derive(Debug, Default, Deserialize)]
struct CargoBuildConfig {
    #[serde(default, rename = "target-dir")]
    target_dir: Option<PathBuf>,
    #[serde(default, rename = "build-dir")]
    build_dir: Option<PathBuf>,
}

/// Resolves the Cargo target directory owned by this repository.
pub fn cargo_target_dir(repo_root: &Path) -> PathBuf {
    #[cfg(test)]
    if let Some(override_dir) = test_cargo_target_dir_override() {
        return override_dir;
    }

    let config = cargo_build_config(repo_root);
    cargo_target_dir_from_sources(
        repo_root,
        env_path_override("CARGO_TARGET_DIR").as_deref(),
        config.target_dir.as_deref(),
    )
}

/// Resolves the Cargo build directory owned by this repository.
pub fn cargo_build_dir(repo_root: &Path) -> PathBuf {
    #[cfg(test)]
    if let Some(override_dir) = test_cargo_build_dir_override() {
        return override_dir;
    }

    let config = cargo_build_config(repo_root);
    let target_dir = cargo_target_dir_from_sources(
        repo_root,
        env_path_override("CARGO_TARGET_DIR").as_deref(),
        config.target_dir.as_deref(),
    );
    cargo_build_dir_from_sources(
        repo_root,
        env_path_override("CARGO_BUILD_BUILD_DIR").as_deref(),
        config.build_dir.as_deref(),
        &target_dir,
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

/// Returns the isolated Cargo target directory used by the coverage gate.
pub fn coverage_target_dir(repo_root: &Path) -> PathBuf {
    sibling_artifact_dir(&cargo_target_dir(repo_root), "coverage-target")
}

/// Returns the isolated Cargo build directory used by the coverage gate.
pub fn coverage_build_dir(repo_root: &Path) -> PathBuf {
    sibling_artifact_dir(&cargo_build_dir(repo_root), "coverage-build")
}

/// Returns the nested Cargo target directory created by `cargo llvm-cov` inside the managed coverage root.
pub(crate) fn coverage_cargo_target_dir(repo_root: &Path) -> PathBuf {
    coverage_target_dir(repo_root).join("llvm-cov-target")
}

/// Returns the nested Cargo build directory created by `cargo llvm-cov` inside the managed coverage root.
pub(crate) fn coverage_cargo_build_dir(repo_root: &Path) -> PathBuf {
    coverage_build_dir(repo_root).join("llvm-cov-target")
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

fn cargo_target_dir_from_sources(
    repo_root: &Path,
    env_target_dir: Option<&Path>,
    config_target_dir: Option<&Path>,
) -> PathBuf {
    env_target_dir
        .or(config_target_dir)
        .map(|path| resolve_artifact_dir(repo_root, path))
        .unwrap_or_else(|| repo_root.join("target"))
}

fn cargo_build_dir_from_sources(
    repo_root: &Path,
    env_build_dir: Option<&Path>,
    config_build_dir: Option<&Path>,
    resolved_target_dir: &Path,
) -> PathBuf {
    env_build_dir
        .or(config_build_dir)
        .map(|path| resolve_artifact_dir(repo_root, path))
        .unwrap_or_else(|| resolved_target_dir.to_path_buf())
}

fn resolve_artifact_dir(repo_root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        repo_root.join(path)
    }
}

fn env_path_override(key: &str) -> Option<PathBuf> {
    #[cfg(test)]
    if test_ignores_process_env() {
        return None;
    }

    std::env::var_os(key)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn sibling_artifact_dir(path: &Path, sibling_name: &str) -> PathBuf {
    path.parent()
        .map(|parent| parent.join(sibling_name))
        .unwrap_or_else(|| PathBuf::from(sibling_name))
}

fn cargo_build_config(repo_root: &Path) -> CargoBuildConfig {
    let config_path = repo_root.join(".cargo").join("config.toml");
    let Ok(contents) = fs::read_to_string(config_path) else {
        return CargoBuildConfig::default();
    };
    toml::from_str::<CargoConfigDocument>(&contents)
        .map(|document| document.build)
        .unwrap_or_default()
}

#[cfg(test)]
pub(crate) fn cargo_target_dir_for_tests(repo_root: &Path, target_dir: Option<&Path>) -> PathBuf {
    cargo_target_dir_from_sources(repo_root, None, target_dir)
}

#[cfg(test)]
pub(crate) fn cargo_build_dir_for_tests(repo_root: &Path, build_dir: Option<&Path>) -> PathBuf {
    cargo_build_dir_from_sources(repo_root, None, build_dir, &cargo_target_dir(repo_root))
}

#[cfg(test)]
pub(crate) fn cargo_target_dir_from_sources_for_tests(
    repo_root: &Path,
    env_target_dir: Option<&Path>,
    config_target_dir: Option<&Path>,
) -> PathBuf {
    cargo_target_dir_from_sources(repo_root, env_target_dir, config_target_dir)
}

#[cfg(test)]
pub(crate) fn cargo_build_dir_from_sources_for_tests(
    repo_root: &Path,
    env_target_dir: Option<&Path>,
    config_target_dir: Option<&Path>,
    env_build_dir: Option<&Path>,
    config_build_dir: Option<&Path>,
) -> PathBuf {
    let resolved_target_dir =
        cargo_target_dir_from_sources(repo_root, env_target_dir, config_target_dir);
    cargo_build_dir_from_sources(
        repo_root,
        env_build_dir,
        config_build_dir,
        &resolved_target_dir,
    )
}

#[cfg(test)]
pub(crate) fn semver_scratch_dir_for_tests(repo_root: &Path, target_dir: Option<&Path>) -> PathBuf {
    cargo_target_dir_for_tests(repo_root, target_dir).join("semver-checks")
}

#[cfg(test)]
pub(crate) fn coverage_target_dir_for_tests(
    repo_root: &Path,
    target_dir: Option<&Path>,
) -> PathBuf {
    sibling_artifact_dir(
        &cargo_target_dir_for_tests(repo_root, target_dir),
        "coverage-target",
    )
}

#[cfg(test)]
pub(crate) fn coverage_build_dir_for_tests(repo_root: &Path, build_dir: Option<&Path>) -> PathBuf {
    sibling_artifact_dir(
        &cargo_build_dir_for_tests(repo_root, build_dir),
        "coverage-build",
    )
}

#[cfg(test)]
pub(crate) fn coverage_cargo_target_dir_for_tests(
    repo_root: &Path,
    target_dir: Option<&Path>,
) -> PathBuf {
    coverage_target_dir_for_tests(repo_root, target_dir).join("llvm-cov-target")
}

#[cfg(test)]
pub(crate) fn coverage_cargo_build_dir_for_tests(
    repo_root: &Path,
    build_dir: Option<&Path>,
) -> PathBuf {
    coverage_build_dir_for_tests(repo_root, build_dir).join("llvm-cov-target")
}

#[cfg(test)]
pub(crate) fn sibling_artifact_dir_for_tests(path: &Path, sibling_name: &str) -> PathBuf {
    sibling_artifact_dir(path, sibling_name)
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

#[cfg(test)]
pub(crate) fn with_cargo_artifact_dir_overrides<T>(
    target_dir: PathBuf,
    build_dir: PathBuf,
    operation: impl FnOnce() -> T,
) -> T {
    TEST_CARGO_TARGET_DIR_OVERRIDE.with_borrow_mut(|slot| {
        assert!(
            slot.is_none(),
            "test cargo target dir override should not already be installed"
        );
        *slot = Some(target_dir);
    });
    TEST_CARGO_BUILD_DIR_OVERRIDE.with_borrow_mut(|slot| {
        assert!(
            slot.is_none(),
            "test cargo build dir override should not already be installed"
        );
        *slot = Some(build_dir);
    });

    let outcome = operation();

    TEST_CARGO_TARGET_DIR_OVERRIDE.with_borrow_mut(|slot| {
        *slot = None;
    });
    TEST_CARGO_BUILD_DIR_OVERRIDE.with_borrow_mut(|slot| {
        *slot = None;
    });

    outcome
}

#[cfg(test)]
pub(crate) fn with_process_env_passthrough_for_tests<T>(operation: impl FnOnce() -> T) -> T {
    TEST_IGNORE_PROCESS_ENV.with_borrow_mut(|slot| {
        assert!(
            *slot,
            "test process env passthrough should not already be enabled"
        );
        *slot = false;
    });

    let outcome = operation();

    TEST_IGNORE_PROCESS_ENV.with_borrow_mut(|slot| {
        *slot = true;
    });

    outcome
}

#[cfg(test)]
fn test_cargo_target_dir_override() -> Option<PathBuf> {
    TEST_CARGO_TARGET_DIR_OVERRIDE.with_borrow(|slot| slot.clone())
}

#[cfg(test)]
fn test_cargo_build_dir_override() -> Option<PathBuf> {
    TEST_CARGO_BUILD_DIR_OVERRIDE.with_borrow(|slot| slot.clone())
}

#[cfg(test)]
fn test_ignores_process_env() -> bool {
    TEST_IGNORE_PROCESS_ENV.with_borrow(|slot| *slot)
}
