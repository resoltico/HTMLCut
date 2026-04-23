//! Shared maintenance primitives behind HTMLCut's `cargo xtask` workflows.
#![deny(missing_docs)]

mod command_exec;
mod coverage;
mod docs;
mod fuzz;
mod host_tools;
mod manifest;
mod model;
mod plan;
mod policy;
mod preflight;
mod release;
#[cfg(test)]
mod tests;
mod toolchain;

pub use command_exec::{capture_command_output, remove_dir_if_exists, repo_root, run_spec};
pub use coverage::{
    coverage_clean_command, coverage_command, coverage_output_path, coverage_preflight_failures,
    coverage_preflight_message, ensure_coverage_output_dir, evaluate_coverage_report,
    read_coverage_report, tracked_files,
};
pub use docs::{markdown_contract_errors, markdown_doc_paths};
pub use fuzz::{
    DEFAULT_FUZZ_SMOKE_RUNS, FuzzSmokePreflightFailure, assert_known_fuzz_target,
    cargo_fuzz_probe_command, fuzz_corpus_dir, fuzz_smoke_command, fuzz_smoke_preflight_failures,
    fuzz_smoke_preflight_message, fuzz_smoke_targets, stage_fuzz_corpus,
};
pub use host_tools::{
    HostToolPreflightFailure, host_tool_preflight_failures, host_tool_preflight_message,
    host_tool_probe_command,
};
pub use manifest::{
    package_version_from_manifest, workspace_rust_version, workspace_rust_version_from_manifest,
    workspace_version, workspace_version_from_manifest,
};
pub use model::{
    CommandSpec, CoverageBranchRecord, CoverageCounter, CoverageDataSet, CoverageFailure,
    CoverageFile, CoverageFileSummary, CoveragePreflightFailure, CoverageReport, CoverageSummary,
    DynResult,
};
pub use plan::{
    binary_name, check_plan, core_manifest_path, is_semver_check_spec, normalize_path,
    release_binary_path, semver_baseline_path, semver_release_type,
    semver_release_type_from_versions, semver_scratch_dir, shell_script_paths,
    strip_dev_dependency_tables, with_workspace_stub,
};
pub use policy::{deny_check_command, deny_graph_targets};
pub use preflight::{
    ensure_coverage_prerequisites, ensure_fuzz_smoke_prerequisites,
    ensure_repo_toolchain_prerequisites,
};
pub use release::{
    ReleaseMatrixEntry, macos_deployment_target, release_asset_names, release_matrix,
    release_target_triples,
};
pub use toolchain::{
    RepoToolchain, RepoToolchainPreflightFailure, repo_toolchain,
    repo_toolchain_component_probe_command, repo_toolchain_from_manifest,
    repo_toolchain_preflight_failures, repo_toolchain_preflight_message,
};

#[cfg(test)]
pub(crate) use model::*;
