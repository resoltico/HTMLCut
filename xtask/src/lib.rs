//! Shared maintenance primitives behind HTMLCut's `cargo xtask` workflows.
#![deny(missing_docs)]

mod coverage;
mod model;
mod plan;
#[cfg(test)]
mod tests;

pub use coverage::{
    coverage_clean_command, coverage_command, coverage_output_path, coverage_preflight_failures,
    coverage_preflight_message, evaluate_coverage_report, read_coverage_report, tracked_files,
};
pub use model::{
    CommandSpec, CoverageBranchRecord, CoverageCounter, CoverageDataSet, CoverageFailure,
    CoverageFile, CoverageFileSummary, CoveragePreflightFailure, CoverageReport, CoverageSummary,
    DynResult,
};
pub use plan::{
    binary_name, check_plan, core_manifest_path, fuzz_manifest_path, is_semver_check_spec,
    normalize_path, release_binary_path, semver_baseline_path, semver_release_type,
    semver_release_type_from_versions, semver_scratch_dir, shell_script_paths, with_workspace_stub,
    workspace_version, workspace_version_from_manifest,
};

#[cfg(test)]
pub(crate) use model::*;
