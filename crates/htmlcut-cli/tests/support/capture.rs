// SPDX-License-Identifier: MPL-2.0
use std::{
    process::{Command, Output},
    time::{Duration, Instant},
};
// Other native suites use fill_pipe; capture only needs its nonblocking setup.
#[allow(dead_code)]
#[path = "../../src/test_pipe.rs"]
mod test_pipe;

pub enum Stdin<'a> {
    Closed,
    Bytes(&'a [u8]),
    Held,
}

// Files prevent pipe backpressure; both sampling and final sizes are bounded before allocation.
pub fn finite_capture(
    command: &mut Command,
    timeout: Duration,
    cap: u64,
    input: Stdin<'_>,
) -> Result<Output, String> {
    let root = tempfile::tempdir().map_err(|e| e.to_string())?;
    let stdout_path = root.path().join("stdout");
    let stderr_path = root.path().join("stderr");
    let stdout = std::fs::File::create(&stdout_path).map_err(|e| e.to_string())?;
    let stderr = std::fs::File::create(&stderr_path).map_err(|e| e.to_string())?;
    // Preload pipe bytes before creating a child. Nonblocking writes either fit
    // or refuse here; no writer thread or unbounded write can outlive ownership.
    let (stdin, _producer) = match input {
        Stdin::Closed => (std::process::Stdio::null(), None),
        Stdin::Bytes(bytes) => {
            use std::io::Write;
            let (reader, mut writer) = std::io::pipe().map_err(|e| e.to_string())?;
            test_pipe::set_pipe_blocking(&writer, false);
            writer
                .write_all(bytes)
                .map_err(|e| format!("test stdin cannot be preloaded without blocking: {e}"))?;
            drop(writer);
            (reader.into(), None)
        }
        Stdin::Held => {
            let (reader, writer) = std::io::pipe().map_err(|e| e.to_string())?;
            (reader.into(), Some(writer))
        }
    };
    let mut child = command
        .stdin(stdin)
        .stdout(stdout)
        .stderr(stderr)
        .spawn()
        .map_err(|e| e.to_string())?;
    let deadline = Instant::now() + timeout;
    let sizes = || -> Result<bool, String> {
        Ok(std::fs::metadata(&stdout_path)
            .map_err(|e| e.to_string())?
            .len()
            <= cap
            && std::fs::metadata(&stderr_path)
                .map_err(|e| e.to_string())?
                .len()
                <= cap)
    };
    let failure = loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !sizes()? {
                    return Err("capture output bound exceeded".into());
                }
                return Ok(Output {
                    status,
                    stdout: std::fs::read(&stdout_path).map_err(|e| e.to_string())?,
                    stderr: std::fs::read(&stderr_path).map_err(|e| e.to_string())?,
                });
            }
            Err(e) => break format!("capture status failed: {e}"),
            Ok(None) => (),
        }
        if !sizes().unwrap_or(false) {
            break "capture output bound exceeded".into();
        }
        if Instant::now() >= deadline {
            break "capture deadline exceeded".into();
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    let kill = child.kill();
    let cleanup_deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return Err(failure),
            _ if Instant::now() >= cleanup_deadline => {
                let retained = root.keep();
                return Err(format!(
                    "{failure}; cleanup unconfirmed ({kill:?}); capture retained at {retained:?}"
                ));
            }
            _ => std::thread::sleep(Duration::from_millis(5)),
        }
    }
}
