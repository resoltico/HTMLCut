use std::collections::BTreeMap;
use std::error::Error;
use std::path::PathBuf;

use serde::Deserialize;

/// Convenience result type used across the repository-maintenance helpers.
pub type DynResult<T> = Result<T, Box<dyn Error>>;
pub(crate) type BranchSpan = (u64, u64, u64, u64);
pub(crate) type BranchCounts = (u64, u64);
pub(crate) type BranchCoverageByFile = BTreeMap<PathBuf, BTreeMap<BranchSpan, BranchCounts>>;

// Coverage is enforced against production modules that carry executable logic.
// Thin crate roots, entrypoints, and declarative-only modules are intentionally
// excluded because llvm-cov reports no executable lines for them.
pub(crate) const TRACKED_RELATIVE_PATHS: &[&str] = &[
    "crates/htmlcut-core/src/catalog.rs",
    "crates/htmlcut-core/src/contracts/mod.rs",
    "crates/htmlcut-core/src/diagnostics.rs",
    "crates/htmlcut-core/src/document.rs",
    "crates/htmlcut-core/src/extract.rs",
    "crates/htmlcut-core/src/inspect.rs",
    "crates/htmlcut-core/src/source.rs",
    "crates/htmlcut-cli/src/error.rs",
    "crates/htmlcut-cli/src/execute.rs",
    "crates/htmlcut-cli/src/prepare.rs",
    "crates/htmlcut-cli/src/render.rs",
    "xtask/src/model.rs",
    "xtask/src/plan.rs",
    "xtask/src/coverage.rs",
];
// HTMLCut stays on stable for normal development. Coverage is the one place we
// intentionally hop to nightly because `cargo llvm-cov --branch` requires it.
pub(crate) const COVERAGE_TOOLCHAIN: &str = "+nightly";

#[derive(Debug, Clone, PartialEq, Eq)]
/// One external command that `cargo xtask` should execute as part of a gate.
pub struct CommandSpec {
    /// The binary or script that should be launched.
    pub program: PathBuf,
    /// Command-line arguments passed to [`Self::program`].
    pub args: Vec<String>,
    /// Whether stdout should be suppressed unless the command fails.
    pub quiet_stdout: bool,
    /// Whether the command should force the clang toolchain for coverage helpers.
    pub force_clang: bool,
}

impl CommandSpec {
    /// Builds one executable command description.
    pub fn new<I, S>(
        program: impl Into<PathBuf>,
        args: I,
        quiet_stdout: bool,
        force_clang: bool,
    ) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            program: program.into(),
            args: args.into_iter().map(Into::into).collect(),
            quiet_stdout,
            force_clang,
        }
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
