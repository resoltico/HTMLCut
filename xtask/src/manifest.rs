use std::fs;
use std::path::Path;

use serde::Deserialize;

use crate::model::{DynResult, XtaskError};

#[derive(Debug, Deserialize)]
struct CargoManifest {
    workspace: Option<WorkspaceSection>,
    package: Option<PackageSection>,
}

#[derive(Debug, Deserialize)]
struct WorkspaceSection {
    package: Option<WorkspacePackageSection>,
    #[serde(default)]
    members: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct WorkspacePackageSection {
    version: Option<String>,
    #[serde(rename = "rust-version")]
    rust_version: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PackageSection {
    version: Option<String>,
}

/// Reads the workspace version from the root manifest.
pub fn workspace_version(repo_root: &Path) -> DynResult<String> {
    workspace_version_from_manifest(&fs::read_to_string(repo_root.join("Cargo.toml"))?)
}

/// Reads the workspace Rust-version floor from the root manifest.
pub fn workspace_rust_version(repo_root: &Path) -> DynResult<String> {
    workspace_rust_version_from_manifest(&fs::read_to_string(repo_root.join("Cargo.toml"))?)
}

/// Reads the workspace member paths from the root manifest.
pub(crate) fn workspace_members(repo_root: &Path) -> DynResult<Vec<String>> {
    workspace_members_from_manifest(&fs::read_to_string(repo_root.join("Cargo.toml"))?)
}

/// Extracts the workspace version from a root `Cargo.toml` string.
pub fn workspace_version_from_manifest(manifest: &str) -> DynResult<String> {
    parse_manifest(manifest)?
        .workspace
        .and_then(|workspace| workspace.package)
        .and_then(|package| package.version)
        .ok_or_else(|| "workspace version not found in Cargo.toml".into())
}

/// Extracts the workspace Rust-version floor from a root `Cargo.toml` string.
pub fn workspace_rust_version_from_manifest(manifest: &str) -> DynResult<String> {
    parse_manifest(manifest)?
        .workspace
        .and_then(|workspace| workspace.package)
        .and_then(|package| package.rust_version)
        .ok_or_else(|| "workspace rust-version not found in Cargo.toml".into())
}

/// Extracts the crate package version from a package `Cargo.toml` string.
pub fn package_version_from_manifest(manifest: &str) -> DynResult<String> {
    parse_manifest(manifest)?
        .package
        .and_then(|package| package.version)
        .ok_or_else(|| "package version not found in Cargo.toml".into())
}

/// Extracts the workspace member paths from a root `Cargo.toml` string.
pub(crate) fn workspace_members_from_manifest(manifest: &str) -> DynResult<Vec<String>> {
    let members = parse_manifest(manifest)?
        .workspace
        .map(|workspace| workspace.members)
        .unwrap_or_default();
    if members.is_empty() {
        return Err("workspace members not found in Cargo.toml".into());
    }

    Ok(members)
}

fn parse_manifest(manifest: &str) -> DynResult<CargoManifest> {
    toml::from_str(manifest).map_err(|source| XtaskError::invalid_toml("Cargo.toml", source))
}
