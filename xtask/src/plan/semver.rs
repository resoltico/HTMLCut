use std::fs;
use std::path::Path;

use crate::{model::DynResult, package_version_from_manifest, workspace_version};
use toml::Value;

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

/// Rewrites vendored dependencies only for Cargo's temporary package-normalization input.
///
/// The resulting manifest is never the checked-in baseline: after Cargo has produced the core
/// package layout, the refresh flow restores the published vendored dependency paths and copies
/// their tagged source into that baseline.
pub fn sanitize_snapshot_workspace_manifest_for_packaging(cargo_toml: &str) -> DynResult<String> {
    let mut manifest = toml::from_str::<Value>(cargo_toml)
        .map_err(|error| crate::model::XtaskError::invalid_toml("snapshot Cargo.toml", error))?;
    let Some(workspace) = manifest.get_mut("workspace").and_then(Value::as_table_mut) else {
        return Ok(cargo_toml.to_owned());
    };
    let Some(dependencies) = workspace
        .get_mut("dependencies")
        .and_then(Value::as_table_mut)
    else {
        return Ok(cargo_toml.to_owned());
    };

    for (_, dependency) in dependencies.iter_mut() {
        let Some(table) = dependency.as_table_mut() else {
            continue;
        };
        sanitize_vendored_workspace_dependency_table(table);
    }

    Ok(toml::to_string_pretty(&manifest)?)
}

/// Returns whether a released workspace ships the vendored selector/parser stack.
pub fn snapshot_uses_vendored_selector_stack(cargo_toml: &str) -> DynResult<bool> {
    let manifest = toml::from_str::<Value>(cargo_toml)
        .map_err(|error| crate::model::XtaskError::invalid_toml("snapshot Cargo.toml", error))?;
    let Some(workspace) = manifest.get("workspace").and_then(Value::as_table) else {
        return Ok(false);
    };
    let Some(dependencies) = workspace.get("dependencies").and_then(Value::as_table) else {
        return Ok(false);
    };

    Ok(dependencies
        .values()
        .any(is_vendored_selector_stack_dependency))
}

/// Restores HTMLCut's published vendored selector-stack dependencies in a packaged baseline.
///
/// Cargo package normalization removes path dependencies, but `htmlcut-core`'s public API is
/// compiled against the repository-owned `htmlcut-*` forks. The checked-in semver baseline must
/// therefore point at its copied tagged forks rather than an upstream registry lookalike.
pub fn restore_vendored_dependency_paths_in_baseline_manifest(
    cargo_toml: &str,
) -> DynResult<Option<String>> {
    let mut manifest = toml::from_str::<Value>(cargo_toml)
        .map_err(|error| crate::model::XtaskError::invalid_toml("baseline Cargo.toml", error))?;
    let Some(dependencies) = manifest
        .get_mut("dependencies")
        .and_then(Value::as_table_mut)
    else {
        return Ok(None);
    };

    let restored = [
        (
            "scraper",
            "htmlcut-scraper",
            "vendor/scraper",
            "0.27.0-htmlcut.1",
        ),
        (
            "selectors",
            "htmlcut-selectors",
            "vendor/selectors",
            "0.38.0-htmlcut.1",
        ),
    ]
    .into_iter()
    .filter(|(alias, package, path, version)| {
        restore_vendored_baseline_dependency(dependencies, alias, package, path, version)
    })
    .count();

    if restored == 0 {
        return Ok(None);
    }

    Ok(Some(toml::to_string_pretty(&manifest)?))
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

fn sanitize_vendored_workspace_dependency_table(table: &mut toml::Table) {
    let package_name = table.get("package").and_then(Value::as_str);
    let path = table.get("path").and_then(Value::as_str);
    let Some(package_name) = package_name else {
        return;
    };
    let Some(path) = path else {
        return;
    };

    if !is_vendored_selector_stack_package(package_name, path) {
        return;
    }

    table.remove("package");
    table.remove("path");

    let version = table
        .get("version")
        .and_then(Value::as_str)
        .map(unvendor_dependency_version);
    version.into_iter().for_each(|version| {
        table.insert("version".to_owned(), Value::String(version));
    });
}

fn is_vendored_selector_stack_dependency(dependency: &Value) -> bool {
    let Some(table) = dependency.as_table() else {
        return false;
    };
    let Some(package_name) = table.get("package").and_then(Value::as_str) else {
        return false;
    };
    let Some(path) = table.get("path").and_then(Value::as_str) else {
        return false;
    };
    is_vendored_selector_stack_package(package_name, path)
}

fn is_vendored_selector_stack_package(package_name: &str, path: &str) -> bool {
    package_name.starts_with("htmlcut-") && path.starts_with("patches/rust/")
}

fn unvendor_dependency_version(version: &str) -> String {
    version
        .split("-htmlcut.")
        .next()
        .unwrap_or(version)
        .to_owned()
}

fn restore_vendored_baseline_dependency(
    dependencies: &mut toml::Table,
    alias: &str,
    package: &str,
    path: &str,
    version: &str,
) -> bool {
    let Some(dependency) = dependencies.get_mut(alias) else {
        return false;
    };
    let Some(table) = dependency.as_table_mut() else {
        return false;
    };

    table.insert("package".to_owned(), Value::String(package.to_owned()));
    table.insert("path".to_owned(), Value::String(path.to_owned()));
    table.insert("version".to_owned(), Value::String(version.to_owned()));
    true
}
