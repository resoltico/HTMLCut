use std::fs;
use std::path::Path;

use crate::model::DynResult;

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
    manifest_field_from_section(manifest, "[workspace.package]", "version")
        .ok_or_else(|| "workspace version not found in Cargo.toml".into())
}

/// Extracts the workspace Rust-version floor from a root `Cargo.toml` string.
pub fn workspace_rust_version_from_manifest(manifest: &str) -> DynResult<String> {
    manifest_field_from_section(manifest, "[workspace.package]", "rust-version")
        .ok_or_else(|| "workspace rust-version not found in Cargo.toml".into())
}

/// Extracts the crate package version from a package `Cargo.toml` string.
pub fn package_version_from_manifest(manifest: &str) -> DynResult<String> {
    manifest_field_from_section(manifest, "[package]", "version")
        .ok_or_else(|| "package version not found in Cargo.toml".into())
}

/// Extracts the workspace member paths from a root `Cargo.toml` string.
pub(crate) fn workspace_members_from_manifest(manifest: &str) -> DynResult<Vec<String>> {
    let mut in_workspace = false;
    let mut in_members = false;
    let mut members = Vec::new();

    for raw_line in manifest.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            in_workspace = line == "[workspace]";
            in_members = false;
            continue;
        }

        if !in_workspace {
            continue;
        }

        if !in_members {
            if let Some(rest) = line.strip_prefix("members") {
                let Some(rest) = rest.trim_start().strip_prefix('=') else {
                    continue;
                };
                let rest = rest.trim_start();
                let Some(rest) = rest.strip_prefix('[') else {
                    continue;
                };
                in_members = true;
                collect_quoted_values(rest, &mut members);
                if rest.contains(']') {
                    break;
                }
            }
            continue;
        }

        collect_quoted_values(line, &mut members);
        if line.contains(']') {
            break;
        }
    }

    if members.is_empty() {
        return Err("workspace members not found in Cargo.toml".into());
    }

    Ok(members)
}

fn manifest_field_from_section(
    manifest: &str,
    section_name: &str,
    field_name: &str,
) -> Option<String> {
    let mut in_section = false;

    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_section = trimmed == section_name;
            continue;
        }

        if !in_section {
            continue;
        }

        if let Some(value) = assignment_value(trimmed, field_name) {
            return Some(value);
        }
    }

    None
}

fn assignment_value(line: &str, field_name: &str) -> Option<String> {
    let remainder = line.strip_prefix(field_name)?.trim_start();
    let remainder = remainder.strip_prefix('=')?.trim_start();
    let remainder = remainder.strip_prefix('"')?;
    let closing_quote = remainder.find('"')?;
    Some(remainder[..closing_quote].to_owned())
}

fn collect_quoted_values(line: &str, values: &mut Vec<String>) {
    let mut cursor = line;
    while let Some(start) = cursor.find('"') {
        let after_start = &cursor[start + 1..];
        let Some(end) = after_start.find('"') else {
            break;
        };
        let value = after_start[..end].to_owned();
        if !values.contains(&value) {
            values.push(value);
        }
        cursor = &after_start[end + 1..];
    }
}
