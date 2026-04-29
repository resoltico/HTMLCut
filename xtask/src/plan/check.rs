use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use crate::command_exec::repo_worktree_files;
use crate::model::{CommandSpec, DynResult};
use crate::{deny_check_command, fuzz::FUZZ_PACKAGE_NAME};

use super::paths::{core_manifest_path, release_binary_path, semver_baseline_path};
use super::semver::semver_release_type;

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
            "clippy",
            "-p",
            "htmlcut-core",
            "--lib",
            "--tests",
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
            "test",
            "-p",
            "htmlcut-core",
            "--lib",
            "--no-default-features",
            "--locked",
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
    plan.push(deny_check_command(repo_root)?);
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
            "-p",
            FUZZ_PACKAGE_NAME,
            "--bins",
            "--features",
            "fuzzing",
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
            "--lib",
            "--tests",
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
        ["doc", "--workspace", "--no-deps", "--locked"],
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
    let mut scripts = if let Some(paths) = repo_worktree_files(repo_root)? {
        paths
            .into_iter()
            .filter(|path| is_maintained_shell_script(repo_root, path))
            .collect()
    } else {
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

        scripts
    };

    scripts.sort();
    Ok(scripts)
}

/// Returns whether a command spec is the semver-checks gate step.
pub fn is_semver_check_spec(spec: &CommandSpec) -> bool {
    spec.program == Path::new("cargo")
        && matches!(spec.args.first().map(String::as_str), Some("semver-checks"))
}

fn path_strings(paths: &[PathBuf]) -> impl Iterator<Item = String> + '_ {
    paths.iter().map(|path| path.to_string_lossy().into_owned())
}

fn is_maintained_shell_script(repo_root: &Path, path: &Path) -> bool {
    let Ok(relative) = path.strip_prefix(repo_root) else {
        return false;
    };

    relative == Path::new("check.sh")
        || (relative.parent() == Some(Path::new("scripts"))
            && relative.extension() == Some(OsStr::new("sh")))
}

#[cfg(test)]
pub(crate) fn is_maintained_shell_script_for_tests(repo_root: &Path, path: &Path) -> bool {
    is_maintained_shell_script(repo_root, path)
}
