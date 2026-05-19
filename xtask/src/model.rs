use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::Deserialize;
use thiserror::Error;

/// Typed error surface used across the repository-maintenance helpers.
#[derive(Debug, Error)]
pub enum XtaskError {
    /// One human-authored maintenance failure message.
    #[error("{0}")]
    Message(String),
    /// Filesystem or process I/O failed.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// UTF-8 decoding of owned bytes failed.
    #[error(transparent)]
    FromUtf8(#[from] std::string::FromUtf8Error),
    /// UTF-8 decoding of borrowed bytes failed.
    #[error(transparent)]
    Utf8(#[from] std::str::Utf8Error),
    /// JSON parsing or serialization failed.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// One repository-owned TOML document failed to parse.
    #[error("invalid {document_name}: {source}")]
    TomlDocument {
        /// Repository-owned document name shown in diagnostics.
        document_name: &'static str,
        /// Underlying TOML parser error.
        #[source]
        source: toml::de::Error,
    },
    /// One repository-owned TOML document failed to serialize.
    #[error(transparent)]
    TomlSerialize(#[from] toml::ser::Error),
    /// One path-prefix operation failed.
    #[error(transparent)]
    StripPrefix(#[from] std::path::StripPrefixError),
    /// One integer parse failed.
    #[error(transparent)]
    ParseInt(#[from] std::num::ParseIntError),
    /// One regex parse failed.
    #[error(transparent)]
    Regex(#[from] regex::Error),
    /// One shell-words parse failed.
    #[error(transparent)]
    ShellWords(#[from] shell_words::ParseError),
    /// Clap rejected a maintainer CLI invocation.
    #[error(transparent)]
    Clap(#[from] clap::Error),
}

impl From<String> for XtaskError {
    fn from(value: String) -> Self {
        Self::Message(value)
    }
}

impl From<&str> for XtaskError {
    fn from(value: &str) -> Self {
        Self::Message(value.to_owned())
    }
}

impl XtaskError {
    /// Wraps one TOML parser failure in a repository-owned document label.
    pub fn invalid_toml(document_name: &'static str, source: toml::de::Error) -> Self {
        Self::TomlDocument {
            document_name,
            source,
        }
    }
}

/// Convenience result type used across the repository-maintenance helpers.
pub type DynResult<T> = Result<T, XtaskError>;
pub(crate) type BranchSpan = (u64, u64, u64, u64);
pub(crate) type BranchCounts = (u64, u64);
pub(crate) type BranchCoverageByFile = BTreeMap<PathBuf, BTreeMap<BranchSpan, BranchCounts>>;

// Coverage is intentionally enforced as a 100% line-and-branch bar over the
// tracked executable modules that define HTMLCut's maintained extraction, CLI
// adapter, and maintainer-gate logic. The tracked set is derived from the
// maintained worktree inventory so future seam splits are scored automatically
// without letting ignored scratch files pollute the gate. Declarative module
// surfaces and constant-only vocabulary files stay in the tracked inventory,
// but the scoring pass treats them as non-executable-by-design instead of
// pretending they should emit runnable line coverage.
pub(crate) const COVERAGE_SOURCE_ROOTS: &[&str] = &[
    "crates/htmlcut-core/src",
    "crates/htmlcut-cli/src",
    "xtask/src",
];

pub(crate) const COVERAGE_EXCLUDED_RELATIVE_PATHS: &[&str] = &[
    "crates/htmlcut-cli/src/args/discovery.rs",
    "crates/htmlcut-cli/src/args/extract.rs",
    "crates/htmlcut-cli/src/args/inspect.rs",
    "crates/htmlcut-cli/src/args/shared.rs",
    "crates/htmlcut-cli/src/model/catalog.rs",
    "crates/htmlcut-cli/src/model/mod.rs",
    "crates/htmlcut-cli/src/model/reports.rs",
    "crates/htmlcut-cli/src/model/schema.rs",
    "crates/htmlcut-cli/src/prepare/build/mod.rs",
    "crates/htmlcut-cli/src/prepare/definition/mod.rs",
    "crates/htmlcut-cli/src/render/inspection/mod.rs",
    "crates/htmlcut-cli/src/prepare/reports/mod.rs",
    "crates/htmlcut-core/src/contracts/request/mod.rs",
    "crates/htmlcut-cli/src/contract/help/mod.rs",
    "crates/htmlcut-core/src/document/mod.rs",
    "crates/htmlcut-core/src/extract/mod.rs",
    "crates/htmlcut-core/src/extract/slice/mod.rs",
    "crates/htmlcut-core/src/interop/mod.rs",
    "crates/htmlcut-core/src/source/http.rs",
    "crates/htmlcut-core/src/lib.rs",
    "crates/htmlcut-core/src/wire/mod.rs",
    "xtask/src/lib.rs",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Coverage scoring expectation for one tracked Rust source file.
pub enum CoverageSourceKind {
    /// The file contains executable Rust semantics and must satisfy the 100% coverage bar.
    Executable,
    /// The file is declarative-only and may legitimately emit no executable lines.
    DeclarativeOnly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// One tracked Rust source file together with its repo-relative display path and scoring policy.
pub struct TrackedCoverageFile {
    /// Repo-relative path used in diagnostics and reports.
    pub display_path: String,
    /// Whether the source should contribute executable lines and branches to the coverage gate.
    pub kind: CoverageSourceKind,
}

impl TrackedCoverageFile {
    /// Builds one tracked executable source file record.
    pub fn executable(display_path: impl Into<String>) -> Self {
        Self {
            display_path: display_path.into(),
            kind: CoverageSourceKind::Executable,
        }
    }

    /// Builds one tracked declarative-only source file record.
    pub fn declarative_only(display_path: impl Into<String>) -> Self {
        Self {
            display_path: display_path.into(),
            kind: CoverageSourceKind::DeclarativeOnly,
        }
    }
}
// HTMLCut stays on stable for normal development. The maintained safety and
// coverage proofs intentionally hop to nightly because Miri, cargo-fuzz, and
// `cargo llvm-cov --branch` still require it.
pub(crate) const MAINTAINED_NIGHTLY_TOOLCHAIN: &str = "+nightly";
pub(crate) const MAINTAINED_NIGHTLY_TOOLCHAIN_NAME: &str = "nightly";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Missing prerequisite for the branch-coverage gate.
pub enum CoveragePreflightFailure {
    /// The nightly toolchain itself is not installed.
    MissingNightlyToolchain,
    /// Nightly exists, but it does not include `llvm-tools-preview`.
    MissingNightlyLlvmTools,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Missing prerequisite for the maintained strict-provenance selector-and-slice Miri proof.
pub enum MiriPreflightFailure {
    /// The nightly toolchain itself is not installed.
    MissingNightlyToolchain,
    /// Nightly exists, but it does not include the `miri` component.
    MissingNightlyMiri,
    /// Nightly exists, but it does not include the `rust-src` component.
    MissingNightlyRustSrc,
    /// `cargo +nightly miri --version` still does not run after rustup reports the components.
    BrokenNightlyMiriBinary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Stdout handling for one external maintainer command.
pub enum CommandStdout {
    /// Stream stdout directly to the terminal.
    Inherit,
    /// Suppress stdout unless the command fails.
    Quiet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Stderr handling for one external maintainer command.
pub enum CommandStderr {
    /// Stream stderr directly to the terminal.
    Inherit,
    /// Suppress stderr unless the command fails.
    Quiet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Environment policy for one external maintainer command.
pub enum CommandToolchainEnv {
    /// Run with the ambient process environment.
    Inherit,
    /// Force the documented clang `CC`/`CXX` toolchain environment.
    ForceClang,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Cargo artifact-root policy for one external maintainer command.
pub enum CommandArtifactLayout {
    /// Use the ambient Cargo artifact layout.
    Inherit,
    /// Route Cargo output into the maintained workspace artifact roots.
    ManagedWorkspace,
    /// Route Cargo output into the isolated maintained coverage artifact roots.
    ManagedCoverage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// One external command that `cargo xtask` should execute as part of a gate.
pub struct CommandSpec {
    /// The binary or script that should be launched.
    pub program: PathBuf,
    /// Command-line arguments passed to [`Self::program`].
    pub args: Vec<String>,
    /// Stdout handling for the command.
    pub stdout: CommandStdout,
    /// Stderr handling for the command.
    pub stderr: CommandStderr,
    /// Environment policy for the command.
    pub toolchain_env: CommandToolchainEnv,
    /// Artifact-root policy for the command.
    pub artifact_layout: CommandArtifactLayout,
    /// Explicit environment overrides for the command.
    pub env: BTreeMap<String, String>,
}

impl CommandSpec {
    /// Builds one executable command description.
    pub fn new<I, S>(
        program: impl Into<PathBuf>,
        args: I,
        stdout: CommandStdout,
        toolchain_env: CommandToolchainEnv,
    ) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            program: program.into(),
            args: args.into_iter().map(Into::into).collect(),
            stdout,
            stderr: CommandStderr::Inherit,
            toolchain_env,
            artifact_layout: CommandArtifactLayout::Inherit,
            env: BTreeMap::new(),
        }
    }

    /// Overrides the default stderr handling for the command.
    pub fn with_stderr(mut self, stderr: CommandStderr) -> Self {
        self.stderr = stderr;
        self
    }

    /// Overrides the default artifact-root policy for the command.
    pub fn with_artifact_layout(mut self, artifact_layout: CommandArtifactLayout) -> Self {
        self.artifact_layout = artifact_layout;
        self
    }

    /// Adds one explicit environment override to the command.
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }
}

#[derive(Debug, Deserialize)]
/// Top-level `cargo llvm-cov --json` payload consumed by the coverage gate.
pub struct CoverageReport {
    #[serde(default)]
    /// Coverage datasets emitted by `llvm-cov`.
    pub data: Vec<CoverageDataSet>,
}

#[derive(Debug, Deserialize)]
/// One dataset entry inside the `llvm-cov` JSON payload.
pub struct CoverageDataSet {
    #[serde(default)]
    /// File-level coverage records inside the dataset.
    pub files: Vec<CoverageFile>,
}

#[derive(Debug, Deserialize)]
/// Raw coverage details for one tracked source file.
pub struct CoverageFile {
    /// Absolute file path as emitted by `llvm-cov`.
    pub filename: PathBuf,
    #[serde(default)]
    /// Segment records `(line, column, count, has_count, is_region_entry, is_gap)`.
    pub segments: Vec<(u64, u64, u64, bool, bool, bool)>,
    #[serde(default)]
    /// Branch coverage records emitted by `llvm-cov`.
    pub branches: Vec<CoverageBranchRecord>,
    #[serde(default)]
    /// Aggregate branch counters for the file.
    pub summary: CoverageFileSummary,
}

/// Raw branch tuple emitted by `llvm-cov`.
pub type CoverageBranchRecord = (u64, u64, u64, u64, u64, u64, u64, u64, u64);

#[derive(Debug, Default, Deserialize, Clone, Copy, PartialEq, Eq)]
/// Aggregate coverage summary for one source file.
pub struct CoverageFileSummary {
    #[serde(default)]
    /// Branch counters for the file.
    pub branches: CoverageCounter,
}

#[derive(Debug, Default, Deserialize, Clone, Copy, PartialEq, Eq)]
/// Generic `count / covered / not covered` counter tuple from `llvm-cov`.
pub struct CoverageCounter {
    #[serde(default)]
    /// Total entities tracked for this counter.
    pub count: u64,
    #[serde(default)]
    /// Covered entities tracked for this counter.
    pub covered: u64,
    #[serde(default, rename = "notcovered")]
    /// Uncovered entities tracked for this counter.
    pub not_covered: u64,
}

#[derive(Debug, PartialEq, Eq)]
/// One tracked file that failed the line or branch coverage bar.
pub struct CoverageFailure {
    /// Repo-relative path to the failing file.
    pub file: String,
    /// Executable lines that were not covered.
    pub uncovered_lines: Vec<String>,
    /// Number of uncovered logical branches after deduplication.
    pub uncovered_branch_count: usize,
}

#[derive(Debug, PartialEq, Eq)]
/// Coverage status across every tracked production module.
pub struct CoverageSummary {
    /// Number of executable lines scored by the gate.
    pub tracked_line_count: usize,
    /// Number of logical branches scored by the gate.
    pub tracked_branch_count: usize,
    /// Files that missed the enforced 100% line or branch bar.
    pub failures: Vec<CoverageFailure>,
}
