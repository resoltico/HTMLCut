//! Exact copied-source materialization for disposable local mutation lanes.

use std::fs;
use std::path::Path;

use crate::DynResult;

const MUTATION_COPY_EXCLUDED_ROOTS: &[&str] = &[
    ".git",
    ".htmlcut-mutant-cargo",
    "mutants.out",
    "target",
    "tmp",
];

/// Materializes one source workspace without build output, temporary probes, or VCS metadata.
pub(super) fn materialize_mutation_workspace(
    repo_root: &Path,
    workspace_root: &Path,
) -> DynResult<()> {
    // A mutation workspace must reproduce the actual checkout, not only the Git index. Tooling
    // and documentation contracts can intentionally depend on ignored local instruction files;
    // the explicit generated-root exclusion policy below is the safe boundary for copy omission.
    copy_mutation_workspace_directory(repo_root, repo_root, workspace_root)
}

fn copy_mutation_workspace_directory(
    repo_root: &Path,
    source_dir: &Path,
    workspace_root: &Path,
) -> DynResult<()> {
    for entry in fs::read_dir(source_dir)? {
        let entry = entry?;
        let source_path = entry.path();
        let relative_path = source_path.strip_prefix(repo_root).map_err(|error| {
            format!(
                "mutation workspace source escaped repository root {}: {error}",
                source_path.display()
            )
        })?;
        if mutation_workspace_path_is_excluded(relative_path) {
            continue;
        }

        if entry.file_type()?.is_dir() {
            copy_mutation_workspace_directory(repo_root, &source_path, workspace_root)?;
        } else {
            copy_mutation_workspace_file(repo_root, &source_path, workspace_root)?;
        }
    }
    Ok(())
}

fn copy_mutation_workspace_file(
    repo_root: &Path,
    source_path: &Path,
    workspace_root: &Path,
) -> DynResult<()> {
    let relative_path = source_path.strip_prefix(repo_root).map_err(|error| {
        format!(
            "mutation workspace source escaped repository root {}: {error}",
            source_path.display()
        )
    })?;
    if mutation_workspace_path_is_excluded(relative_path) {
        return Ok(());
    }

    if !fs::symlink_metadata(source_path)?.file_type().is_file() {
        return Err(format!(
            "mutation workspace source is not a regular file: {}",
            source_path.display()
        )
        .into());
    }

    let destination_path = workspace_root.join(relative_path);
    let destination_parent = destination_path
        .parent()
        .expect("a copied repository child always has a destination parent");
    fs::create_dir_all(destination_parent)?;
    fs::copy(source_path, destination_path)?;
    Ok(())
}

pub(super) fn mutation_workspace_path_is_excluded(relative_path: &Path) -> bool {
    relative_path
        .components()
        .next()
        .and_then(|component| component.as_os_str().to_str())
        .is_some_and(|root| MUTATION_COPY_EXCLUDED_ROOTS.contains(&root))
}

#[cfg(test)]
mod tests {
    use super::*;
    use htmlcut_tempdir::tempdir;

    #[test]
    fn materialization_rejects_a_source_directory_outside_its_declared_root() {
        let repository = tempdir().expect("repository root");
        let outside = tempdir().expect("outside root");
        let destination = tempdir().expect("destination root");
        fs::write(outside.path().join("outside.rs"), "pub fn outside() {}\n")
            .expect("write outside source");
        let error = copy_mutation_workspace_directory(
            repository.path(),
            outside.path(),
            destination.path(),
        )
        .expect_err("outside source directory must be rejected");
        assert!(error.to_string().contains("escaped repository root"));
    }

    #[cfg(unix)]
    #[test]
    fn materialization_rejects_symbolic_link_source_files() {
        let repository = tempdir().expect("repository root");
        let destination = tempdir().expect("destination root");
        let source = repository.path().join("source.rs");
        fs::write(&source, "pub fn source() {}\n").expect("write source");
        std::os::unix::fs::symlink(&source, repository.path().join("linked.rs"))
            .expect("create source link");

        let error = materialize_mutation_workspace(repository.path(), destination.path())
            .expect_err("symbolic source must be rejected");
        assert!(error.to_string().contains("not a regular file"));
    }

    #[test]
    fn exclusion_policy_recognizes_only_generated_root_names() {
        assert!(mutation_workspace_path_is_excluded(Path::new(
            "target/debug"
        )));
        assert!(mutation_workspace_path_is_excluded(Path::new(
            ".git/config"
        )));
        assert!(!mutation_workspace_path_is_excluded(Path::new(
            "src/target.rs"
        )));
    }

    #[test]
    fn direct_file_copy_rejects_foreign_sources_and_skips_excluded_generated_roots() {
        let repository = tempdir().expect("repository root");
        let outside = tempdir().expect("outside root");
        let destination = tempdir().expect("destination root");
        let outside_source = outside.path().join("outside.rs");
        fs::write(&outside_source, "pub fn outside() {}\n").expect("write outside source");
        assert!(
            copy_mutation_workspace_file(repository.path(), &outside_source, destination.path(),)
                .is_err()
        );

        let excluded_source = repository.path().join("target/generated.rs");
        fs::create_dir_all(excluded_source.parent().expect("generated parent"))
            .expect("create generated parent");
        fs::write(&excluded_source, "generated").expect("write generated source");
        copy_mutation_workspace_file(repository.path(), &excluded_source, destination.path())
            .expect("exclude generated root");
        assert!(!destination.path().join("target/generated.rs").exists());
    }
}
