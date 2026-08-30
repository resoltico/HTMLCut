//! Streaming evidence analysis and human rendering for maintainer-gate commands.

use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::Path;

use crate::model::CommandSpec;

use super::model::{GateCommand, GateStream, GateWarning};

pub(super) const FAILURE_TAIL_BYTES: usize = 8 * 1024;

pub(super) fn command_document(spec: &CommandSpec) -> GateCommand {
    GateCommand {
        program: spec.program.display().to_string(),
        args: spec.args.clone(),
        environment_keys: spec.env.keys().cloned().collect(),
    }
}

pub(super) fn render_command(spec: &CommandSpec) -> String {
    std::iter::once(spec.program.display().to_string())
        .chain(spec.args.iter().cloned())
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn warnings_from_output(stdout: &[u8], stderr: &[u8]) -> Vec<GateWarning> {
    [(GateStream::Stdout, stdout), (GateStream::Stderr, stderr)]
        .into_iter()
        .flat_map(|(stream, bytes)| {
            String::from_utf8_lossy(bytes)
                .lines()
                .map(str::trim)
                .filter(|line| line.starts_with("warning:") || line.starts_with("warning["))
                .map(move |line| GateWarning {
                    stream,
                    message: line.to_owned(),
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

pub(super) fn warnings_from_logs(
    stdout_path: &Path,
    stderr_path: &Path,
) -> std::io::Result<Vec<GateWarning>> {
    [
        (GateStream::Stdout, stdout_path),
        (GateStream::Stderr, stderr_path),
    ]
    .into_iter()
    .map(|(stream, path)| warnings_from_log(stream, path))
    .collect::<std::io::Result<Vec<_>>>()
    .map(|warnings| warnings.into_iter().flatten().collect())
}

fn warnings_from_log(stream: GateStream, path: &Path) -> std::io::Result<Vec<GateWarning>> {
    BufReader::new(File::open(path)?)
        .split(b'\n')
        .map(|line| {
            let line = line?;
            let line = String::from_utf8_lossy(&line);
            Ok(
                (line.trim().starts_with("warning:") || line.trim().starts_with("warning[")).then(
                    || GateWarning {
                        stream,
                        message: line.trim().to_owned(),
                    },
                ),
            )
        })
        .filter_map(|warning| warning.transpose())
        .collect()
}

pub(super) fn log_byte_count(path: &Path) -> std::io::Result<u64> {
    fs::metadata(path).map(|metadata| metadata.len())
}

pub(super) fn combined_failure_tail(stdout: &[u8], stderr: &[u8]) -> String {
    let mut combined = Vec::new();
    if !stdout.is_empty() {
        combined.extend_from_slice(b"stdout:\n");
        combined.extend_from_slice(stdout);
    }
    if !stderr.is_empty() {
        if !combined.is_empty() {
            combined.push(b'\n');
        }
        combined.extend_from_slice(b"stderr:\n");
        combined.extend_from_slice(stderr);
    }
    bounded_tail(&combined)
}

pub(super) fn combined_failure_tail_from_logs(
    stdout_path: &Path,
    stderr_path: &Path,
) -> std::io::Result<String> {
    let stdout = log_tail(stdout_path)?;
    let stderr = log_tail(stderr_path)?;
    Ok(combined_failure_tail(&stdout, &stderr))
}

pub(super) fn bounded_tail(bytes: &[u8]) -> String {
    let start = bytes.len().saturating_sub(FAILURE_TAIL_BYTES);
    String::from_utf8_lossy(&bytes[start..]).into_owned()
}

fn log_tail(path: &Path) -> std::io::Result<Vec<u8>> {
    let mut file = File::open(path)?;
    let length = file.metadata()?.len();
    let tail_start = length.saturating_sub(FAILURE_TAIL_BYTES as u64);
    file.seek(SeekFrom::Start(tail_start))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    Ok(bytes)
}

pub(super) fn replay_stream(name: &str, bytes: &[u8], stderr: bool) {
    if bytes.is_empty() {
        return;
    }
    let text = String::from_utf8_lossy(bytes);
    if stderr {
        eprintln!("--- {name} ---\n{text}");
    } else {
        println!("--- {name} ---\n{text}");
    }
}

pub(super) fn replay_log_stream(name: &str, path: &Path, stderr: bool) {
    match fs::read(path) {
        Ok(bytes) => replay_stream(name, &bytes, stderr),
        Err(error) => eprintln!(
            "could not replay retained {name} log {}: {error}",
            path.display()
        ),
    }
}

pub(super) fn render_stream(stream: GateStream) -> &'static str {
    match stream {
        GateStream::Stdout => "stdout",
        GateStream::Stderr => "stderr",
    }
}
