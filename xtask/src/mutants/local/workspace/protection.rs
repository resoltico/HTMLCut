//! Private Cargo-home and source-integrity boundaries for one disposable mutation worker.

use std::env;
use std::fs;
use std::fs::FileType;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use crate::DynResult;

/// Cache trees safely shared read-only by isolated offline worker Cargo homes.
const SHARED_CARGO_HOME_DIRECTORIES: &[&str] = &["registry", "git"];
/// Configuration that selects registries but does not contain credentials.
const SHARED_CARGO_HOME_CONFIG_FILES: &[&str] = &["config.toml"];
const WORKER_CARGO_HOME_DIRECTORY: &str = ".htmlcut-mutant-cargo/cargo-home";

/// Evidence that a worker source tree passed through the non-deleting protection boundary.
pub(super) struct ProtectedSourceTree;
/// Evidence that a worker source tree was restored after its mutation process exited.
pub(super) struct RestoredSourceTree;
/// Evidence that one private Cargo home links a verified shared cache directory.
struct LinkedDirectory;

enum SourceEntryKind {
    Directory,
    File,
}

pub(super) fn prepare_worker_cargo_home_at(workspace_root: &Path) -> DynResult<PathBuf> {
    let source = shared_cargo_home()?;
    let destination = workspace_root.join(WORKER_CARGO_HOME_DIRECTORY);
    prepare_worker_cargo_home_from(&source, &destination)?;
    Ok(destination)
}

pub(super) fn protect_worker_source_tree(workspace_root: &Path) -> DynResult<ProtectedSourceTree> {
    #[cfg(unix)]
    {
        protect_source_directory(workspace_root, true)?;
        Ok(ProtectedSourceTree)
    }
    #[cfg(not(unix))]
    {
        let _ = workspace_root;
        Err("safe local mutation workspaces require Unix directory permissions".into())
    }
}

pub(super) fn restore_worker_source_tree(workspace_root: &Path) -> DynResult<RestoredSourceTree> {
    #[cfg(unix)]
    {
        restore_source_directory(workspace_root, true)?;
        Ok(RestoredSourceTree)
    }
    #[cfg(not(unix))]
    {
        let _ = workspace_root;
        Err("safe local mutation workspaces require Unix directory permissions".into())
    }
}

#[cfg(unix)]
fn protect_source_directory(directory: &Path, is_root: bool) -> DynResult<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        if is_root && entry.file_name() == ".htmlcut-mutant-cargo" {
            continue;
        }
        match source_entry_kind(entry.file_type()?, &path)? {
            SourceEntryKind::Directory => {
                protect_source_directory(&path, false)?;
                remove_unix_write_permission(&path)?;
            }
            SourceEntryKind::File => {
                if is_mutable_mutation_source(&path) {
                    add_unix_owner_write_permission(&path)?;
                } else {
                    remove_unix_write_permission(&path)?;
                }
            }
        }
    }
    remove_unix_write_permission(directory)
}

fn is_mutable_mutation_source(path: &Path) -> bool {
    path.extension().is_some_and(|extension| extension == "rs")
}

#[cfg(unix)]
fn restore_source_directory(directory: &Path, is_root: bool) -> DynResult<()> {
    add_unix_owner_write_permission(directory)?;
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        if is_root && entry.file_name() == ".htmlcut-mutant-cargo" {
            continue;
        }
        match source_entry_kind(entry.file_type()?, &path)? {
            SourceEntryKind::Directory => restore_source_directory(&path, false)?,
            SourceEntryKind::File => add_unix_owner_write_permission(&path)?,
        }
    }
    Ok(())
}

fn source_entry_kind(file_type: FileType, path: &Path) -> DynResult<SourceEntryKind> {
    if file_type.is_dir() {
        return Ok(SourceEntryKind::Directory);
    }
    if file_type.is_file() {
        return Ok(SourceEntryKind::File);
    }
    Err(format!(
        "mutation workspace source is not a regular file or directory: {}",
        path.display()
    )
    .into())
}

#[cfg(unix)]
fn remove_unix_write_permission(path: &Path) -> DynResult<()> {
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(permissions.mode() & !0o222);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

#[cfg(unix)]
fn add_unix_owner_write_permission(path: &Path) -> DynResult<()> {
    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(permissions.mode() | 0o200);
    fs::set_permissions(path, permissions)?;
    Ok(())
}

fn shared_cargo_home() -> DynResult<PathBuf> {
    shared_cargo_home_from_sources(
        env::var_os("CARGO_HOME"),
        env::var_os("HOME"),
        env::var_os("USERPROFILE"),
    )
}

fn shared_cargo_home_from_sources(
    cargo_home: Option<std::ffi::OsString>,
    home: Option<std::ffi::OsString>,
    user_profile: Option<std::ffi::OsString>,
) -> DynResult<PathBuf> {
    if let Some(cargo_home) = cargo_home.filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(cargo_home));
    }

    home.or(user_profile)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .map(|home| home.join(".cargo"))
        .ok_or_else(|| "could not determine the shared Cargo home".into())
}

fn prepare_worker_cargo_home_from(source: &Path, destination: &Path) -> DynResult<()> {
    if !source.is_dir() {
        return Err(format!("shared Cargo home is not a directory: {}", source.display()).into());
    }
    fs::create_dir_all(destination)?;
    for file_name in SHARED_CARGO_HOME_CONFIG_FILES {
        let source_file = source.join(file_name);
        if source_file.is_file() {
            fs::copy(&source_file, destination.join(file_name)).map_err(|error| {
                format!(
                    "failed to copy Cargo configuration {} into worker home: {error}",
                    source_file.display()
                )
            })?;
        }
    }
    for directory_name in SHARED_CARGO_HOME_DIRECTORIES {
        let source_directory = source.join(directory_name);
        if !source_directory.exists() {
            continue;
        }
        if !source_directory.is_dir() {
            return Err(format!(
                "shared Cargo cache member is not a directory: {}",
                source_directory.display()
            )
            .into());
        }
        link_directory(&source_directory, &destination.join(directory_name))?;
    }
    Ok(())
}

fn link_directory(source: &Path, destination: &Path) -> DynResult<LinkedDirectory> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(source, destination).map_err(|error| -> crate::XtaskError {
            format!(
                "failed to link shared Cargo cache {} into worker home {}: {error}",
                source.display(),
                destination.display()
            )
            .into()
        })?;
        Ok(LinkedDirectory)
    }
    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_dir(source, destination).map_err(
            |error| -> crate::XtaskError {
                format!(
                    "failed to link shared Cargo cache {} into worker home {}: {error}",
                    source.display(),
                    destination.display()
                )
                .into()
            },
        )?;
        Ok(LinkedDirectory)
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (source, destination);
        Err("safe local mutation workspaces require directory-link support".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use htmlcut_tempdir::tempdir;

    #[cfg(unix)]
    #[test]
    fn worker_cargo_home_uses_private_locks_and_shared_cached_registry() {
        let source = tempdir().expect("shared Cargo home");
        let destination = tempdir().expect("worker parent");
        fs::create_dir_all(source.path().join("registry/cache"))
            .expect("create shared registry cache");
        fs::write(
            source.path().join("registry/cache/package"),
            "cached package",
        )
        .expect("write shared package");
        fs::write(
            source.path().join("config.toml"),
            "[net]\noffline = false\n",
        )
        .expect("write Cargo config");

        let worker_home = destination.path().join("cargo-home");
        prepare_worker_cargo_home_from(source.path(), &worker_home)
            .expect("prepare private worker Cargo home");

        assert!(
            fs::symlink_metadata(worker_home.join("registry"))
                .expect("worker registry metadata")
                .file_type()
                .is_symlink()
        );
        assert_eq!(
            fs::read_to_string(worker_home.join("registry/cache/package"))
                .expect("read shared cached package"),
            "cached package"
        );
        assert!(!worker_home.join(".package-cache-mutate").exists());
    }

    #[cfg(unix)]
    #[test]
    fn protected_worker_source_tree_allows_mutation_but_blocks_source_deletion() {
        let root = tempdir().expect("worker root");
        let workspace = root.path().join("workspace");
        let mutable_root = workspace.join(".htmlcut-mutant-cargo");
        let source_file = workspace.join("src/lib.rs");
        fs::create_dir_all(source_file.parent().expect("source parent"))
            .expect("create source parent");
        fs::create_dir_all(&mutable_root).expect("create mutable worker root");
        fs::write(&source_file, "pub fn original() {}\n").expect("write source");
        let executable_script = workspace.join("scripts/release-targets.sh");
        fs::create_dir_all(executable_script.parent().expect("script parent"))
            .expect("create script parent");
        fs::write(&executable_script, "#!/usr/bin/env bash\n").expect("write script");
        let cargo_config = workspace.join(".cargo/config.toml");
        fs::create_dir_all(cargo_config.parent().expect("Cargo config parent"))
            .expect("create Cargo config parent");
        fs::write(&cargo_config, "[net]\noffline = true\n").expect("write Cargo config");
        let mut permissions = fs::metadata(&executable_script)
            .expect("script metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&executable_script, permissions).expect("make script executable");

        protect_worker_source_tree(&workspace).expect("protect source tree");
        fs::write(&source_file, "pub fn mutated() {}\n").expect("overwrite source file");
        assert!(fs::remove_file(&source_file).is_err());
        fs::write(mutable_root.join("artifact"), "artifact")
            .expect("mutable worker artifacts remain writable");
        assert!(fs::write(&cargo_config, "[net]\noffline = false\n").is_err());
        assert_ne!(
            fs::metadata(&executable_script)
                .expect("script metadata")
                .permissions()
                .mode()
                & 0o100,
            0
        );

        restore_worker_source_tree(&workspace).expect("restore source tree");
        fs::remove_file(&source_file).expect("source deletion is possible after restoration");
    }

    #[cfg(unix)]
    #[test]
    fn private_cargo_home_rejects_non_directory_sources_and_cache_members() {
        let root = tempdir().expect("temporary root");
        let source_file = root.path().join("source-file");
        fs::write(&source_file, "not a directory").expect("write source file");
        assert!(
            prepare_worker_cargo_home_from(&source_file, &root.path().join("destination")).is_err()
        );

        let source = root.path().join("source");
        fs::create_dir_all(&source).expect("create source home");
        fs::write(source.join("registry"), "not a directory").expect("write bad registry");
        assert!(
            prepare_worker_cargo_home_from(&source, &root.path().join("destination-two")).is_err()
        );
    }

    #[cfg(unix)]
    #[test]
    fn private_cargo_home_rejects_an_existing_destination_cache_path() {
        let root = tempdir().expect("temporary root");
        let source = root.path().join("source");
        let destination = root.path().join("destination");
        fs::create_dir_all(source.join("registry")).expect("create source cache");
        fs::create_dir_all(destination.join("registry")).expect("create conflicting cache");

        assert!(prepare_worker_cargo_home_from(&source, &destination).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn protection_helpers_fail_for_missing_paths() {
        let root = tempdir().expect("temporary root");
        let missing = root.path().join("missing");
        assert!(protect_worker_source_tree(&missing).is_err());
        assert!(restore_worker_source_tree(&missing).is_err());
        assert!(remove_unix_write_permission(&missing).is_err());
        assert!(add_unix_owner_write_permission(&missing).is_err());
    }

    #[test]
    fn cargo_home_resolution_prefers_explicit_then_home_then_userprofile() {
        assert_eq!(
            shared_cargo_home_from_sources(
                Some("/explicit/cargo".into()),
                Some("/home".into()),
                Some("/profile".into()),
            )
            .expect("explicit cargo home"),
            PathBuf::from("/explicit/cargo")
        );
        assert_eq!(
            shared_cargo_home_from_sources(None, Some("/home".into()), Some("/profile".into()))
                .expect("home cargo root"),
            PathBuf::from("/home/.cargo")
        );
        assert_eq!(
            shared_cargo_home_from_sources(None, None, Some("/profile".into()))
                .expect("profile cargo root"),
            PathBuf::from("/profile/.cargo")
        );
        assert!(shared_cargo_home_from_sources(None, None, None).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn source_protection_walks_regular_files_and_rejects_non_directory_cache_members() {
        let root = tempdir().expect("workspace root");
        let workspace = root.path().join("workspace");
        let source = workspace.join("nested/source.rs");
        fs::create_dir_all(source.parent().expect("source parent")).expect("create source parent");
        fs::write(&source, "pub fn source() {}\n").expect("write source");
        protect_source_directory(&workspace, true).expect("protect nested source");
        restore_source_directory(&workspace, true).expect("restore nested source");

        let cargo_home = root.path().join("cargo-home");
        fs::create_dir_all(&cargo_home).expect("create cargo home");
        fs::write(cargo_home.join("git"), "not a directory").expect("write bad cache member");
        assert!(
            prepare_worker_cargo_home_from(&cargo_home, &root.path().join("destination")).is_err()
        );
    }

    #[cfg(unix)]
    #[test]
    fn source_protection_rejects_symbolic_links_instead_of_silently_skipping_them() {
        let root = tempdir().expect("workspace root");
        let workspace = root.path().join("workspace");
        let target = workspace.join("target.rs");
        fs::create_dir_all(&workspace).expect("create workspace");
        fs::write(&target, "pub fn target() {}\n").expect("write source");
        std::os::unix::fs::symlink(&target, workspace.join("linked.rs"))
            .expect("create source link");

        assert!(protect_source_directory(&workspace, true).is_err());
        assert!(restore_source_directory(&workspace, true).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn private_cargo_home_reports_configuration_copy_failures() {
        let root = tempdir().expect("temporary root");
        let source = root.path().join("source");
        let destination = root.path().join("destination");
        fs::create_dir_all(&source).expect("create source home");
        fs::write(source.join("config.toml"), "[net]\noffline = true\n")
            .expect("write cargo config");
        fs::create_dir_all(destination.join("config.toml")).expect("block config file path");

        assert!(prepare_worker_cargo_home_from(&source, &destination).is_err());
    }
}
