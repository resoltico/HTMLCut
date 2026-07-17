use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use clap::ValueEnum;
use serde::Serialize;

use crate::model::{CommandArtifactLayout, DynResult};
use crate::plan::{
    cargo_build_dir, cargo_target_dir, coverage_build_dir, coverage_cargo_build_dir,
    coverage_cargo_target_dir, coverage_target_dir, gate_report_dir, semver_scratch_dir,
};
use crate::remove_dir_if_exists;

const GIB: u64 = 1024 * 1024 * 1024;
const MIB: u64 = 1024 * 1024;
const ARTIFACT_MANIFEST_NAME: &str = ".htmlcut-artifact.toml";
const CACHEDIR_TAG_NAME: &str = "CACHEDIR.TAG";
const CACHEDIR_TAG_CONTENTS: &str = "Signature: 8a477f597d28d172789f06886806bc55\n# This directory stores disposable HTMLCut build cache data.\n";
const SCHEMA_NAME: &str = "htmlcut.hygiene_report@1";
const ARTIFACT_SCHEMA_NAME: &str = "htmlcut.artifact_root@1";
const MANAGED_TARGET_BUDGET_BYTES: u64 = 4 * GIB;
const MANAGED_BUILD_BUDGET_BYTES: u64 = 24 * GIB;
const MANAGED_COVERAGE_TARGET_BUDGET_BYTES: u64 = 2 * GIB;
const MANAGED_COVERAGE_BUILD_BUDGET_BYTES: u64 = 8 * GIB;
const MANAGED_GATE_REPORT_BUDGET_BYTES: u64 = GIB;
const LEGACY_REPO_TARGET_BUDGET_BYTES: u64 = 512 * MIB;
const REPO_TMP_BUDGET_BYTES: u64 = 256 * MIB;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum ManagedArtifactKind {
    WorkspaceTarget,
    WorkspaceBuild,
    CoverageTarget,
    CoverageBuild,
    GateReports,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
/// Output format for `cargo xtask hygiene report`.
pub enum HygieneReportFormat {
    /// Human-readable text.
    Text,
    /// Structured JSON.
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
/// Cleanup profile for `cargo xtask hygiene clean`.
pub enum HygieneCleanMode {
    /// Remove disposable scratch, the repo-local temporary workspace, and accidental legacy target trees.
    Safe,
    /// Remove every rebuildable artifact root, including managed caches.
    Rebuildable,
}

#[derive(Debug, Serialize)]
/// Machine-readable repository artifact report.
pub struct HygieneReport {
    /// Schema identity for the report document.
    pub schema: &'static str,
    /// Repository root that owns this report.
    pub repo_root: String,
    /// Total bytes across the reported entries.
    pub total_bytes: u64,
    /// Reported artifact entries.
    pub entries: Vec<HygieneEntry>,
    /// Policy violations detected for the current repository.
    pub violations: Vec<HygieneViolation>,
}

#[derive(Debug, Serialize)]
/// One classified artifact root or aggregate inside the hygiene report.
pub struct HygieneEntry {
    /// Stable report identifier.
    pub id: String,
    /// Artifact class name.
    pub kind: String,
    /// Path represented by the entry.
    pub path: String,
    /// Whether the path currently exists.
    pub present: bool,
    /// Total bytes under the path or aggregate.
    pub bytes: u64,
    /// Optional budget enforced for the entry.
    pub budget_bytes: Option<u64>,
    /// Whether the entry is owned by the managed hygiene system.
    pub managed: bool,
    /// Whether the entry is safe to delete and rebuild.
    pub safe_to_delete: bool,
    /// Human-readable details for aggregate entries.
    pub details: Vec<String>,
}

#[derive(Debug, Serialize)]
/// One hygiene-policy violation.
pub struct HygieneViolation {
    /// Report entry or policy identifier that failed.
    pub id: String,
    /// Human-readable explanation of the violation.
    pub message: String,
}

#[derive(Debug, Default)]
/// Summary of one cleanup operation.
pub struct HygieneCleanResult {
    /// Number of bytes reclaimed by the cleanup.
    pub reclaimed_bytes: u64,
    /// Removed artifact roots.
    pub removed_paths: Vec<PathBuf>,
}

#[derive(Debug, Serialize)]
struct ArtifactRootManifest<'a> {
    schema: &'static str,
    kind: ManagedArtifactKind,
    repo_root: &'a str,
    safe_to_delete: bool,
    purpose: &'a str,
}

struct EntrySpec<'a> {
    id: &'a str,
    kind: &'a str,
    path: &'a Path,
    budget_bytes: Option<u64>,
    managed: bool,
    safe_to_delete: bool,
    details: Vec<String>,
}

/// Prepares the managed artifact roots for one command layout and returns the env paths to use.
pub fn prepare_artifact_layout(
    repo_root: &Path,
    layout: CommandArtifactLayout,
) -> DynResult<Option<(PathBuf, PathBuf)>> {
    match layout {
        CommandArtifactLayout::Inherit => Ok(None),
        CommandArtifactLayout::ManagedWorkspace => {
            let target_dir = cargo_target_dir(repo_root);
            let build_dir = cargo_build_dir(repo_root);
            prepare_managed_artifact_roots(
                repo_root,
                [
                    (
                        target_dir.as_path(),
                        ManagedArtifactKind::WorkspaceTarget,
                        "final Cargo artifacts for maintained workspace commands",
                    ),
                    (
                        build_dir.as_path(),
                        ManagedArtifactKind::WorkspaceBuild,
                        "intermediate Cargo build cache for maintained workspace commands",
                    ),
                ],
            )?;
            Ok(Some((target_dir, build_dir)))
        }
        CommandArtifactLayout::ManagedCoverage => {
            let target_dir = coverage_target_dir(repo_root);
            let build_dir = coverage_build_dir(repo_root);
            let cargo_target_dir = coverage_cargo_target_dir(repo_root);
            let cargo_build_dir = coverage_cargo_build_dir(repo_root);
            prepare_managed_artifact_roots(
                repo_root,
                [
                    (
                        target_dir.as_path(),
                        ManagedArtifactKind::CoverageTarget,
                        "managed coverage workspace root for the maintained llvm-cov gate",
                    ),
                    (
                        build_dir.as_path(),
                        ManagedArtifactKind::CoverageBuild,
                        "managed coverage build root for the maintained llvm-cov gate",
                    ),
                    (
                        cargo_target_dir.as_path(),
                        ManagedArtifactKind::CoverageTarget,
                        "nested Cargo target root created by cargo llvm-cov",
                    ),
                    (
                        cargo_build_dir.as_path(),
                        ManagedArtifactKind::CoverageBuild,
                        "nested Cargo build root created by cargo llvm-cov",
                    ),
                ],
            )?;
            Ok(Some((target_dir, build_dir)))
        }
    }
}

/// Prepares and returns the managed root that retains completed maintainer-gate evidence.
pub fn prepare_gate_report_root(repo_root: &Path) -> DynResult<PathBuf> {
    let root = gate_report_dir(repo_root);
    prepare_managed_artifact_roots(
        repo_root,
        [(
            root.as_path(),
            ManagedArtifactKind::GateReports,
            "retained stdout, stderr, and JSON evidence for completed maintainer-gate runs",
        )],
    )?;
    Ok(root)
}

/// Builds a full repository artifact report, including managed caches and legacy local roots.
pub fn hygiene_report(repo_root: &Path) -> DynResult<HygieneReport> {
    let tmp_root = repo_root.join("tmp");
    let legacy_target_root = repo_root.join("target");
    let tmp_cargo_roots = repo_tmp_cargo_roots(repo_root)?;
    let managed_target = cargo_target_dir(repo_root);
    let managed_build = cargo_build_dir(repo_root);
    let managed_coverage_target = coverage_target_dir(repo_root);
    let managed_coverage_build = coverage_build_dir(repo_root);
    let managed_gate_reports = gate_report_dir(repo_root);

    let tmp_cargo_entry = repo_tmp_cargo_entry(&tmp_root, &tmp_cargo_roots)?;

    let mut entries = managed_entries([
        (
            "managed-workspace-target",
            "workspace-target",
            managed_target.as_path(),
            MANAGED_TARGET_BUDGET_BYTES,
        ),
        (
            "managed-workspace-build",
            "workspace-build",
            managed_build.as_path(),
            MANAGED_BUILD_BUDGET_BYTES,
        ),
        (
            "managed-coverage-target",
            "coverage-target",
            managed_coverage_target.as_path(),
            MANAGED_COVERAGE_TARGET_BUDGET_BYTES,
        ),
        (
            "managed-coverage-build",
            "coverage-build",
            managed_coverage_build.as_path(),
            MANAGED_COVERAGE_BUILD_BUDGET_BYTES,
        ),
        (
            "managed-gate-reports",
            "gate-reports",
            managed_gate_reports.as_path(),
            MANAGED_GATE_REPORT_BUDGET_BYTES,
        ),
    ])?;
    entries.extend(unmanaged_entries([
        (
            "legacy-repo-target",
            "legacy-repo-target",
            legacy_target_root.as_path(),
            LEGACY_REPO_TARGET_BUDGET_BYTES,
            vec![
                "Legacy repo-local Cargo target tree. Use `cargo xtask hygiene clean --mode rebuildable` to reclaim it."
                    .to_owned(),
            ],
        ),
    ])?);
    entries.push(repo_tmp_entry(&tmp_root, &tmp_cargo_roots)?);
    entries.push(tmp_cargo_entry);

    let total_bytes = entries.iter().map(|entry| entry.bytes).sum();
    let violations = report_violations(&entries);
    entries.sort_by(|left, right| left.id.cmp(&right.id));

    Ok(HygieneReport {
        schema: SCHEMA_NAME,
        repo_root: repo_root.display().to_string(),
        total_bytes,
        entries,
        violations,
    })
}

/// Renders the report as human-readable text.
pub fn render_hygiene_report(report: &HygieneReport) -> String {
    let mut lines = vec![
        format!("schema: {}", report.schema),
        format!("repo_root: {}", report.repo_root),
        format!(
            "total_bytes: {} ({})",
            report.total_bytes,
            format_bytes(report.total_bytes)
        ),
        "entries:".to_owned(),
    ];

    for entry in &report.entries {
        let budget = entry
            .budget_bytes
            .map(format_bytes)
            .unwrap_or_else(|| "n/a".to_owned());
        lines.push(format!(
            "- {} | {} | {} | present={} | bytes={} ({}) | budget={} | managed={} | safe_to_delete={}",
            entry.id,
            entry.kind,
            entry.path,
            entry.present,
            entry.bytes,
            format_bytes(entry.bytes),
            budget,
            entry.managed,
            entry.safe_to_delete
        ));
        for detail in &entry.details {
            lines.push(format!("  detail: {detail}"));
        }
    }

    if report.violations.is_empty() {
        lines.push("violations: none".to_owned());
    } else {
        lines.push("violations:".to_owned());
        for violation in &report.violations {
            lines.push(format!("- {}: {}", violation.id, violation.message));
        }
    }

    lines.join("\n")
}

/// Fails when the current repository violates the maintained hygiene policy.
pub fn ensure_hygiene(repo_root: &Path) -> DynResult<()> {
    let started = Instant::now();
    let report = hygiene_report(repo_root)?;
    let result: DynResult<()> = if report.violations.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "artifact hygiene policy failed.\n{}\n\nRepair with `cargo xtask hygiene report` and `cargo xtask hygiene clean --mode rebuildable`.",
            render_hygiene_report(&report)
        )
        .into())
    };

    if crate::gate_report::is_active() {
        crate::gate_report::record_internal_check(
            "Artifact hygiene policy",
            result
                .as_ref()
                .map(|_| ())
                .map_err(|error| error.to_string()),
            started.elapsed(),
        );
    }

    result
}

/// Removes disposable artifact roots according to the requested cleanup mode.
pub fn clean_hygiene(repo_root: &Path, mode: HygieneCleanMode) -> DynResult<HygieneCleanResult> {
    let mut removal_roots = vec![
        coverage_target_dir(repo_root),
        coverage_build_dir(repo_root),
        semver_scratch_dir(repo_root),
        repo_root.join("tmp"),
        repo_root.join("target"),
        repo_root.join("target").join("llvm-cov-target"),
        repo_root.join("target").join("semver-checks"),
    ];

    if mode == HygieneCleanMode::Rebuildable {
        removal_roots.push(cargo_target_dir(repo_root));
        removal_roots.push(cargo_build_dir(repo_root));
        removal_roots.push(gate_report_dir(repo_root));
    }

    let removal_roots = deduplicate_root_set(removal_roots);
    let mut result = HygieneCleanResult::default();

    for path in removal_roots {
        if !path.exists() {
            continue;
        }

        let bytes = dir_size_bytes(&path)?;
        remove_dir_if_exists(&path).map_err(|error| {
            format!(
                "failed to remove hygiene artifact root {}: {error}",
                path.display()
            )
        })?;
        result.reclaimed_bytes += bytes;
        result.removed_paths.push(path);
    }

    Ok(result)
}

mod support;

#[cfg(all(test, unix))]
pub(crate) use self::support::{
    aggregate_entry_for_tests, dir_size_bytes_for_tests, dir_size_bytes_result_for_tests,
};
use self::support::{
    deduplicate_root_set, dir_size_bytes, format_bytes, managed_entries,
    prepare_managed_artifact_roots, repo_tmp_cargo_entry, repo_tmp_cargo_roots, repo_tmp_entry,
    report_violations, unmanaged_entries,
};
#[cfg(test)]
pub(crate) use self::support::{format_bytes_for_tests, looks_like_cargo_target_dir_for_tests};
