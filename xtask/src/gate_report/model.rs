//! Typed public report documents for one HTMLCut maintainer-gate run.

use std::path::Path;

use clap::{Args, ValueEnum};
use serde::Serialize;

/// Stable schema-family name for one maintainer-gate run report.
pub const GATE_RUN_REPORT_SCHEMA_NAME: &str = "htmlcut.gate_run";
/// The stable schema identity for one maintainer-gate run report.
pub const GATE_RUN_REPORT_SCHEMA: &str = "htmlcut.gate_run@1";

/// Selects the public rendering of one maintainer-gate run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum GateOutputFormat {
    /// Concise, human-oriented progress and final summary.
    Human,
    /// One machine-readable gate-run report on standard output.
    Json,
}

/// Shared rendering controls for maintainer commands that execute a quality gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Args)]
pub struct GateOutputOptions {
    /// Render a concise human summary or one structured JSON report.
    #[arg(long, value_enum, default_value_t = GateOutputFormat::Human)]
    pub format: GateOutputFormat,
    /// Also replay each retained command stream after that command finishes.
    #[arg(long)]
    pub verbose: bool,
}

/// Machine-readable evidence produced by one complete maintainer-gate run.
#[derive(Debug, Serialize)]
pub(super) struct GateRunReport {
    /// Schema identity for the report document.
    pub schema: &'static str,
    /// Stable name of the invoked quality gate.
    pub gate: String,
    /// Repository root that owns this execution.
    pub repo_root: String,
    /// Unique execution identity derived from timestamp, process, and sequence.
    pub run_id: String,
    /// Absolute path of this report document.
    pub report_path: String,
    /// Unix timestamp in milliseconds when execution began.
    pub started_at_unix_ms: u128,
    /// Unix timestamp in milliseconds when execution ended.
    pub finished_at_unix_ms: u128,
    /// End-to-end wall-clock duration.
    pub duration_ms: u128,
    /// Final gate outcome.
    pub outcome: GateOutcome,
    /// Failure that stopped the run, when any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure: Option<GateFailure>,
    /// Ordered command and internal-check records.
    pub steps: Vec<GateStep>,
    /// Deduplicated warning text observed across the retained command streams.
    pub warnings: Vec<GateWarning>,
}

impl GateRunReport {
    /// Starts an incomplete gate-run report with its immutable identity fields.
    pub(super) fn started(
        gate: &str,
        repo_root: &Path,
        run_id: String,
        report_path: &Path,
        started_at_unix_ms: u128,
    ) -> Self {
        Self {
            schema: GATE_RUN_REPORT_SCHEMA,
            gate: gate.to_owned(),
            repo_root: repo_root.display().to_string(),
            run_id,
            report_path: report_path.display().to_string(),
            started_at_unix_ms,
            finished_at_unix_ms: started_at_unix_ms,
            duration_ms: 0,
            outcome: GateOutcome::Running,
            failure: None,
            steps: Vec::new(),
            warnings: Vec::new(),
        }
    }
}

/// The lifecycle outcome of one gate run or individual recorded step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum GateOutcome {
    /// The gate has started but has not yet reached its final state.
    Running,
    /// The gate or step completed successfully.
    Passed,
    /// The gate or step failed.
    Failed,
}

/// One ordered command or internal verification result inside a gate run.
#[derive(Debug, Serialize)]
pub(super) struct GateStep {
    /// One-based execution order within this run.
    pub index: usize,
    /// Stable run-local identifier derived from the gate name and order.
    pub id: String,
    /// Whether this record is a spawned command or an internal verification.
    pub kind: GateStepKind,
    /// Human-readable command or verification label.
    pub label: String,
    /// Process invocation when this is a spawned command.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<GateCommand>,
    /// Result of the command or verification.
    pub outcome: GateOutcome,
    /// Process exit code when the command exited normally.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    /// Wall-clock duration of the command or verification.
    pub duration_ms: u128,
    /// Retained standard-output log path, relative to the run directory.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stdout_log: Option<String>,
    /// Retained standard-error log path, relative to the run directory.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stderr_log: Option<String>,
    /// Raw standard-output byte count.
    pub stdout_bytes: usize,
    /// Raw standard-error byte count.
    pub stderr_bytes: usize,
    /// Warning diagnostics extracted from this step's raw streams.
    pub warnings: Vec<GateWarning>,
    /// Bounded diagnostic tail for a failed command or verification.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_tail: Option<String>,
}

/// The implementation category of one recorded step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum GateStepKind {
    /// A command spawned by the maintainer workflow.
    Command,
    /// An in-process verification such as source-shape or coverage scoring.
    InternalCheck,
}

/// One command invocation, without environment values that could contain secrets.
#[derive(Debug, Serialize)]
pub(super) struct GateCommand {
    /// Executable path or program name.
    pub program: String,
    /// Exact command-line arguments.
    pub args: Vec<String>,
    /// Names of explicitly overridden environment variables.
    pub environment_keys: Vec<String>,
}

/// One retained warning line and the stream that emitted it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(super) struct GateWarning {
    /// Whether this warning originated on standard output or standard error.
    pub stream: GateStream,
    /// The normalized warning line.
    pub message: String,
}

/// One command stream that can contribute diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum GateStream {
    /// Standard output.
    Stdout,
    /// Standard error.
    Stderr,
}

/// Final failure context retained by the report even when no command was spawned.
#[derive(Debug, Serialize)]
pub(super) struct GateFailure {
    /// Human-readable root cause returned by the gate.
    pub message: String,
}
