//! Retained evidence and concise rendering for HTMLCut maintainer-gate runs.

mod model;

use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::hygiene::prepare_gate_report_root;
use crate::model::{CommandSpec, DynResult};

pub(crate) use model::GATE_RUN_REPORT_SCHEMA_NAME;
use model::{
    GateCommand, GateFailure, GateOutcome, GateRunReport, GateStep, GateStepKind, GateStream,
    GateWarning,
};
pub use model::{GateOutputFormat, GateOutputOptions};

const MAX_RETAINED_RUNS: usize = 20;
const FAILURE_TAIL_BYTES: usize = 8 * 1024;

thread_local! {
    static ACTIVE_GATE_RUN: RefCell<Option<Rc<RefCell<GateRun>>>> = const { RefCell::new(None) };
}

static RUN_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Executes one gate while retaining every spawned-command stream and rendering one final report.
pub fn with_gate_report<T>(
    repo_root: &Path,
    gate: &str,
    options: GateOutputOptions,
    operation: impl FnOnce() -> DynResult<T>,
) -> DynResult<T> {
    if is_active() {
        return Err("nested maintainer gate reporting is not supported".into());
    }

    let active = Rc::new(RefCell::new(GateRun::start(repo_root, gate, options)?));
    install_active(Rc::clone(&active));
    let result = operation();
    clear_active();

    let failure = result.as_ref().err().map(ToString::to_string);
    let finish = active.borrow_mut().finish(failure.as_deref());
    match (result, finish) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(error)) => Err(error),
        (Err(operation_error), Err(report_error)) => Err(format!(
            "{operation_error}\n\nAdditionally, HTMLCut could not finalize the gate report: {report_error}"
        )
        .into()),
    }
}

/// Returns whether the current thread is executing an instrumented maintainer gate.
pub fn is_active() -> bool {
    ACTIVE_GATE_RUN.with(|slot| slot.borrow().is_some())
}

/// Announces a command before it starts and returns its run-local step index.
pub(crate) fn begin_command(spec: &CommandSpec) -> Option<usize> {
    with_active(|run| run.begin_command(spec))
}

/// Retains one completed command and returns the run-local failure context when it failed.
pub(crate) fn finish_command(
    index: usize,
    spec: &CommandSpec,
    output: &Output,
    duration: Duration,
) -> Option<String> {
    with_active(|run| run.finish_command(index, spec, output, duration))
}

/// Retains a command-spawn failure that produced no process output.
pub(crate) fn finish_command_spawn_failure(
    index: usize,
    spec: &CommandSpec,
    error: &std::io::Error,
    duration: Duration,
) -> Option<String> {
    with_active(|run| run.finish_command_spawn_failure(index, spec, error, duration))
}

/// Records one in-process verification result without corrupting JSON-mode output.
pub(crate) fn record_internal_check(label: &str, result: Result<(), String>, duration: Duration) {
    if is_active() {
        let _recorded = with_active(|run| run.record_internal_check(label, result, duration));
        return;
    }

    match result {
        Ok(()) => println!("{label} passed."),
        Err(message) => eprintln!("{label} failed: {message}"),
    }
}

fn install_active(active: Rc<RefCell<GateRun>>) {
    ACTIVE_GATE_RUN.with(|slot| {
        let mut slot = slot.borrow_mut();
        debug_assert!(
            slot.is_none(),
            "maintainer gate reporter must be installed once"
        );
        *slot = Some(active);
    });
}

fn clear_active() {
    ACTIVE_GATE_RUN.with(|slot| {
        *slot.borrow_mut() = None;
    });
}

fn with_active<T>(operation: impl FnOnce(&mut GateRun) -> T) -> Option<T> {
    ACTIVE_GATE_RUN.with(|slot| {
        let active = slot.borrow().as_ref().cloned()?;
        Some(operation(&mut active.borrow_mut()))
    })
}

struct GateRun {
    options: GateOutputOptions,
    run_dir: PathBuf,
    started: Instant,
    report: GateRunReport,
}

impl GateRun {
    fn start(repo_root: &Path, gate: &str, options: GateOutputOptions) -> DynResult<Self> {
        let root = prepare_gate_report_root(repo_root)?;
        prune_completed_runs(&root)?;
        let started_at = SystemTime::now();
        let started_at_unix_ms = unix_millis(started_at)?;
        let run_id = format!(
            "run-{started_at_unix_ms}-{}-{}",
            std::process::id(),
            RUN_SEQUENCE.fetch_add(1, Ordering::Relaxed),
        );
        let run_dir = root.join(&run_id);
        fs::create_dir_all(run_dir.join("steps"))?;
        let report_path = run_dir.join("report.json");
        let report =
            GateRunReport::started(gate, repo_root, run_id, &report_path, started_at_unix_ms);

        if options.format == GateOutputFormat::Human {
            println!("==> HTMLCut gate: {gate}");
        }

        Ok(Self {
            options,
            run_dir,
            started: Instant::now(),
            report,
        })
    }

    fn begin_command(&mut self, spec: &CommandSpec) -> usize {
        let index = self.report.steps.len() + 1;
        if self.options.format == GateOutputFormat::Human {
            println!("==> [{index:02}] {}", render_command(spec));
        }
        index
    }

    fn finish_command(
        &mut self,
        index: usize,
        spec: &CommandSpec,
        output: &Output,
        duration: Duration,
    ) -> String {
        let stdout_log = format!("steps/{index:03}.stdout.log");
        let stderr_log = format!("steps/{index:03}.stderr.log");
        let stdout_path = self.run_dir.join(&stdout_log);
        let stderr_path = self.run_dir.join(&stderr_log);
        let write_result = fs::write(&stdout_path, &output.stdout)
            .and_then(|()| fs::write(&stderr_path, &output.stderr));

        let status = output.status;
        let write_error = write_result.as_ref().err().map(ToString::to_string);
        let success = status.success() && write_error.is_none();
        let warning_list = warnings_from_output(&output.stdout, &output.stderr);
        let failure_tail = if success {
            None
        } else {
            let mut tail = String::new();
            if let Some(error) = &write_error {
                tail.push_str(&format!("failed to retain command logs: {error}\n"));
            }
            let diagnostic_tail = combined_failure_tail(&output.stdout, &output.stderr);
            tail.push_str(&diagnostic_tail);
            (!tail.is_empty()).then_some(tail)
        };
        let outcome = if success {
            GateOutcome::Passed
        } else {
            GateOutcome::Failed
        };
        let failure_context = if success {
            None
        } else if let Some(error) = write_error {
            Some(format!(
                "could not retain evidence for `{}`: {error}",
                render_command(spec)
            ))
        } else {
            Some(format!(
                "command `{}` failed with status {status}; retained logs: {} and {}",
                render_command(spec),
                stdout_path.display(),
                stderr_path.display(),
            ))
        };
        self.record_warnings(&warning_list);
        self.report.steps.push(GateStep {
            index,
            id: format!("{}/{index:03}", self.report.gate),
            kind: GateStepKind::Command,
            label: render_command(spec),
            command: Some(command_document(spec)),
            outcome,
            exit_code: status.code(),
            duration_ms: duration.as_millis(),
            stdout_log: Some(stdout_log),
            stderr_log: Some(stderr_log),
            stdout_bytes: output.stdout.len(),
            stderr_bytes: output.stderr.len(),
            warnings: warning_list,
            failure_tail,
        });

        if self.options.format == GateOutputFormat::Human {
            if success {
                println!("    passed in {} ms", duration.as_millis());
            } else {
                eprintln!("    failed in {} ms", duration.as_millis());
            }
            if self.options.verbose {
                replay_stream("stdout", &output.stdout, false);
                replay_stream("stderr", &output.stderr, true);
            }
        }

        failure_context.unwrap_or_default()
    }

    fn finish_command_spawn_failure(
        &mut self,
        index: usize,
        spec: &CommandSpec,
        error: &std::io::Error,
        duration: Duration,
    ) -> String {
        let message = format!("could not start `{}`: {error}", render_command(spec));
        self.report.steps.push(GateStep {
            index,
            id: format!("{}/{index:03}", self.report.gate),
            kind: GateStepKind::Command,
            label: render_command(spec),
            command: Some(command_document(spec)),
            outcome: GateOutcome::Failed,
            exit_code: None,
            duration_ms: duration.as_millis(),
            stdout_log: None,
            stderr_log: None,
            stdout_bytes: 0,
            stderr_bytes: 0,
            warnings: Vec::new(),
            failure_tail: Some(message.clone()),
        });
        if self.options.format == GateOutputFormat::Human {
            eprintln!("    failed in {} ms", duration.as_millis());
        }
        message
    }

    fn record_internal_check(
        &mut self,
        label: &str,
        result: Result<(), String>,
        duration: Duration,
    ) {
        let index = self.report.steps.len() + 1;
        let (outcome, failure_tail) = match result {
            Ok(()) => (GateOutcome::Passed, None),
            Err(message) => (GateOutcome::Failed, Some(message)),
        };
        if self.options.format == GateOutputFormat::Human {
            println!("==> [{index:02}] {label}");
        }
        self.report.steps.push(GateStep {
            index,
            id: format!("{}/{index:03}", self.report.gate),
            kind: GateStepKind::InternalCheck,
            label: label.to_owned(),
            command: None,
            outcome,
            exit_code: None,
            duration_ms: duration.as_millis(),
            stdout_log: None,
            stderr_log: None,
            stdout_bytes: 0,
            stderr_bytes: 0,
            warnings: Vec::new(),
            failure_tail,
        });
        if self.options.format == GateOutputFormat::Human {
            if outcome == GateOutcome::Passed {
                println!("    passed in {} ms", duration.as_millis());
            } else {
                eprintln!("    failed in {} ms", duration.as_millis());
            }
        }
    }

    fn record_warnings(&mut self, warnings: &[GateWarning]) {
        for warning in warnings {
            if !self.report.warnings.contains(warning) {
                self.report.warnings.push(warning.clone());
            }
        }
    }

    fn finish(&mut self, failure: Option<&str>) -> DynResult<()> {
        self.report.finished_at_unix_ms = unix_millis(SystemTime::now())?;
        self.report.duration_ms = self.started.elapsed().as_millis();
        self.report.outcome = if failure.is_some() {
            GateOutcome::Failed
        } else {
            GateOutcome::Passed
        };
        self.report.failure = failure.map(|message| GateFailure {
            message: message.to_owned(),
        });
        let serialized = serde_json::to_vec_pretty(&self.report)?;
        fs::write(&self.report.report_path, &serialized)?;

        match self.options.format {
            GateOutputFormat::Human => self.render_human_summary(),
            GateOutputFormat::Json => println!("{}", String::from_utf8(serialized)?),
        }
        Ok(())
    }

    fn render_human_summary(&self) {
        let passed = self
            .report
            .steps
            .iter()
            .filter(|step| step.outcome == GateOutcome::Passed)
            .count();
        let failed = self.report.steps.len() - passed;
        let summary = format!(
            "HTMLCut gate `{}` {}: {} passed, {} failed, {} distinct warnings, {} ms.",
            self.report.gate,
            if self.report.outcome == GateOutcome::Passed {
                "passed"
            } else {
                "failed"
            },
            passed,
            failed,
            self.report.warnings.len(),
            self.report.duration_ms,
        );
        if self.report.outcome == GateOutcome::Passed {
            println!("{summary}");
        } else {
            eprintln!("{summary}");
        }
        for warning in &self.report.warnings {
            eprintln!(
                "warning [{}]: {}",
                render_stream(warning.stream),
                warning.message
            );
        }
        if let Some(failure) = &self.report.failure {
            eprintln!("failure: {}", failure.message);
        }
        for step in self
            .report
            .steps
            .iter()
            .filter(|step| step.outcome == GateOutcome::Failed)
        {
            if let Some(tail) = &step.failure_tail {
                eprintln!("--- failed step {} ---\n{tail}", step.label);
            }
        }
        println!("Gate report: {}", self.report.report_path);
    }
}

fn prune_completed_runs(root: &Path) -> DynResult<()> {
    let mut runs = fs::read_dir(root)?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let file_type = entry.file_type().ok()?;
            (file_type.is_dir() && !file_type.is_symlink()).then_some(entry)
        })
        .filter(|entry| entry.path().join("report.json").is_file())
        .collect::<Vec<_>>();
    runs.sort_by_key(|entry| entry.file_name());
    let count_to_remove = runs.len().saturating_sub(MAX_RETAINED_RUNS - 1);
    for entry in runs.into_iter().take(count_to_remove) {
        fs::remove_dir_all(entry.path())?;
    }
    Ok(())
}

fn unix_millis(timestamp: SystemTime) -> DynResult<u128> {
    timestamp
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .map_err(|error| format!("system clock is before the Unix epoch: {error}").into())
}

fn command_document(spec: &CommandSpec) -> GateCommand {
    GateCommand {
        program: spec.program.display().to_string(),
        args: spec.args.clone(),
        environment_keys: spec.env.keys().cloned().collect(),
    }
}

fn render_command(spec: &CommandSpec) -> String {
    std::iter::once(spec.program.display().to_string())
        .chain(spec.args.iter().cloned())
        .collect::<Vec<_>>()
        .join(" ")
}

fn warnings_from_output(stdout: &[u8], stderr: &[u8]) -> Vec<GateWarning> {
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

fn bounded_tail(bytes: &[u8]) -> String {
    let start = bytes.len().saturating_sub(FAILURE_TAIL_BYTES);
    String::from_utf8_lossy(&bytes[start..]).into_owned()
}

fn combined_failure_tail(stdout: &[u8], stderr: &[u8]) -> String {
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

fn replay_stream(name: &str, bytes: &[u8], stderr: bool) {
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

fn render_stream(stream: GateStream) -> &'static str {
    match stream {
        GateStream::Stdout => "stdout",
        GateStream::Stderr => "stderr",
    }
}

#[cfg(test)]
mod tests;
