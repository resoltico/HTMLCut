use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use crate::command_exec::{capture_command_output, repo_worktree_files};
use crate::model::{
    CommandArtifactLayout, CommandSpec, CommandStderr, CommandStdout, CommandToolchainEnv,
    DynResult,
};
use crate::{
    deny_check_command, ensure_deny_targets_match_release_targets, fuzz::FUZZ_PACKAGE_NAME,
    miri_contract_command, outdated_check_command,
};

use super::paths::{core_manifest_path, release_binary_path, semver_baseline_path};
use super::semver::semver_release_type;

const DEVCONTAINER_RELEVANT_PATHS: &[&str] = &[
    ".devcontainer",
    "scripts/validate-devcontainer.sh",
    "scripts/devcontainer-check.sh",
    "scripts/devcontainer-prepare-user-home.sh",
    "scripts/devcontainer-bootstrap.sh",
    "scripts/devcontainer-cli-helper.Dockerfile",
    "scripts/common.sh",
    "scripts/xtask.sh",
    "check.sh",
];

/// Builds the ordered command plan for `cargo xtask check`.
pub fn check_plan(repo_root: &Path) -> DynResult<Vec<CommandSpec>> {
    ensure_clean_semver_baseline(repo_root)?;
    ensure_deny_targets_match_release_targets(repo_root)?;
    let scripts = shell_script_paths(repo_root)?;
    let semver_release_type = semver_release_type(repo_root)?;
    let mut plan = Vec::new();

    plan.push(CommandSpec::new(
        "bash",
        std::iter::once("-n".to_owned()).chain(path_strings(&scripts)),
        CommandStdout::Inherit,
        CommandToolchainEnv::Inherit,
    ));
    plan.push(CommandSpec::new(
        "shellcheck",
        path_strings(&scripts),
        CommandStdout::Inherit,
        CommandToolchainEnv::Inherit,
    ));

    if should_run_devcontainer_validation(repo_root)? {
        plan.push(devcontainer_validation_command(repo_root));
    }

    plan.push(format_check_command());
    plan.push(
        CommandSpec::new(
            "cargo",
            [
                "clippy",
                "-p",
                "htmlcut-core",
                "--lib",
                "--tests",
                "--locked",
                "--no-deps",
                "--",
                "-D",
                "warnings",
            ],
            CommandStdout::Inherit,
            CommandToolchainEnv::Inherit,
        )
        .with_artifact_layout(CommandArtifactLayout::ManagedWorkspace),
    );
    plan.push(
        CommandSpec::new(
            "cargo",
            [
                "test",
                "-p",
                "htmlcut-core",
                "--lib",
                "--no-default-features",
                "--locked",
            ],
            CommandStdout::Inherit,
            CommandToolchainEnv::Inherit,
        )
        .with_artifact_layout(CommandArtifactLayout::ManagedWorkspace),
    );
    plan.push(miri_contract_command());
    plan.push(workspace_clippy_command());
    plan.push(workspace_outdated_command());
    plan.push(workspace_audit_command());
    plan.push(deny_check_command(repo_root)?);
    plan.push(semver_check_command(repo_root, &semver_release_type));
    plan.push(
        CommandSpec::new(
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
            CommandStdout::Inherit,
            CommandToolchainEnv::Inherit,
        )
        .with_artifact_layout(CommandArtifactLayout::ManagedWorkspace),
    );
    plan.push(
        CommandSpec::new(
            "cargo",
            ["test", "--workspace", "--doc", "--all-features", "--locked"],
            CommandStdout::Inherit,
            CommandToolchainEnv::Inherit,
        )
        .with_artifact_layout(CommandArtifactLayout::ManagedWorkspace),
    );
    plan.push(
        CommandSpec::new(
            "cargo",
            ["doc", "--workspace", "--no-deps", "--locked"],
            CommandStdout::Inherit,
            CommandToolchainEnv::Inherit,
        )
        .with_artifact_layout(CommandArtifactLayout::ManagedWorkspace),
    );
    plan.push(
        CommandSpec::new(
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
            CommandStdout::Inherit,
            CommandToolchainEnv::Inherit,
        )
        .with_artifact_layout(CommandArtifactLayout::ManagedWorkspace),
    );
    plan.push(CommandSpec::new(
        release_binary_path(repo_root),
        ["--version"],
        CommandStdout::Quiet,
        CommandToolchainEnv::Inherit,
    ));

    Ok(plan)
}

/// Builds the curated Rust gate executed by cross-platform CI jobs.
pub fn ci_rust_gate_plan(repo_root: &Path) -> DynResult<Vec<CommandSpec>> {
    ensure_clean_semver_baseline(repo_root)?;
    ensure_deny_targets_match_release_targets(repo_root)?;
    let semver_release_type = semver_release_type(repo_root)?;

    let mut plan = vec![format_check_command(), workspace_clippy_command()];
    plan.extend(all_features_test_specs());
    plan.push(workspace_outdated_command());
    plan.push(workspace_audit_command());
    plan.push(deny_check_command(repo_root)?);
    plan.push(semver_check_command(repo_root, &semver_release_type));
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

fn ensure_clean_semver_baseline(repo_root: &Path) -> DynResult<()> {
    if !repo_root.join(".git").exists() {
        return Ok(());
    }

    let baseline_path = semver_baseline_path(repo_root);
    let baseline_arg = baseline_path
        .strip_prefix(repo_root)
        .unwrap_or(baseline_path.as_path())
        .to_string_lossy()
        .into_owned();
    let status_spec = CommandSpec::new(
        "git",
        [
            "status",
            "--porcelain=1",
            "--untracked-files=all",
            "--",
            baseline_arg.as_str(),
        ],
        CommandStdout::Quiet,
        CommandToolchainEnv::Inherit,
    );
    let output = capture_command_output(repo_root, &status_spec)?;
    if output.is_empty() {
        return Ok(());
    }

    let mut dirty_entries = String::from_utf8(output)?
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    dirty_entries.sort();

    Err(format!(
        "semver baseline {} is dirty. Restore it to the last published snapshot before running `cargo xtask check`.\n{}",
        baseline_arg,
        dirty_entries.join("\n")
    )
    .into())
}

fn path_strings(paths: &[PathBuf]) -> impl Iterator<Item = String> + '_ {
    paths.iter().map(|path| path.to_string_lossy().into_owned())
}

fn devcontainer_validation_command(repo_root: &Path) -> CommandSpec {
    CommandSpec::new(
        "bash",
        [repo_root
            .join("scripts")
            .join("validate-devcontainer.sh")
            .to_string_lossy()
            .into_owned()],
        CommandStdout::Inherit,
        CommandToolchainEnv::Inherit,
    )
}

fn should_run_devcontainer_validation(repo_root: &Path) -> DynResult<bool> {
    if !repo_root.join(".git").exists() {
        return Ok(false);
    }

    let changed_output = capture_command_output(
        repo_root,
        &CommandSpec::new(
            "git",
            devcontainer_changed_file_args(repo_root)?,
            CommandStdout::Quiet,
            CommandToolchainEnv::Inherit,
        ),
    )?;
    if !changed_output.is_empty() {
        return Ok(true);
    }

    let untracked_output = capture_command_output(
        repo_root,
        &CommandSpec::new(
            "git",
            devcontainer_untracked_file_args(),
            CommandStdout::Quiet,
            CommandToolchainEnv::Inherit,
        ),
    )?;
    Ok(!untracked_output.is_empty())
}

fn devcontainer_changed_file_args(repo_root: &Path) -> DynResult<Vec<String>> {
    let merge_base_spec = CommandSpec::new(
        "git",
        ["merge-base", "HEAD", "origin/main"],
        CommandStdout::Quiet,
        CommandToolchainEnv::Inherit,
    )
    .with_stderr(CommandStderr::Quiet);
    let merge_base = capture_command_output(repo_root, &merge_base_spec)
        .ok()
        .and_then(|output| String::from_utf8(output).ok())
        .map(|text| text.trim().to_owned())
        .filter(|text| !text.is_empty());

    let mut args = vec!["diff".to_owned(), "--name-only".to_owned(), "-z".to_owned()];
    if let Some(merge_base) = merge_base {
        args.push(merge_base);
    } else {
        args.push("HEAD".to_owned());
    }
    args.push("--".to_owned());
    args.extend(
        DEVCONTAINER_RELEVANT_PATHS
            .iter()
            .map(|path| (*path).to_owned()),
    );
    Ok(args)
}

fn devcontainer_untracked_file_args() -> Vec<String> {
    let mut args = vec![
        "ls-files".to_owned(),
        "--others".to_owned(),
        "--exclude-standard".to_owned(),
        "-z".to_owned(),
        "--".to_owned(),
    ];
    args.extend(
        DEVCONTAINER_RELEVANT_PATHS
            .iter()
            .map(|path| (*path).to_owned()),
    );
    args
}

fn format_check_command() -> CommandSpec {
    CommandSpec::new(
        "cargo",
        ["fmt", "--check"],
        CommandStdout::Inherit,
        CommandToolchainEnv::Inherit,
    )
    .with_artifact_layout(CommandArtifactLayout::ManagedWorkspace)
}

fn workspace_clippy_command() -> CommandSpec {
    CommandSpec::new(
        "cargo",
        [
            "clippy",
            "--workspace",
            "--all-targets",
            "--all-features",
            "--locked",
            "--no-deps",
            "--",
            "-D",
            "warnings",
        ],
        CommandStdout::Inherit,
        CommandToolchainEnv::Inherit,
    )
    .with_artifact_layout(CommandArtifactLayout::ManagedWorkspace)
}

fn core_all_features_lib_test_command() -> CommandSpec {
    CommandSpec::new(
        "cargo",
        [
            "test",
            "-p",
            "htmlcut-core",
            "--lib",
            "--all-features",
            "--locked",
        ],
        CommandStdout::Inherit,
        CommandToolchainEnv::Inherit,
    )
    .with_artifact_layout(CommandArtifactLayout::ManagedWorkspace)
}

fn workspace_outdated_command() -> CommandSpec {
    outdated_check_command()
}

fn workspace_audit_command() -> CommandSpec {
    CommandSpec::new(
        "cargo",
        ["audit", "-D", "warnings"],
        CommandStdout::Inherit,
        CommandToolchainEnv::Inherit,
    )
    .with_artifact_layout(CommandArtifactLayout::ManagedWorkspace)
}

fn semver_check_command(repo_root: &Path, semver_release_type: &str) -> CommandSpec {
    CommandSpec::new(
        "cargo",
        [
            "semver-checks",
            "--manifest-path",
            core_manifest_path(repo_root).to_string_lossy().as_ref(),
            "--baseline-root",
            semver_baseline_path(repo_root).to_string_lossy().as_ref(),
            "--release-type",
            semver_release_type,
            "--all-features",
        ],
        CommandStdout::Inherit,
        CommandToolchainEnv::Inherit,
    )
    .with_artifact_layout(CommandArtifactLayout::ManagedWorkspace)
}

fn is_maintained_shell_script(repo_root: &Path, path: &Path) -> bool {
    let Ok(relative) = path.strip_prefix(repo_root) else {
        return false;
    };

    relative == Path::new("check.sh")
        || (relative.parent() == Some(Path::new("scripts"))
            && relative.extension() == Some(OsStr::new("sh")))
}

fn all_features_test_specs() -> Vec<CommandSpec> {
    vec![
        core_all_features_lib_test_command(),
        CommandSpec::new(
            "cargo",
            [
                "test",
                "-p",
                "htmlcut-cli",
                "--lib",
                "--tests",
                "--all-features",
                "--locked",
            ],
            CommandStdout::Inherit,
            CommandToolchainEnv::Inherit,
        )
        .with_artifact_layout(CommandArtifactLayout::ManagedWorkspace),
        CommandSpec::new(
            "cargo",
            [
                "nextest",
                "run",
                "-p",
                "htmlcut-tempdir",
                "--lib",
                "--tests",
                "--locked",
            ],
            CommandStdout::Inherit,
            CommandToolchainEnv::Inherit,
        )
        .with_artifact_layout(CommandArtifactLayout::ManagedWorkspace),
    ]
}

#[cfg(test)]
pub(crate) fn is_maintained_shell_script_for_tests(repo_root: &Path, path: &Path) -> bool {
    is_maintained_shell_script(repo_root, path)
}

#[cfg(test)]
pub(crate) fn devcontainer_validation_command_for_tests(repo_root: &Path) -> CommandSpec {
    devcontainer_validation_command(repo_root)
}

#[cfg(test)]
pub(crate) fn should_run_devcontainer_validation_for_tests(repo_root: &Path) -> DynResult<bool> {
    should_run_devcontainer_validation(repo_root)
}

#[cfg(test)]
pub(crate) fn devcontainer_changed_file_args_for_tests(repo_root: &Path) -> DynResult<Vec<String>> {
    devcontainer_changed_file_args(repo_root)
}

#[cfg(test)]
pub(crate) fn devcontainer_untracked_file_args_for_tests() -> Vec<String> {
    devcontainer_untracked_file_args()
}
