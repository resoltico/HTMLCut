//! Bounded invocations of the once-built CLI; no supported CLI Rust API.

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

const MAX_CAPTURE_BYTES: u64 = 2 * 1024 * 1024;

pub(super) struct Capture {
    pub(super) code: i32,
    pub(super) stdout: Vec<u8>,
    pub(super) stderr: Vec<u8>,
}

fn binary() -> io::Result<PathBuf> {
    static BINARY: OnceLock<Result<PathBuf, String>> = OnceLock::new();
    BINARY
        .get_or_init(|| build().map_err(|error| error.to_string()))
        .clone()
        .map_err(io::Error::other)
}

fn build() -> io::Result<PathBuf> {
    let root = crate::command_exec::repo_root();
    let path = crate::cargo_target_dir(&root)
        .join("debug")
        .join(if cfg!(windows) {
            "htmlcut.exe"
        } else {
            "htmlcut"
        });
    let mut command = Command::new("cargo");
    command
        .current_dir(&root)
        .args(["build", "--locked", "-p", "htmlcut-cli", "--bin", "htmlcut"])
        .env_remove("MAKEFLAGS")
        .env_remove("MFLAGS")
        .env_remove("CARGO_MAKEFLAGS");
    let output = capture(&mut command, None, Duration::from_secs(600))?;
    if output.code != 0 || !path.is_file() {
        return Err(io::Error::other(
            "Could not build the maintained CLI for documentation examples.",
        ));
    }
    Ok(path)
}

pub(super) fn invoke(tokens: &[String], directory: &Path, input: &[u8]) -> io::Result<Capture> {
    let mut command = Command::new(binary()?);
    command.current_dir(directory).args(tokens.iter().skip(1));
    capture(&mut command, Some(input), Duration::from_secs(30))
}

fn capture(command: &mut Command, input: Option<&[u8]>, timeout: Duration) -> io::Result<Capture> {
    let root = htmlcut_tempdir::tempdir()?;
    let stdout_path = root.path().join("stdout");
    let stderr_path = root.path().join("stderr");
    command
        .stdout(Stdio::from(File::create(&stdout_path)?))
        .stderr(Stdio::from(File::create(&stderr_path)?))
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        });
    let mut child = command.spawn()?;
    if let Some(input) = input {
        if input.len() > 4096 {
            let _ = child.kill();
            let _ = child.wait();
            return Err(io::Error::other(
                "Documentation stdin fixture exceeded its bound.",
            ));
        }
        if let Err(error) = child.stdin.take().unwrap().write_all(input) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }
    }
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        let oversized = [&stdout_path, &stderr_path]
            .into_iter()
            .any(|path| fs::metadata(path).is_ok_and(|m| m.len() > MAX_CAPTURE_BYTES));
        if oversized || started.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Err(io::Error::other(
                "Documentation CLI exceeded its capture or time budget.",
            ));
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    for path in [&stdout_path, &stderr_path] {
        if fs::metadata(path)?.len() > MAX_CAPTURE_BYTES {
            return Err(io::Error::other(
                "Documentation CLI exceeded its capture budget.",
            ));
        }
    }
    Ok(Capture {
        code: status.code().unwrap_or(6),
        stdout: fs::read(stdout_path)?,
        stderr: fs::read(stderr_path)?,
    })
}
