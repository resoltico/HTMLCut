use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use htmlcut_tempdir::tempdir;

use crate::model::{CommandSpec, CommandStdout, CommandToolchainEnv, DynResult};
use crate::{cargo_build_dir, cargo_target_dir, prepare_artifact_layout};

#[cfg(test)]
use std::cell::RefCell;

const DETACHED_LAUNCHER_ENV: &str = "HTMLCUT_XTASK_DETACHED_LAUNCHER";

#[cfg(test)]
type CaptureOverride = dyn FnMut(&Path, &CommandSpec) -> Option<DynResult<Vec<u8>>>;
#[cfg(test)]
type RunSpecOverride = dyn FnMut(&Path, &CommandSpec) -> Option<DynResult<()>>;

#[cfg(test)]
thread_local! {
    static CAPTURE_OVERRIDE: RefCell<Option<Box<CaptureOverride>>> = RefCell::new(None);
    static RUN_SPEC_OVERRIDE: RefCell<Option<Box<RunSpecOverride>>> = RefCell::new(None);
    static DETACHED_LAUNCHER_ENV_OVERRIDE: RefCell<Option<bool>> = const { RefCell::new(None) };
}

#[cfg(all(test, unix))]
thread_local! {
    static CURRENT_EXECUTABLE_OVERRIDE: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
}

/// Executes one maintainer command against the repository root.
pub fn run_spec(repo_root: &Path, spec: &CommandSpec) -> DynResult<()> {
    #[cfg(test)]
    if let Some(result) = run_spec_override_result(repo_root, spec) {
        return result;
    }

    let mut command = Command::new(&spec.program);
    command.current_dir(repo_root);
    command.args(&spec.args);
    command.stdin(Stdio::inherit());
    if spec.stdout == CommandStdout::Quiet {
        command.stdout(Stdio::null());
    } else {
        command.stdout(Stdio::inherit());
    }
    command.stderr(Stdio::inherit());
    apply_clang_override(&mut command, spec);
    apply_artifact_layout(&mut command, repo_root, spec)?;
    apply_command_environment(&mut command, spec);

    let status = command.status()?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("command failed with status {status}").into())
    }
}

/// Captures stdout for one maintainer command and fails when the command exits non-zero.
pub fn capture_command_output(repo_root: &Path, spec: &CommandSpec) -> DynResult<Vec<u8>> {
    #[cfg(test)]
    if let Some(result) = capture_override_result(repo_root, spec) {
        return result;
    }

    let mut command = Command::new(&spec.program);
    command.current_dir(repo_root);
    command.args(&spec.args);
    command.stdin(Stdio::null());
    command.stdout(Stdio::piped());
    command.stderr(Stdio::inherit());
    apply_clang_override(&mut command, spec);
    apply_artifact_layout(&mut command, repo_root, spec)?;
    apply_command_environment(&mut command, spec);

    let output = command.output()?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(format!("command failed with status {}", output.status).into())
    }
}

/// Lists existing maintained worktree paths via Git when the repository root is a Git worktree.
pub(crate) fn repo_worktree_files(repo_root: &Path) -> DynResult<Option<Vec<PathBuf>>> {
    if !repo_root.join(".git").exists() {
        return Ok(None);
    }

    let output = capture_command_output(
        repo_root,
        &CommandSpec::new(
            "git",
            [
                "ls-files",
                "--cached",
                "--others",
                "--exclude-standard",
                "-z",
            ],
            CommandStdout::Quiet,
            CommandToolchainEnv::Inherit,
        ),
    )?;
    Ok(Some(parse_repo_worktree_files(repo_root, &output)?))
}

/// Removes one directory tree when it exists and otherwise succeeds quietly.
pub fn remove_dir_if_exists(path: &Path) -> DynResult<()> {
    if path.exists() {
        fs::remove_dir_all(path)?;
    }

    Ok(())
}

/// Resolves the repository root that contains the `xtask` package.
pub fn repo_root() -> PathBuf {
    let mut repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let _popped = repo_root.pop();
    repo_root
}

/// Re-runs the current `xtask` executable from one detached temporary copy when the live
/// executable resides under the same mutable Cargo artifact roots that maintainer subcommands will
/// rebuild.
pub fn run_from_detached_launcher_if_needed(
    repo_root: &Path,
    args: &[OsString],
) -> DynResult<bool> {
    if detached_launcher_env_is_set() {
        return Ok(false);
    }

    let current_executable = current_executable_path()?;
    if !launcher_requires_detach(repo_root, &current_executable) {
        return Ok(false);
    }

    let detached_root = tempdir()?;
    let detached_executable =
        prepare_detached_executable_copy(&current_executable, detached_root.path())?;

    let mut command = Command::new(&detached_executable);
    command.current_dir(repo_root);
    command.args(args.iter().skip(1));
    command.stdin(Stdio::inherit());
    command.stdout(Stdio::inherit());
    command.stderr(Stdio::inherit());
    command.env(DETACHED_LAUNCHER_ENV, "1");

    let status = command.status()?;
    if status.success() {
        Ok(true)
    } else {
        Err(format!("detached xtask launcher exited with status {status}").into())
    }
}

fn current_executable_path() -> DynResult<PathBuf> {
    #[cfg(all(test, unix))]
    if let Some(override_path) = test_current_executable_override() {
        return Ok(override_path);
    }

    Ok(env::current_exe()?)
}

fn prepare_detached_executable_copy(
    current_executable: &Path,
    detached_root: &Path,
) -> DynResult<PathBuf> {
    let detached_executable =
        detached_root.join(current_executable.file_name().unwrap_or_default());
    fs::copy(current_executable, &detached_executable)?;
    let source_permissions = fs::metadata(current_executable)?.permissions();
    fs::set_permissions(&detached_executable, source_permissions)?;
    Ok(detached_executable)
}

fn detached_launcher_env_is_set() -> bool {
    #[cfg(test)]
    if let Some(override_value) = test_detached_launcher_env_override() {
        return override_value;
    }

    env::var_os(DETACHED_LAUNCHER_ENV).is_some()
}

fn launcher_requires_detach(repo_root: &Path, executable_path: &Path) -> bool {
    artifact_roots_requiring_detach(repo_root)
        .into_iter()
        .any(|artifact_root| executable_path.starts_with(artifact_root))
}

fn artifact_roots_requiring_detach(repo_root: &Path) -> Vec<PathBuf> {
    let managed_target_dir = cargo_target_dir(repo_root);
    let managed_build_dir = cargo_build_dir(repo_root);
    vec![
        managed_target_dir.clone(),
        managed_build_dir.clone(),
        managed_target_dir.join("debug"),
        managed_target_dir.join("release"),
        managed_target_dir.join("dist"),
        managed_build_dir.join("debug"),
        managed_build_dir.join("release"),
        managed_build_dir.join("dist"),
    ]
}

fn parse_repo_worktree_files(repo_root: &Path, output: &[u8]) -> DynResult<Vec<PathBuf>> {
    let mut paths = output
        .split(|byte| *byte == 0)
        .filter(|entry| !entry.is_empty())
        .map(std::str::from_utf8)
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(|relative| repo_root.join(relative))
        .filter(|path| path.exists())
        .collect::<Vec<_>>();
    paths.sort();
    paths.dedup();
    Ok(paths)
}

fn apply_clang_override(command: &mut Command, spec: &CommandSpec) {
    if spec.toolchain_env == CommandToolchainEnv::ForceClang {
        command.env("CC", "clang");
        command.env("CXX", "clang++");
    }
}

fn apply_artifact_layout(
    command: &mut Command,
    repo_root: &Path,
    spec: &CommandSpec,
) -> DynResult<()> {
    let Some((target_dir, build_dir)) = prepare_artifact_layout(repo_root, spec.artifact_layout)?
    else {
        return Ok(());
    };

    command.env("CARGO_TARGET_DIR", target_dir);
    command.env("CARGO_BUILD_BUILD_DIR", build_dir);
    Ok(())
}

fn apply_command_environment(command: &mut Command, spec: &CommandSpec) {
    for (key, value) in &spec.env {
        command.env(key, value);
    }
}

#[cfg(test)]
pub(crate) fn command_environment_for_tests(
    spec: &CommandSpec,
) -> Vec<(std::ffi::OsString, Option<std::ffi::OsString>)> {
    let mut command = Command::new(&spec.program);
    apply_command_environment(&mut command, spec);
    command
        .get_envs()
        .map(|(key, value)| (key.to_owned(), value.map(std::ffi::OsString::from)))
        .collect()
}

#[cfg(test)]
fn capture_override_result(repo_root: &Path, spec: &CommandSpec) -> Option<DynResult<Vec<u8>>> {
    CAPTURE_OVERRIDE.with_borrow_mut(|override_fn| {
        override_fn
            .as_mut()
            .and_then(|override_fn| override_fn(repo_root, spec))
    })
}

#[cfg(test)]
fn run_spec_override_result(repo_root: &Path, spec: &CommandSpec) -> Option<DynResult<()>> {
    RUN_SPEC_OVERRIDE.with_borrow_mut(|override_fn| {
        override_fn
            .as_mut()
            .and_then(|override_fn| override_fn(repo_root, spec))
    })
}

#[cfg(test)]
pub(crate) fn with_capture_command_output_override<F, T>(
    override_fn: F,
    operation: impl FnOnce() -> T,
) -> T
where
    F: FnMut(&Path, &CommandSpec) -> Option<DynResult<Vec<u8>>> + 'static,
{
    CAPTURE_OVERRIDE.with_borrow_mut(|slot| {
        assert!(
            slot.is_none(),
            "capture override should not already be installed"
        );
        *slot = Some(Box::new(override_fn));
    });

    let outcome = operation();

    CAPTURE_OVERRIDE.with_borrow_mut(|slot| {
        *slot = None;
    });

    outcome
}

#[cfg(test)]
pub(crate) fn with_run_spec_override<F, T>(override_fn: F, operation: impl FnOnce() -> T) -> T
where
    F: FnMut(&Path, &CommandSpec) -> Option<DynResult<()>> + 'static,
{
    RUN_SPEC_OVERRIDE.with_borrow_mut(|slot| {
        assert!(
            slot.is_none(),
            "run-spec override should not already be installed"
        );
        *slot = Some(Box::new(override_fn));
    });

    let outcome = operation();

    RUN_SPEC_OVERRIDE.with_borrow_mut(|slot| {
        *slot = None;
    });

    outcome
}

#[cfg(test)]
pub(crate) fn launcher_requires_detach_for_tests(repo_root: &Path, executable_path: &Path) -> bool {
    launcher_requires_detach(repo_root, executable_path)
}

#[cfg(all(test, unix))]
pub(crate) fn current_executable_path_for_tests() -> DynResult<PathBuf> {
    current_executable_path()
}

#[cfg(all(test, unix))]
pub(crate) fn prepare_detached_executable_copy_for_tests(
    current_executable: &Path,
    detached_root: &Path,
) -> DynResult<PathBuf> {
    prepare_detached_executable_copy(current_executable, detached_root)
}

#[cfg(all(test, unix))]
pub(crate) fn with_current_executable_override<T>(
    override_path: PathBuf,
    operation: impl FnOnce() -> T,
) -> T {
    CURRENT_EXECUTABLE_OVERRIDE.with_borrow_mut(|slot| {
        assert!(
            slot.is_none(),
            "current executable override should not already be installed"
        );
        *slot = Some(override_path);
    });

    let outcome = operation();

    CURRENT_EXECUTABLE_OVERRIDE.with_borrow_mut(|slot| {
        *slot = None;
    });

    outcome
}

#[cfg(all(test, unix))]
fn test_current_executable_override() -> Option<PathBuf> {
    CURRENT_EXECUTABLE_OVERRIDE.with_borrow(|slot| slot.clone())
}

#[cfg(test)]
pub(crate) fn with_detached_launcher_env_override<T>(
    override_value: bool,
    operation: impl FnOnce() -> T,
) -> T {
    DETACHED_LAUNCHER_ENV_OVERRIDE.with_borrow_mut(|slot| {
        assert!(
            slot.is_none(),
            "detached launcher env override should not already be installed"
        );
        *slot = Some(override_value);
    });

    let outcome = operation();

    DETACHED_LAUNCHER_ENV_OVERRIDE.with_borrow_mut(|slot| {
        *slot = None;
    });

    outcome
}

#[cfg(test)]
fn test_detached_launcher_env_override() -> Option<bool> {
    DETACHED_LAUNCHER_ENV_OVERRIDE.with_borrow(|slot| *slot)
}
