use std::fs;
use std::path::Path;

use htmlcut_tempdir::tempdir;

use crate::manifest::workspace_members;
use crate::model::{
    CommandArtifactLayout, CommandSpec, CommandStdout, CommandToolchainEnv, DynResult, XtaskError,
};
use crate::run_spec;

/// Builds the maintained dependency-freshness gate command.
pub fn outdated_check_command() -> CommandSpec {
    CommandSpec::new(
        "cargo",
        ["run", "-p", "xtask", "--", "outdated-check"],
        CommandStdout::Inherit,
        CommandToolchainEnv::Inherit,
    )
    .with_artifact_layout(CommandArtifactLayout::ManagedWorkspace)
}

/// Runs the maintained dependency-freshness gate through a sanitized workspace snapshot.
pub fn run_outdated_check(repo_root: &Path) -> DynResult<()> {
    let scratch = tempdir()?;
    let snapshot_root = scratch.path().join("workspace");
    materialize_outdated_workspace(repo_root, &snapshot_root)?;
    run_spec(
        repo_root,
        &cargo_outdated_command(&snapshot_root.join("Cargo.toml")),
    )
}

fn cargo_outdated_command(manifest_path: &Path) -> CommandSpec {
    CommandSpec::new(
        "cargo",
        [
            "outdated",
            "--workspace",
            "--root-deps-only",
            "--exit-code",
            "1",
            "--manifest-path",
            manifest_path.to_string_lossy().as_ref(),
        ],
        CommandStdout::Inherit,
        CommandToolchainEnv::Inherit,
    )
    .with_artifact_layout(CommandArtifactLayout::ManagedWorkspace)
}

fn materialize_outdated_workspace(repo_root: &Path, snapshot_root: &Path) -> DynResult<()> {
    fs::create_dir_all(snapshot_root)?;
    let root_manifest = fs::read_to_string(repo_root.join("Cargo.toml"))?;
    fs::write(
        snapshot_root.join("Cargo.toml"),
        strip_patch_crates_io(&root_manifest)?,
    )?;

    for member in workspace_members(repo_root)? {
        copy_member_package_layout(&repo_root.join(&member), &snapshot_root.join(&member))?;
    }

    Ok(())
}

fn copy_member_package_layout(source_root: &Path, destination_root: &Path) -> DynResult<()> {
    fs::create_dir_all(destination_root)?;
    let manifest = fs::read_to_string(source_root.join("Cargo.toml"))?;
    fs::write(
        destination_root.join("Cargo.toml"),
        strip_patch_crates_io(&manifest)?,
    )?;

    // Flat upstream carriers have lib.rs and sibling modules at the package root.
    for entry in fs::read_dir(source_root)? {
        let entry = entry?;
        if entry.file_type()?.is_file()
            && entry
                .path()
                .extension()
                .is_some_and(|extension| extension == "rs")
        {
            copy_file(&entry.path(), &destination_root.join(entry.file_name()))?;
        }
    }

    for directory in ["src", "tests", "examples", "benches", "fuzz_targets"] {
        let source_dir = source_root.join(directory);
        if source_dir.exists() {
            copy_dir_recursively(&source_dir, &destination_root.join(directory))?;
        }
    }

    let build_script = source_root.join("build.rs");
    if build_script.exists() {
        copy_file(&build_script, &destination_root.join("build.rs"))?;
    }

    Ok(())
}

fn copy_dir_recursively(source: &Path, destination: &Path) -> DynResult<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let entry_type = entry.file_type()?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if entry_type.is_dir() {
            copy_dir_recursively(&source_path, &destination_path)?;
        } else if entry_type.is_file() {
            copy_file(&source_path, &destination_path)?;
        }
    }

    Ok(())
}

fn copy_file(source: &Path, destination: &Path) -> DynResult<()> {
    let parent = destination.parent().ok_or_else(|| {
        format!(
            "manifest destination has no parent: {}",
            destination.display()
        )
    })?;
    fs::create_dir_all(parent)?;
    fs::copy(source, destination)?;
    Ok(())
}

fn strip_patch_crates_io(manifest: &str) -> DynResult<String> {
    let mut value = toml::from_str::<toml::Value>(manifest)
        .map_err(|source| XtaskError::invalid_toml("Cargo.toml", source))?;
    if let Some(toml::Value::Table(patch_table)) = value.get_mut("patch") {
        patch_table.remove("crates-io");
        if patch_table.is_empty() {
            value
                .as_table_mut()
                .expect("root manifest should deserialize as a table")
                .remove("patch");
        }
    }
    sanitize_repo_owned_workspace_dependencies(&mut value);
    sanitize_member_dependencies(&mut value);

    Ok(toml::to_string(&value)?)
}

fn sanitize_member_dependencies(value: &mut toml::Value) {
    for group in ["dependencies", "dev-dependencies", "build-dependencies"] {
        if let Some(table) = value.get_mut(group).and_then(toml::Value::as_table_mut) {
            for (alias, dependency) in table.iter_mut() {
                sanitize_repo_owned_dependency(alias, dependency);
            }
        }
    }
    if let Some(targets) = value.get_mut("target").and_then(toml::Value::as_table_mut) {
        for (_, target) in targets.iter_mut() {
            sanitize_member_dependencies(target);
        }
    }
}

fn sanitize_repo_owned_workspace_dependencies(value: &mut toml::Value) {
    let Some(workspace_table) = value
        .get_mut("workspace")
        .and_then(toml::Value::as_table_mut)
    else {
        return;
    };
    let Some(dependencies_table) = workspace_table
        .get_mut("dependencies")
        .and_then(toml::Value::as_table_mut)
    else {
        return;
    };

    for (dependency_name, dependency_value) in dependencies_table.iter_mut() {
        sanitize_repo_owned_dependency(dependency_name, dependency_value);
    }
}

fn sanitize_repo_owned_dependency(dependency_name: &str, dependency_value: &mut toml::Value) {
    let Some(dependency_table) = dependency_value.as_table_mut() else {
        return;
    };
    let Some(package_name) = dependency_table
        .get("package")
        .and_then(toml::Value::as_str)
    else {
        return;
    };
    let Some(path) = dependency_table.get("path").and_then(toml::Value::as_str) else {
        return;
    };
    let Some(version) = dependency_table
        .get("version")
        .and_then(toml::Value::as_str)
        .map(str::to_owned)
    else {
        return;
    };

    if !package_name.starts_with("htmlcut-")
        || !(path.starts_with("patches/rust/") || path.starts_with("../"))
        || !version.contains("-htmlcut.")
    {
        return;
    }

    let upstream = package_name
        .strip_prefix("htmlcut-")
        .unwrap()
        .replace("servo-arc", "servo_arc");
    if dependency_name == upstream {
        dependency_table.remove("package");
    } else {
        dependency_table.insert("package".into(), toml::Value::String(upstream));
    }
    dependency_table.remove("path");
    dependency_table.insert(
        "version".to_owned(),
        toml::Value::String(version_base(&version).to_owned()),
    );
}

fn version_base(version: &str) -> &str {
    version
        .split_once("-htmlcut.")
        .map_or(version, |(base, _)| base)
}

#[cfg(test)]
pub(crate) fn materialize_outdated_workspace_for_tests(
    repo_root: &Path,
    snapshot_root: &Path,
) -> DynResult<()> {
    materialize_outdated_workspace(repo_root, snapshot_root)
}

#[cfg(test)]
pub(crate) fn strip_patch_crates_io_for_tests(manifest: &str) -> DynResult<String> {
    strip_patch_crates_io(manifest)
}

#[cfg(test)]
pub(crate) fn sanitize_repo_owned_workspace_dependencies_for_tests(value: &mut toml::Value) {
    sanitize_repo_owned_workspace_dependencies(value);
}

#[cfg(test)]
pub(crate) fn copy_member_package_layout_for_tests(
    source_root: &Path,
    destination_root: &Path,
) -> DynResult<()> {
    copy_member_package_layout(source_root, destination_root)
}

#[cfg(test)]
pub(crate) fn copy_file_for_tests(source: &Path, destination: &Path) -> DynResult<()> {
    copy_file(source, destination)
}
