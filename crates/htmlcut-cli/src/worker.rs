// SPDX-License-Identifier: MPL-2.0
//! One-shot private execution boundary. Pipes never become public data channels.
use htmlcut_core::{
    CompiledPlan, ExtractionError, ExtractionPlan, PreparationLimits, PreparedDocument,
    SnapshotMetadata, SourceSnapshot,
};
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};
pub(crate) const MODE: &str = "HTMLCUT_PRIVATE_WORKER";
pub(crate) const MEMORY_BYTES: u64 = 1024 * 1024 * 1024;
const DEADLINE: Duration = Duration::from_secs(10);
const PLAN_FRAME_BYTES: usize = htmlcut_core::MAX_PLAN_BYTES * 6 + 16_384;
const SOURCE_FRAME_BYTES: usize = htmlcut_core::MAX_SOURCE_BYTES * 6 + 65_536;
const RESPONSE_BYTES: usize = htmlcut_core::MAX_DATA_BYTES + 1;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Request {
    pub plan: Option<ExtractionPlan>,
    pub inspect: Option<String>,
    pub samples: u32,
    pub raw: bool,
}
fn failure() -> ExtractionError {
    ExtractionError::new(
        htmlcut_core::ErrorCode::InternalInvariant,
        "worker",
        "Worker execution did not provide a verified complete response.",
    )
}
struct Reap(Child);
impl Drop for Reap {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
pub(crate) fn execute(
    request: Request,
    acquire: impl FnOnce() -> Result<SourceSnapshot, ExtractionError>,
) -> Result<Vec<u8>, ExtractionError> {
    execute_mode(request, acquire, "execute")
}
fn execute_mode(
    request: Request,
    acquire: impl FnOnce() -> Result<SourceSnapshot, ExtractionError>,
    mode: &str,
) -> Result<Vec<u8>, ExtractionError> {
    let bytes = crate::publication::json_payload(&request, PLAN_FRAME_BYTES)?;
    let executable = std::env::current_exe().map_err(|_| failure())?;
    #[cfg(test)]
    let executable = {
        let candidate = executable
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join(if cfg!(windows) {
                "htmlcut.exe"
            } else {
                "htmlcut"
            });
        let mut parts = std::path::PathBuf::new();
        for component in candidate.components() {
            if component.as_os_str() == "build" {
                parts.push("target");
            } else {
                parts.push(component.as_os_str());
            }
        }
        std::env::var_os("HTMLCUT_TEST_BINARY")
            .map(std::path::PathBuf::from)
            .unwrap_or(parts)
    };
    let child = Command::new(executable)
        .env(MODE, mode)
        .env_remove("DYLD_INSERT_LIBRARIES")
        .env_remove("LD_PRELOAD")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| failure())?;
    let mut child = Reap(child);
    #[cfg(windows)]
    let _policy = crate::native_resource::attach(&child.0).map_err(|_| failure())?;
    #[cfg(unix)]
    crate::native_resource::attach(&child.0).map_err(|_| failure())?;
    let mut input = child.0.stdin.take().ok_or_else(failure)?;
    input.write_all(&[8]).map_err(|_| failure())?;
    let mut output = child.0.stdout.take().ok_or_else(failure)?;
    let mut errors = child.0.stderr.take().ok_or_else(failure)?;
    let probe_started = AtomicBool::new(false);
    std::thread::scope(|scope| {
        let (events, receive) = mpsc::channel();
        let writer_events = events.clone();
        let (start, ready) = mpsc::channel();
        let (source_send, source_receive) = mpsc::channel::<Vec<u8>>();
        let (compiled_send, compiled_receive) = mpsc::channel();
        let writer = scope.spawn(move || {
            if ready.recv().is_err() {
                return Err(failure());
            }
            let result = (|| {
                input.write_all(&(bytes.len() as u32).to_le_bytes())?;
                input.write_all(&bytes)?;
                if let Ok(source) = source_receive.recv() {
                    input.write_all(&(source.len() as u32).to_le_bytes())?;
                    input.write_all(&source)?;
                }
                Ok::<_, std::io::Error>(())
            })();
            if result.is_err() {
                let _ = writer_events.send(Some(failure()));
            }
            result.map_err(|_| failure())
        });
        let output_events = events.clone();
        let probe_started_ref = &probe_started;
        let reader = scope.spawn(move || {
            let mut greeting = [0; 8];
            if output.read_exact(&mut greeting).is_err() || &greeting != b"HCREADY8" {
                let _ = output_events.send(Some(failure()));
                return Err(failure());
            }
            if start.send(()).is_err() {
                return Err(failure());
            }
            let mut phase = [0];
            if output.read_exact(&mut phase).is_err() {
                let _ = output_events.send(Some(failure()));
                return Err(failure());
            }
            let _ = compiled_send.send(phase[0] == 0);
            let result = if phase[0] == 0 {
                if mode != "execute" {
                    let mut marker = [0; 8];
                    if output.read_exact(&mut marker).is_err() || &marker != b"HCPROBE8" {
                        let _ = output_events.send(Some(failure()));
                        return Err(failure());
                    }
                    probe_started_ref.store(true, Ordering::SeqCst);
                    if mode == "timeout" {
                        // Private evaluation observes the existing workload-start ACK.
                        // Public extraction mode never emits this fixed diagnostic.
                        if std::io::stderr()
                            .write_all(b"HCPROBE8\n")
                            .and_then(|_| std::io::stderr().flush())
                            .is_err()
                        {
                            let _ = output_events.send(Some(failure()));
                            return Err(failure());
                        }
                    }
                }
                bounded(&mut output, RESPONSE_BYTES + 5, "worker_stdout_bytes")
            } else if phase[0] == 1 {
                bounded(&mut output, RESPONSE_BYTES + 4, "worker_stdout_bytes").map(|mut bytes| {
                    bytes.insert(0, 1);
                    bytes
                })
            } else {
                Err(failure())
            };
            let _ = output_events.send(result.as_ref().err().cloned());
            result
        });
        let error_reader = scope.spawn(move || {
            // A legitimate worker emits no stderr. Consume concurrently; cap panic/flood bytes.
            let result = bounded(&mut errors, 4096, "worker_stderr_bytes");
            let _ = events.send(match &result {
                Ok(bytes) if bytes.is_empty() => None,
                Ok(_) => Some(failure()),
                Err(error) => Some(error.clone()),
            });
            result
        });
        let mut deadline = Instant::now() + DEADLINE;
        let mut acquire = Some(acquire);
        let mut source_send = Some(source_send);
        let status = loop {
            if let Ok(ready) = compiled_receive.try_recv() {
                if ready {
                    let source = match acquire.take().unwrap()() {
                        Ok(source) => source,
                        Err(error) => {
                            let _ = child.0.kill();
                            break Err(error);
                        }
                    };
                    let source = match crate::publication::json_payload(
                        &Source {
                            html: source.html().to_owned(),
                            base: source.metadata().base_url.clone(),
                        },
                        SOURCE_FRAME_BYTES,
                    ) {
                        Ok(source) => source,
                        Err(error) => {
                            let _ = child.0.kill();
                            break Err(error);
                        }
                    };
                    deadline = Instant::now() + DEADLINE;
                    if source_send.take().unwrap().send(source).is_err() {
                        let _ = child.0.kill();
                        break Err(failure());
                    }
                } else {
                    source_send.take();
                }
            }
            if let Some(error) = receive.try_iter().flatten().next() {
                let _ = child.0.kill();
                break Err(error);
            }
            if Instant::now() >= deadline {
                let _ = child.0.kill();
                break Err(ExtractionError::resource(
                    "worker",
                    "worker_deadline_ms",
                    DEADLINE.as_millis() as u64,
                ));
            }
            match child.0.try_wait() {
                Ok(Some(status)) => {
                    break if status.success() {
                        Ok(())
                    } else {
                        Err(failure())
                    };
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(2)),
                Err(_) => {
                    let _ = child.0.kill();
                    break Err(failure());
                }
            }
        };
        let _ = child.0.wait();
        drop(source_send);
        if status.is_err() {
            let _ = writer.join();
            let _ = reader.join();
            let _ = error_reader.join();
            if mode != "execute" && !probe_started.load(Ordering::SeqCst) {
                return Err(ExtractionError::new(
                    htmlcut_core::ErrorCode::InternalInvariant,
                    "worker_probe",
                    "Probe workload did not start.",
                ));
            }
            return status.map(|_| Vec::new());
        }
        writer.join().map_err(|_| failure())??;
        let response = reader.join().map_err(|_| failure())??;
        if !error_reader.join().map_err(|_| failure())??.is_empty() {
            return Err(failure());
        }
        status?;
        decode(&response)
    })
}
fn bounded(
    reader: &mut impl Read,
    maximum: usize,
    counter: &'static str,
) -> Result<Vec<u8>, ExtractionError> {
    let mut bytes = Vec::new();
    let mut chunk = [0; 8192];
    loop {
        let count = reader.read(&mut chunk).map_err(|_| failure())?;
        if count == 0 {
            return Ok(bytes);
        }
        if count > maximum.saturating_sub(bytes.len()) {
            return Err(ExtractionError::resource("worker", counter, maximum as u64));
        }
        bytes.extend_from_slice(&chunk[..count]);
    }
}
fn decode(bytes: &[u8]) -> Result<Vec<u8>, ExtractionError> {
    if bytes.len() < 5 {
        return Err(failure());
    }
    let length = u32::from_le_bytes(bytes[1..5].try_into().unwrap()) as usize;
    if length != bytes.len() - 5 || length > RESPONSE_BYTES {
        return Err(failure());
    }
    match bytes[0] {
        0 => Ok(bytes[5..].to_vec()),
        1 if length <= 4096 => Err(serde_json::from_slice(&bytes[5..]).map_err(|_| failure())?),
        _ => Err(failure()),
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Source {
    html: String,
    base: Option<String>,
}
pub(crate) fn child_main() -> i32 {
    let mode = std::env::var(MODE).unwrap_or_default();
    if let Some(probe) = mode
        .strip_prefix("verify-")
        .or_else(|| (mode == "measure-pressure").then_some("pressure"))
    {
        let request = Request {
            plan: Some(ExtractionPlan::css("p").unwrap()),
            inspect: None,
            samples: 0,
            raw: false,
        };
        let result = execute_mode(
            request,
            || {
                SourceSnapshot::new(
                    if probe == "blocked-input" {
                        "x".repeat(1024 * 1024)
                    } else {
                        "<p>x</p>".to_owned()
                    },
                    SnapshotMetadata::default(),
                )
            },
            probe,
        );
        let pressure_proved = probe == "pressure"
            && result.as_ref().is_ok_and(|bytes| {
                let text = String::from_utf8_lossy(bytes);
                let mut parts = text.split(':');
                parts.next() == Some("memory-pressure")
                    && parts
                        .next()
                        .and_then(|n| n.parse::<u64>().ok())
                        .is_some_and(|n| (768 * 1024 * 1024..=MEMORY_BYTES).contains(&n))
                    && parts
                        .next()
                        .and_then(|n| n.parse::<u64>().ok())
                        .is_some_and(|n| n >= 512 * 1024 * 1024)
                    && parts.next().is_none()
            });
        if mode == "measure-pressure" {
            if !pressure_proved {
                return 4;
            }
            let bytes = result.unwrap();
            let text = String::from_utf8_lossy(&bytes);
            let numbers = text
                .split(':')
                .skip(1)
                .map(|n| n.parse::<u64>().unwrap())
                .collect::<Vec<_>>();
            let diagnostic = serde_json::json!({"allocated_bytes": numbers[0], "resident_bytes": numbers[1], "policy_bytes": MEMORY_BYTES});
            return match crate::publication::json(&diagnostic, 1024)
                .and_then(|bytes| std::io::stdout().write_all(&bytes).map_err(|_| failure()))
            {
                Ok(()) => 0,
                Err(_) => 5,
            };
        }
        return if pressure_proved
            || (probe == "memory"
                && result
                    .as_ref()
                    .is_ok_and(|bytes| bytes == b"memory-enforced"))
            || ([
                "timeout",
                "blocked-input",
                "stdout-flood",
                "stderr-flood",
                "abnormal",
                "malformed",
            ]
            .contains(&probe)
                && result.as_ref().is_err_and(|error| match probe {
                    "timeout" | "blocked-input" => {
                        error.resource_counter.as_deref() == Some("worker_deadline_ms")
                    }
                    "stdout-flood" => {
                        error.resource_counter.as_deref() == Some("worker_stdout_bytes")
                    }
                    "stderr-flood" => {
                        error.resource_counter.as_deref() == Some("worker_stderr_bytes")
                    }
                    "abnormal" | "malformed" => {
                        error.code == htmlcut_core::ErrorCode::InternalInvariant
                    }
                    _ => false,
                }))
        {
            0
        } else {
            4
        };
    }
    #[cfg(unix)]
    if crate::native_resource::install().is_err() {
        return 4;
    }
    let mut start = [0];
    if std::io::stdin().read_exact(&mut start).is_err() || start != [8] {
        return 4;
    }
    #[cfg(windows)]
    if crate::native_resource::install().is_err() {
        return 4;
    }
    if ![
        "execute",
        "memory",
        "pressure",
        "timeout",
        "blocked-input",
        "stdout-flood",
        "stderr-flood",
        "abnormal",
        "malformed",
    ]
    .contains(&mode.as_str())
    {
        return 4;
    }
    let mut stdout = std::io::stdout().lock();
    if stdout
        .write_all(b"HCREADY8")
        .and_then(|_| stdout.flush())
        .is_err()
    {
        return 4;
    }
    let result = child_request(&mut stdout, &mode);
    let (kind, payload) = match result {
        Ok(bytes) => (0, bytes),
        Err(error) => match crate::publication::json_payload(&error, 4096) {
            Ok(bytes) => (1, bytes),
            Err(_) => return 4,
        },
    };
    if stdout
        .write_all(&[kind])
        .and_then(|_| stdout.write_all(&(payload.len() as u32).to_le_bytes()))
        .and_then(|_| stdout.write_all(&payload))
        .and_then(|_| stdout.flush())
        .is_err()
    {
        return 4;
    }
    0
}
fn child_request(stdout: &mut impl Write, mode: &str) -> Result<Vec<u8>, ExtractionError> {
    let mut stdin = std::io::stdin().lock();
    let mut header = [0; 4];
    stdin.read_exact(&mut header).map_err(|_| failure())?;
    let length = u32::from_le_bytes(header) as usize;
    if length > PLAN_FRAME_BYTES {
        return Err(failure());
    }
    let mut bytes = vec![0; length];
    stdin.read_exact(&mut bytes).map_err(|_| failure())?;
    let request: Request = serde_json::from_slice(&bytes).map_err(|_| failure())?;
    if request.plan.is_some() == request.inspect.is_some() {
        return Err(failure());
    }
    let compiled = request
        .plan
        .as_ref()
        .map(CompiledPlan::compile)
        .transpose()?;
    if let Some(select) = &request.inspect {
        CompiledPlan::compile(&ExtractionPlan::css(select)?)?;
    }
    drop(bytes);
    stdout
        .write_all(&[0])
        .and_then(|_| stdout.flush())
        .map_err(|_| failure())?;
    if mode == "blocked-input" {
        stdout
            .write_all(b"HCPROBE8")
            .and_then(|_| stdout.flush())
            .map_err(|_| failure())?;
        loop {
            std::thread::sleep(Duration::from_secs(1));
        }
    }
    stdin.read_exact(&mut header).map_err(|_| failure())?;
    let length = u32::from_le_bytes(header) as usize;
    if length > SOURCE_FRAME_BYTES {
        return Err(failure());
    }
    let mut bytes = vec![0; length];
    stdin.read_exact(&mut bytes).map_err(|_| failure())?;
    let mut tail = [0];
    if stdin.read(&mut tail).map_err(|_| failure())? != 0 {
        return Err(failure());
    }
    let source: Source = serde_json::from_slice(&bytes).map_err(|_| failure())?;
    let snapshot = SourceSnapshot::new(
        &source.html,
        SnapshotMetadata {
            base_url: source.base.clone(),
        },
    )?;
    drop(bytes);
    drop(source);
    if mode != "execute" {
        stdout
            .write_all(b"HCPROBE8")
            .and_then(|_| stdout.flush())
            .map_err(|_| failure())?;
    }
    match mode {
        "memory" => {
            crate::native_resource::probe().map_err(|_| failure())?;
            return Ok(b"memory-enforced".to_vec());
        }
        "pressure" => {
            let (allocated, resident) =
                crate::native_resource::pressure().map_err(|_| failure())?;
            return Ok(format!("memory-pressure:{allocated}:{resident}").into_bytes());
        }
        "timeout" => loop {
            std::thread::sleep(Duration::from_secs(1));
        },
        "stdout-flood" => loop {
            stdout.write_all(&[0; 8192]).map_err(|_| failure())?;
        },
        "stderr-flood" => loop {
            std::io::stderr()
                .write_all(&[0; 8192])
                .map_err(|_| failure())?;
        },
        "abnormal" => std::process::exit(77),
        "malformed" => {
            stdout.write_all(b"BROKEN").map_err(|_| failure())?;
            std::process::exit(0);
        }
        _ => {}
    }
    let document = PreparedDocument::new(snapshot, PreparationLimits::default())?;
    if let Some(compiled) = compiled {
        let result = document.execute(&compiled)?;
        if request.raw {
            return match result.data() {
                htmlcut_core::ExtractionData::Values(values) if values.len() == 1 => {
                    Ok(values[0].as_bytes().to_vec())
                }
                _ => Err(crate::input::options(
                    "Raw output requires exactly one flat string.",
                )),
            };
        }
        let mut bytes =
            crate::publication::json_payload(result.data(), htmlcut_core::MAX_DATA_BYTES)?;
        bytes.push(b'\n');
        Ok(bytes)
    } else if let Some(select) = request.inspect {
        crate::publication::json(&document.inspect(&select, request.samples)?, 16_384)
    } else {
        Err(failure())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_incomplete_extra_and_unknown_frames() {
        for bytes in [
            vec![],
            vec![0, 2, 0, 0, 0, b'a'],
            vec![0, 0, 0, 0, 0, b'a'],
            vec![2, 0, 0, 0, 0],
        ] {
            assert!(decode(&bytes).is_err());
        }
        assert_eq!(decode(&[0, 1, 0, 0, 0, b'a']).unwrap(), b"a");
    }
}
