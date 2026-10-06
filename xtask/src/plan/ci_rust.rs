// SPDX-License-Identifier: MPL-2.0
//! Curated Rust verification executed on every supported native CI platform.

use std::path::Path;

use super::check::{
    all_features_test_specs, ensure_clean_semver_baseline, format_check_command,
    maintained_fork_quality_specs, semver_check_command, workspace_audit_command,
    workspace_clippy_command, workspace_outdated_command,
};
use super::semver::semver_release_type;
use crate::model::{
    CommandArtifactLayout, CommandSpec, CommandStdout, CommandToolchainEnv, DynResult,
};
use crate::{deny_check_command, ensure_deny_targets_match_release_targets};

/// Builds the curated Rust gate executed by cross-platform CI jobs.
pub fn ci_rust_gate_plan(repo_root: &Path) -> DynResult<Vec<CommandSpec>> {
    ensure_clean_semver_baseline(repo_root)?;
    ensure_deny_targets_match_release_targets(repo_root)?;
    let semver_release_type = semver_release_type(repo_root)?;

    let mut plan = vec![format_check_command(), workspace_clippy_command()];
    plan.extend(maintained_fork_quality_specs());
    plan.extend(all_features_test_specs());
    plan.push(workspace_outdated_command());
    plan.push(workspace_audit_command());
    plan.push(deny_check_command(repo_root)?);
    plan.push(
        CommandSpec::new(
            "cargo",
            [
                "test",
                "-p",
                "xtask",
                "--lib",
                "--all-features",
                "--locked",
                "hygiene::source_boundary::tests",
            ],
            CommandStdout::Inherit,
            CommandToolchainEnv::Inherit,
        )
        .with_artifact_layout(CommandArtifactLayout::ManagedWorkspace),
    );
    plan.push(semver_check_command(repo_root, &semver_release_type));
    Ok(plan)
}
