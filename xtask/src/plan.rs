use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use crate::model::{CommandSpec, DynResult};

/// Builds the ordered command plan for `cargo xtask check`.
pub fn check_plan(repo_root: &Path) -> DynResult<Vec<CommandSpec>> {
    let scripts = shell_script_paths(repo_root)?;
    let semver_release_type = semver_release_type(repo_root)?;
    let mut plan = Vec::new();

    if !scripts.is_empty() {
        plan.push(CommandSpec::new(
            "bash",
            std::iter::once("-n".to_owned()).chain(path_strings(&scripts)),
            false,
            false,
        ));
        plan.push(CommandSpec::new(
            "shellcheck",
            path_strings(&scripts),
            false,
            false,
        ));
    }

    plan.push(CommandSpec::new("cargo", ["fmt", "--check"], false, false));
    plan.push(CommandSpec::new(
        "cargo",
        [
            "clippy",
            "--workspace",
            "--all-targets",
            "--all-features",
            "--locked",
            "--",
            "-D",
            "warnings",
        ],
        false,
        true,
    ));
    plan.push(CommandSpec::new(
        "cargo",
        [
            "outdated",
            "--workspace",
            "--root-deps-only",
            "--exit-code",
            "1",
        ],
        false,
        false,
    ));
    plan.push(CommandSpec::new(
        "cargo",
        ["audit", "-D", "warnings"],
        false,
        false,
    ));
    plan.push(CommandSpec::new(
        "cargo",
        ["deny", "check", "advisories", "bans", "licenses", "sources"],
        false,
        false,
    ));
    plan.push(CommandSpec::new(
        "cargo",
        [
            "semver-checks",
            "--manifest-path",
            core_manifest_path(repo_root).to_string_lossy().as_ref(),
            "--baseline-root",
            semver_baseline_path(repo_root).to_string_lossy().as_ref(),
            "--release-type",
            semver_release_type.as_str(),
            "--all-features",
        ],
        false,
        true,
    ));
    plan.push(CommandSpec::new(
        "cargo",
        [
            "check",
            "--manifest-path",
            fuzz_manifest_path(repo_root).to_string_lossy().as_ref(),
            "--bins",
            "--locked",
        ],
        false,
        true,
    ));
    plan.push(CommandSpec::new(
        "cargo",
        [
            "nextest",
            "run",
            "--workspace",
            "--all-targets",
            "--all-features",
            "--locked",
        ],
        false,
        true,
    ));
    plan.push(CommandSpec::new(
        "cargo",
        ["test", "--workspace", "--doc", "--all-features", "--locked"],
        false,
        true,
    ));
    plan.push(CommandSpec::new(
        "cargo",
        [
            "build",
            "--profile",
            "dist",
            "-p",
            "htmlcut-cli",
            "--bin",
            "htmlcut",
            "--locked",
        ],
        false,
        true,
    ));
    plan.push(CommandSpec::new(
        release_binary_path(repo_root),
        ["--version"],
        true,
        false,
    ));

    Ok(plan)
}

/// Lists shell scripts that should be syntax-checked and linted by the maintainer gate.
pub fn shell_script_paths(repo_root: &Path) -> DynResult<Vec<PathBuf>> {
    let scripts_dir = repo_root.join("scripts");
    if !scripts_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut scripts = fs::read_dir(&scripts_dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension() == Some(OsStr::new("sh")))
        .collect::<Vec<_>>();
    scripts.sort();
    Ok(scripts)
}

/// Reads the workspace version from the root manifest.
pub fn workspace_version(repo_root: &Path) -> DynResult<String> {
    workspace_version_from_manifest(&fs::read_to_string(repo_root.join("Cargo.toml"))?)
}

/// Infers the semver release type that `cargo semver-checks` should enforce.
pub fn semver_release_type(repo_root: &Path) -> DynResult<String> {
    let workspace_version = workspace_version(repo_root)?;
    let baseline_manifest_path = semver_baseline_path(repo_root).join("Cargo.toml");
    let baseline_manifest = fs::read_to_string(baseline_manifest_path)?;
    let baseline_version = workspace_version_from_manifest(&baseline_manifest)?;
    Ok(semver_release_type_from_versions(
        &workspace_version,
        &baseline_version,
    ))
}

/// Extracts the workspace version from a root `Cargo.toml` string.
pub fn workspace_version_from_manifest(manifest: &str) -> DynResult<String> {
    manifest
        .lines()
        .find_map(|line| {
            line.strip_prefix("version = \"")
                .and_then(|line| line.strip_suffix('"'))
                .map(ToOwned::to_owned)
        })
        .ok_or_else(|| "workspace version not found in Cargo.toml".into())
}

/// Maps the workspace and baseline versions to the semver release type checked in CI.
pub fn semver_release_type_from_versions(
    workspace_version: &str,
    baseline_version: &str,
) -> String {
    if workspace_version == baseline_version {
        "minor".to_owned()
    } else {
        "major".to_owned()
    }
}

/// Adds a minimal workspace stub to isolated manifests used by the semver baseline flow.
pub fn with_workspace_stub(cargo_toml: &str) -> String {
    if cargo_toml.contains("\n[workspace]\n") {
        return cargo_toml.to_owned();
    }

    format!("{cargo_toml}\n[workspace]\n")
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
    repo_root.join("target").join("dist").join(binary_name())
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

/// Returns the dedicated fuzz-package manifest path.
pub fn fuzz_manifest_path(repo_root: &Path) -> PathBuf {
    repo_root.join("fuzz").join("Cargo.toml")
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

fn path_strings(paths: &[PathBuf]) -> impl Iterator<Item = String> + '_ {
    paths.iter().map(|path| path.to_string_lossy().into_owned())
}
