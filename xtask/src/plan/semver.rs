use std::fs;
use std::path::Path;

use crate::{model::DynResult, package_version_from_manifest, workspace_version};

use super::paths::semver_baseline_path;

/// Infers the semver release type that `cargo semver-checks` should enforce.
pub fn semver_release_type(repo_root: &Path) -> DynResult<String> {
    let workspace_version = workspace_version(repo_root)?;
    let baseline_manifest_path = semver_baseline_path(repo_root).join("Cargo.toml");
    let baseline_manifest = fs::read_to_string(baseline_manifest_path)?;
    let baseline_version = package_version_from_manifest(&baseline_manifest)?;
    Ok(semver_release_type_from_versions(
        &workspace_version,
        &baseline_version,
    ))
}

/// Maps the workspace and baseline versions to the semver release type checked in CI.
pub fn semver_release_type_from_versions(
    workspace_version: &str,
    baseline_version: &str,
) -> String {
    if workspace_version == baseline_version {
        "minor".to_owned()
    } else {
        "major".to_owned()
    }
}

/// Adds a minimal workspace stub to isolated manifests used by the semver baseline flow.
pub fn with_workspace_stub(cargo_toml: &str) -> String {
    if cargo_toml.contains("\n[workspace]\n") {
        return cargo_toml.to_owned();
    }

    format!("{cargo_toml}\n[workspace]\n")
}

/// Removes dev-dependency tables from a manifest used only for semver-baseline packaging.
pub fn strip_dev_dependency_tables(cargo_toml: &str) -> String {
    let mut sanitized = Vec::new();
    let mut skipping = false;

    for line in cargo_toml.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            skipping = trimmed.contains("dev-dependencies");
            if skipping {
                continue;
            }
        }

        if !skipping {
            sanitized.push(line);
        }
    }

    let mut result = sanitized.join("\n");
    if cargo_toml.ends_with('\n') {
        result.push('\n');
    }
    result
}
