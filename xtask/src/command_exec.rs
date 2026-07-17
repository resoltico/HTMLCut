use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;

use crate::model::{CommandSpec, CommandStderr, CommandStdout, CommandToolchainEnv, DynResult};
use crate::prepare_artifact_layout;

#[cfg(test)]
use std::cell::RefCell;

#[cfg(test)]
type CaptureOverride = dyn FnMut(&Path, &CommandSpec) -> Option<DynResult<Vec<u8>>>;
#[cfg(test)]
type RunSpecOverride = dyn FnMut(&Path, &CommandSpec) -> Option<DynResult<()>>;
#[cfg(test)]
type StreamWriteOverride = dyn FnMut(bool, &[u8]) -> DynResult<()>;

#[cfg(test)]
thread_local! {
    static CAPTURE_OVERRIDE: RefCell<Option<Box<CaptureOverride>>> = RefCell::new(None);
    static RUN_SPEC_OVERRIDE: RefCell<Option<Box<RunSpecOverride>>> = RefCell::new(None);
    static STREAM_WRITE_OVERRIDE: RefCell<Option<Box<StreamWriteOverride>>> = RefCell::new(None);
}

/// Executes one maintainer command against the repository root.
pub fn run_spec(repo_root: &Path, spec: &CommandSpec) -> DynResult<()> {
    #[cfg(test)]
    if let Some(result) = run_spec_override_result(repo_root, spec) {
        return result;
    }

    if let Some(index) = crate::gate_report::begin_command(spec) {
        return run_reported_spec(repo_root, spec, index);
    }

    let mut command = configured_command(repo_root, spec, Stdio::inherit())?;
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());
    let output = command.output()?;
    if output.status.success() {
        if spec.stdout == CommandStdout::Inherit {
            write_stdout(&output.stdout)?;
        }
        if spec.stderr == CommandStderr::Inherit {
            write_stderr(&output.stderr)?;
        }
        Ok(())
    } else {
        write_stdout(&output.stdout)?;
        write_stderr(&output.stderr)?;
        Err(command_failure(
            spec,
            output.status,
            &output.stdout,
            &output.stderr,
        ))
    }
}

/// Captures stdout for one maintainer command and fails when the command exits non-zero.
pub fn capture_command_output(repo_root: &Path, spec: &CommandSpec) -> DynResult<Vec<u8>> {
    #[cfg(test)]
    if let Some(result) = capture_override_result(repo_root, spec) {
        return result;
    }

    if let Some(index) = crate::gate_report::begin_command(spec) {
        return capture_reported_command_output(repo_root, spec, index);
    }

    let mut command = configured_command(repo_root, spec, Stdio::null())?;
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    let output = command.output()?;
    if output.status.success() {
        if spec.stderr == CommandStderr::Inherit {
            write_stderr(&output.stderr)?;
        }
        Ok(output.stdout)
    } else {
        write_stderr(&output.stderr)?;
        Err(command_failure(
            spec,
            output.status,
            &output.stdout,
            &output.stderr,
        ))
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

fn run_reported_spec(repo_root: &Path, spec: &CommandSpec, index: usize) -> DynResult<()> {
    let started = Instant::now();
    let mut command = configured_command(repo_root, spec, Stdio::inherit())?;
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());
    let output = match command.output() {
        Ok(output) => output,
        Err(error) => {
            let context = crate::gate_report::finish_command_spawn_failure(
                index,
                spec,
                &error,
                started.elapsed(),
            )
            .unwrap_or_else(|| format!("could not start command: {error}"));
            return Err(context.into());
        }
    };
    let context = crate::gate_report::finish_command(index, spec, &output, started.elapsed())
        .unwrap_or_else(|| format!("command failed with status {}", output.status));
    if output.status.success() && context.is_empty() {
        Ok(())
    } else {
        Err(context.into())
    }
}

fn capture_reported_command_output(
    repo_root: &Path,
    spec: &CommandSpec,
    index: usize,
) -> DynResult<Vec<u8>> {
    let started = Instant::now();
    let mut command = configured_command(repo_root, spec, Stdio::null())?;
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());
    let output = match command.output() {
        Ok(output) => output,
        Err(error) => {
            let context = crate::gate_report::finish_command_spawn_failure(
                index,
                spec,
                &error,
                started.elapsed(),
            )
            .unwrap_or_else(|| format!("could not start command: {error}"));
            return Err(context.into());
        }
    };
    let context = crate::gate_report::finish_command(index, spec, &output, started.elapsed())
        .unwrap_or_else(|| format!("command failed with status {}", output.status));
    if output.status.success() && context.is_empty() {
        Ok(output.stdout)
    } else {
        Err(context.into())
    }
}

fn configured_command(repo_root: &Path, spec: &CommandSpec, stdin: Stdio) -> DynResult<Command> {
    let mut command = Command::new(&spec.program);
    command.current_dir(repo_root);
    command.args(&spec.args);
    command.stdin(stdin);
    apply_clang_override(&mut command, spec);
    apply_artifact_layout(&mut command, repo_root, spec)?;
    apply_command_environment(&mut command, spec);
    Ok(command)
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

fn command_failure(
    spec: &CommandSpec,
    status: std::process::ExitStatus,
    stdout: &[u8],
    stderr: &[u8],
) -> crate::model::XtaskError {
    let mut message = format!(
        "command failed with status {status}: {}",
        render_command(spec)
    );
    let tail = combined_tail(stdout, stderr);
    if !tail.is_empty() {
        message.push_str("\n\n");
        message.push_str(&tail);
    }
    message.into()
}

fn render_command(spec: &CommandSpec) -> String {
    std::iter::once(spec.program.display().to_string())
        .chain(spec.args.iter().cloned())
        .collect::<Vec<_>>()
        .join(" ")
}

fn combined_tail(stdout: &[u8], stderr: &[u8]) -> String {
    const LIMIT: usize = 8 * 1024;
    let mut bytes = Vec::new();
    if !stdout.is_empty() {
        bytes.extend_from_slice(b"stdout:\n");
        bytes.extend_from_slice(stdout);
    }
    if !stderr.is_empty() {
        if !bytes.is_empty() {
            bytes.push(b'\n');
        }
        bytes.extend_from_slice(b"stderr:\n");
        bytes.extend_from_slice(stderr);
    }
    let start = bytes.len().saturating_sub(LIMIT);
    String::from_utf8_lossy(&bytes[start..]).into_owned()
}

fn write_stdout(bytes: &[u8]) -> DynResult<()> {
    #[cfg(test)]
    if let Some(result) = stream_write_override_result(false, bytes) {
        return result;
    }

    let mut stdout = io::stdout().lock();
    stdout.write_all(bytes)?;
    stdout.flush()?;
    Ok(())
}

fn write_stderr(bytes: &[u8]) -> DynResult<()> {
    #[cfg(test)]
    if let Some(result) = stream_write_override_result(true, bytes) {
        return result;
    }

    let mut stderr = io::stderr().lock();
    stderr.write_all(bytes)?;
    stderr.flush()?;
    Ok(())
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
pub(crate) fn command_failure_message_for_tests(
    spec: &CommandSpec,
    status: std::process::ExitStatus,
    stdout: &[u8],
    stderr: &[u8],
) -> String {
    command_failure(spec, status, stdout, stderr).to_string()
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
fn stream_write_override_result(stderr: bool, bytes: &[u8]) -> Option<DynResult<()>> {
    STREAM_WRITE_OVERRIDE.with_borrow_mut(|override_fn| {
        override_fn
            .as_mut()
            .map(|override_fn| override_fn(stderr, bytes))
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
pub(crate) fn with_stream_write_override<F, T>(override_fn: F, operation: impl FnOnce() -> T) -> T
where
    F: FnMut(bool, &[u8]) -> DynResult<()> + 'static,
{
    STREAM_WRITE_OVERRIDE.with_borrow_mut(|slot| {
        assert!(
            slot.is_none(),
            "stream-write override should not already be installed"
        );
        *slot = Some(Box::new(override_fn));
    });

    let outcome = operation();

    STREAM_WRITE_OVERRIDE.with_borrow_mut(|slot| {
        *slot = None;
    });

    outcome
}
