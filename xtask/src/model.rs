use std::collections::BTreeMap;
use std::error::Error;
use std::path::PathBuf;

use serde::Deserialize;

/// Convenience result type used across the repository-maintenance helpers.
pub type DynResult<T> = Result<T, Box<dyn Error>>;
pub(crate) type BranchSpan = (u64, u64, u64, u64);
pub(crate) type BranchCounts = (u64, u64);
pub(crate) type BranchCoverageByFile = BTreeMap<PathBuf, BTreeMap<BranchSpan, BranchCounts>>;

// Coverage is intentionally enforced as a 100% line-and-branch bar over the
// executable modules that define HTMLCut's maintained extraction, CLI adapter,
// and maintainer-gate logic. The tracked set is derived from the live source
// tree so future seam splits are scored automatically; only declarative-only
// modules with no maintained executable logic stay on the explicit exclusion
// list.
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
    "crates/htmlcut-core/src/cli_contract/help/mod.rs",
    "crates/htmlcut-core/src/document/mod.rs",
    "crates/htmlcut-core/src/extract/mod.rs",
    "crates/htmlcut-core/src/extract/slice/mod.rs",
    "crates/htmlcut-core/src/interop/mod.rs",
    "crates/htmlcut-core/src/lib.rs",
    "xtask/src/lib.rs",
];
// HTMLCut stays on stable for normal development. Coverage is the one place we
// intentionally hop to nightly because `cargo llvm-cov --branch` requires it.
pub(crate) const COVERAGE_TOOLCHAIN: &str = "+nightly";
pub(crate) const COVERAGE_TOOLCHAIN_NAME: &str = "nightly";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Missing prerequisite for the branch-coverage gate.
pub enum CoveragePreflightFailure {
    /// The nightly toolchain itself is not installed.
    MissingNightlyToolchain,
    /// Nightly exists, but it does not include `llvm-tools-preview`.
    MissingNightlyLlvmTools,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// One external command that `cargo xtask` should execute as part of a gate.
pub struct CommandSpec {
    /// The binary or script that should be launched.
    pub program: PathBuf,
    /// Command-line arguments passed to [`Self::program`].
    pub args: Vec<String>,
    /// Whether stdout should be suppressed unless the command fails.
    pub quiet_stdout: bool,
    /// Whether the command should force the documented clang `CC`/`CXX` toolchain environment.
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
