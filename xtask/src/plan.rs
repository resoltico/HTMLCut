use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use crate::model::{CommandSpec, DynResult};
use crate::{package_version_from_manifest, workspace_version};

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
            "fmt",
            "--check",
            "--manifest-path",
            fuzz_manifest_path(repo_root).to_string_lossy().as_ref(),
        ],
        false,
        false,
    ));
    plan.push(CommandSpec::new(
        "cargo",
        ["test", "-p", "xtask", "--lib", "--locked"],
        false,
        false,
    ));
    plan.push(CommandSpec::new(
        "cargo",
        [
            "test",
            "-p",
            "htmlcut-core",
            "--lib",
            "--locked",
            "contract_lint",
        ],
        false,
        true,
    ));
    plan.push(CommandSpec::new(
        "cargo",
        [
            "test",
            "-p",
            "htmlcut-cli",
            "--lib",
            "--locked",
            "contract_lint",
        ],
        false,
        true,
    ));
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
            "clippy",
            "--manifest-path",
            fuzz_manifest_path(repo_root).to_string_lossy().as_ref(),
            "--bins",
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
        [
            "outdated",
            "--manifest-path",
            fuzz_manifest_path(repo_root).to_string_lossy().as_ref(),
            "--root-deps-only",
            "--exit-code",
            "1",
        ],
        false,
        false,
    ));
    plan.push(CommandSpec::new(
        "cargo",
        [
            "audit",
            "-D",
            "warnings",
            "--file",
            fuzz_lockfile_path(repo_root).to_string_lossy().as_ref(),
        ],
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
    let root_check = repo_root.join("check.sh");
    let scripts_dir = repo_root.join("scripts");
    let mut scripts = Vec::new();

    if root_check.is_file() {
        scripts.push(root_check);
    }

    if scripts_dir.is_dir() {
        scripts.extend(
            fs::read_dir(&scripts_dir)?
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| path.extension() == Some(OsStr::new("sh"))),
        );
    }

    scripts.sort();
    Ok(scripts)
}

/// Resolves the Cargo target directory, honoring `CARGO_TARGET_DIR` when present.
pub fn cargo_target_dir(repo_root: &Path) -> PathBuf {
    cargo_target_dir_from(
        repo_root,
        env::var_os("CARGO_TARGET_DIR").map(PathBuf::from),
    )
}

/// Infers the semver release type that `cargo semver-checks` should enforce.
pub fn semver_release_type(repo_root: &Path) -> DynResult<String> {
    let workspace_version = workspace_version(repo_root)?;
    let baseline_manifest_path = semver_baseline_path(repo_root).join("Cargo.toml");
    let baseline_manifest = fs::read_to_string(baseline_manifest_path)?;
    let baseline_version = package_version_from_manifest(&baseline_manifest)?;
    Ok(semver_release_type_from_versions(
        &workspace_version,
        &baseline_version,
    ))
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

/// Removes dev-dependency tables from a manifest used only for semver-baseline packaging.
pub fn strip_dev_dependency_tables(cargo_toml: &str) -> String {
    let mut sanitized = Vec::new();
    let mut skipping = false;

    for line in cargo_toml.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            skipping = trimmed.contains("dev-dependencies");
            if skipping {
                continue;
            }
        }

        if !skipping {
            sanitized.push(line);
        }
    }

    let mut result = sanitized.join("\n");
    if cargo_toml.ends_with('\n') {
        result.push('\n');
    }
    result
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

/// Returns whether a command spec is the semver-checks gate step.
pub fn is_semver_check_spec(spec: &CommandSpec) -> bool {
    spec.program == Path::new("cargo")
        && matches!(spec.args.first().map(String::as_str), Some("semver-checks"))
}

/// Returns the dedicated fuzz-package manifest path.
pub fn fuzz_manifest_path(repo_root: &Path) -> PathBuf {
    repo_root.join("fuzz").join("Cargo.toml")
}

/// Returns the dedicated fuzz-package lockfile path.
pub fn fuzz_lockfile_path(repo_root: &Path) -> PathBuf {
    repo_root.join("fuzz").join("Cargo.lock")
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
