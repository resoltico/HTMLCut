use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::model::{CommandSpec, DynResult};

#[cfg(test)]
use std::cell::RefCell;

#[cfg(test)]
type CaptureOverride = dyn FnMut(&Path, &CommandSpec) -> Option<DynResult<Vec<u8>>>;

#[cfg(test)]
thread_local! {
    static CAPTURE_OVERRIDE: RefCell<Option<Box<CaptureOverride>>> = RefCell::new(None);
}

/// Executes one maintainer command against the repository root.
pub fn run_spec(repo_root: &Path, spec: &CommandSpec) -> DynResult<()> {
    let mut command = Command::new(&spec.program);
    command.current_dir(repo_root);
    command.args(&spec.args);
    command.stdin(Stdio::inherit());
    if spec.quiet_stdout {
        command.stdout(Stdio::null());
    } else {
        command.stdout(Stdio::inherit());
    }
    command.stderr(Stdio::inherit());
    apply_clang_override(&mut command, spec);

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
            true,
            false,
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
    if spec.force_clang {
        command.env("CC", "clang");
        command.env("CXX", "clang++");
    }
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
