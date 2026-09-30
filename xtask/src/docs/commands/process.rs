//! Bounded invocations of the once-built CLI; no supported CLI Rust API.

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
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
    let target = process_root(&root, "CARGO_TARGET_DIR", crate::cargo_target_dir(&root));
    let build = process_root(
        &root,
        "CARGO_BUILD_BUILD_DIR",
        crate::cargo_build_dir(&root),
    );
    let path = target
        .join("debug")
        .join(format!("htmlcut{}", std::env::consts::EXE_SUFFIX));
    let mut command = Command::new("cargo");
    command
        .current_dir(&root)
        .args(["build", "--locked", "-p", "htmlcut-cli", "--bin", "htmlcut"])
        // Real subprocess builds must honor actual mutation/coverage/container roots,
        // even when this module is compiled inside an otherwise isolated unit suite.
        .env("CARGO_TARGET_DIR", target)
        .env("CARGO_BUILD_BUILD_DIR", build)
        .env_remove("MAKEFLAGS")
        .env_remove("MFLAGS")
        .env_remove("CARGO_MAKEFLAGS");
    build_with(&mut command, path)
}

fn process_root(root: &Path, key: &str, fallback: PathBuf) -> PathBuf {
    override_root(root, std::env::var_os(key).as_deref(), fallback)
}

fn override_root(root: &Path, value: Option<&std::ffi::OsStr>, fallback: PathBuf) -> PathBuf {
    value
        .filter(|value| !value.is_empty())
        .map(|value| root.join(value))
        .unwrap_or(fallback)
}

fn build_with(command: &mut Command, path: PathBuf) -> io::Result<PathBuf> {
    let output = capture(command, None, Duration::from_secs(600))?;
    if output.code != 0 || !path.is_file() {
        return Err(io::Error::other(
            "Could not build the maintained CLI for documentation examples.",
        ));
    }
    Ok(path)
}

pub(super) fn invoke(
    tokens: &[String],
    directory: &Path,
    input: &[u8],
    fixture_url: &str,
) -> io::Result<Capture> {
    let mut command = Command::new(binary()?);
    command.current_dir(directory).args(tokens.iter().skip(1));
    command.env("HTMLCUT_SOURCE_URL", fixture_url);
    capture(&mut command, Some(input), Duration::from_secs(30))
}

// Reap on every return, including stdin and status errors. Child's own Drop does not
// terminate a process, so a fallible adapter must own this cleanup explicitly.
trait Lifecycle {
    fn terminate(&mut self) -> io::Result<()>;
    fn reap(&mut self) -> io::Result<()>;
}
impl Lifecycle for Child {
    fn terminate(&mut self) -> io::Result<()> {
        self.kill()
    }
    fn reap(&mut self) -> io::Result<()> {
        self.wait().map(|_| ())
    }
}
struct Running<P: Lifecycle>(P);
impl<P: Lifecycle> Drop for Running<P> {
    fn drop(&mut self) {
        let _ = self.0.terminate();
        let _ = self.0.reap();
    }
}

fn capture(command: &mut Command, input: Option<&[u8]>, timeout: Duration) -> io::Result<Capture> {
    if input.is_some_and(|input| input.len() > 4096) {
        return Err(io::Error::other(
            "Documentation stdin fixture exceeded its bound.",
        ));
    }
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
    let mut running = Running(command.spawn()?);
    let child = &mut running.0;
    if let Some(input) = input {
        write_fixture(&mut child.stdin.take().unwrap(), input)?;
    }
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        let oversized = capture_exceeded(&stdout_path, &stderr_path);
        if oversized || started.elapsed() >= timeout {
            return Err(io::Error::other(
                "Documentation CLI exceeded its capture or time budget.",
            ));
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    finish(status, &stdout_path, &stderr_path)
}

fn capture_exceeded(stdout: &Path, stderr: &Path) -> bool {
    [stdout, stderr]
        .into_iter()
        .any(|path| fs::metadata(path).is_ok_and(|metadata| metadata.len() > MAX_CAPTURE_BYTES))
}

fn write_fixture(writer: &mut impl Write, input: &[u8]) -> io::Result<()> {
    match writer.write_all(input) {
        // Commands such as describe/schema do not consume a source. If they close
        // stdin early, still collect their actual exit status and output below.
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        result => result,
    }
}

fn finish(status: ExitStatus, stdout_path: &Path, stderr_path: &Path) -> io::Result<Capture> {
    for path in [stdout_path, stderr_path] {
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

#[cfg(test)]
#[path = "process/tests.rs"]
mod tests;
