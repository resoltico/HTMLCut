// SPDX-License-Identifier: MPL-2.0
//! Rejects artifact configurations that could mark, scan or delete the source checkout.

use std::io;
use std::path::{Component, Path, PathBuf};

use super::{
    DynResult, fs, managed_artifact_container_dir, managed_artifact_roots, scratch_artifact_roots,
};

pub(super) fn ensure_source_boundary(repo_root: &Path) -> DynResult<()> {
    let source = fs::canonicalize(repo_root)?;
    let mut paths = managed_artifact_roots(repo_root);
    paths.extend(scratch_artifact_roots(repo_root));
    paths.extend(managed_artifact_container_dir(repo_root));
    for path in paths {
        let resolved = resolve_artifact_path(&path).map_err(|error| {
            format!(
                "failed to resolve hygiene artifact path {}: {error}",
                path.display()
            )
        })?;
        if source.starts_with(&resolved) {
            return Err(format!(
                "hygiene artifact path {} contains the source checkout; configure dedicated CARGO_TARGET_DIR and CARGO_BUILD_BUILD_DIR directories outside its ancestors",
                path.display()
            ).into());
        }
    }
    Ok(())
}

// Resolve again after each component: a missing directory followed by `..` can return to an
// existing symlink. A purely lexical suffix after the first missing directory would miss it.
fn resolve_artifact_path(path: &Path) -> io::Result<PathBuf> {
    let absolute = std::path::absolute(path)?;
    let root = absolute
        .ancestors()
        .last()
        .expect("an absolute path has a root");
    let mut resolved = fs::canonicalize(root)?;
    for component in absolute
        .strip_prefix(root)
        .expect("the root is a prefix")
        .components()
    {
        match component {
            Component::ParentDir => {
                resolved.pop();
            }
            _ => resolved.push(component.as_os_str()),
        }
        match fs::canonicalize(&resolved) {
            Ok(canonical) => resolved = canonical,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hygiene::{
        HygieneCleanMode, clean_hygiene, hygiene_report, prepare_artifact_layout,
        prepare_gate_report_root, prepare_mutation_report_root,
    };
    use crate::model::CommandArtifactLayout;
    use htmlcut_tempdir::tempdir;

    fn reject_before_actions(repo: &Path, target: PathBuf, build: PathBuf) {
        crate::plan::with_cargo_artifact_dir_overrides_for_tests(target, build, || {
            for command in ["mutants", "check"] {
                let error = crate::main_entry_with(repo, ["xtask", command])
                    .expect_err("gate report validates before mutation/semver cleanup");
                assert!(error.to_string().contains("contains the source checkout"));
            }
            assert!(hygiene_report(repo).is_err());
            assert!(prepare_gate_report_root(repo).is_err());
            assert!(prepare_mutation_report_root(repo).is_err());
            for layout in [
                CommandArtifactLayout::ManagedWorkspace,
                CommandArtifactLayout::ManagedCoverage,
            ] {
                assert!(prepare_artifact_layout(repo, layout).is_err());
            }
            for mode in [HygieneCleanMode::Safe, HygieneCleanMode::Rebuildable] {
                let error =
                    clean_hygiene(repo, mode).expect_err("source-containing artifacts fail");
                assert!(error.to_string().contains("contains the source checkout"));
                assert!(error.to_string().contains("CARGO_TARGET_DIR"));
            }
        });
    }

    #[test]
    fn source_and_siblings_survive_root_and_ancestor_artifact_configuration() {
        // The whole potentially scanned/deleted parent is a private fixture, never system TMP.
        let fixture = tempdir().expect("fixture");
        let repo = fixture.path().join("source");
        fs::create_dir(&repo).expect("source");
        fs::write(repo.join("source.rs"), "source sentinel").expect("source sentinel");
        fs::write(fixture.path().join("sibling.txt"), "sibling sentinel")
            .expect("sibling sentinel");
        for (target, build) in [
            (repo.clone(), repo.clone()),
            (repo.join("."), repo.join(".")),
            (repo.join("../target"), repo.join("../build")),
        ] {
            reject_before_actions(&repo, target, build);
            assert_eq!(
                fs::read(repo.join("source.rs")).expect("source retained"),
                b"source sentinel"
            );
            assert_eq!(
                fs::read(fixture.path().join("sibling.txt")).expect("sibling retained"),
                b"sibling sentinel"
            );
            assert!(!fixture.path().join("CACHEDIR.TAG").exists());
            assert!(!repo.join("CACHEDIR.TAG").exists());
            assert!(!repo.join("tmp").exists());
            assert!(!fixture.path().join("target").exists());
        }
    }

    #[cfg(unix)]
    #[test]
    fn symlink_and_missing_parent_components_cannot_hide_source_ancestors() {
        use std::os::unix::fs::symlink;
        let fixture = tempdir().expect("fixture");
        let repo = fixture.path().join("source");
        fs::create_dir(&repo).expect("source");
        fs::write(repo.join("source.rs"), "source sentinel").expect("source sentinel");
        symlink(fixture.path(), repo.join("ancestor")).expect("ancestor alias");
        for parent in [repo.join("ancestor"), repo.join("missing/../ancestor")] {
            reject_before_actions(&repo, parent.join("target"), parent.join("build"));
            assert!(!fixture.path().join("target").exists());
            assert!(!fixture.path().join("CACHEDIR.TAG").exists());
            assert!(!repo.join("missing").exists());
            assert_eq!(
                fs::read(repo.join("source.rs")).expect("source retained"),
                b"source sentinel"
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn nested_coverage_and_scratch_aliases_are_checked_before_cleanup() {
        use std::os::unix::fs::symlink;
        let fixture = tempdir().expect("fixture");
        let repo = fixture.path().join("source");
        let container = fixture.path().join("artifacts");
        fs::create_dir(&repo).expect("source");
        fs::create_dir(&container).expect("artifacts");
        fs::write(repo.join("source.rs"), "source sentinel").expect("source sentinel");
        for alias in [
            container.join("coverage-target/llvm-cov-target"),
            container.join("coverage-build/llvm-cov-target"),
            container.join("target/semver-checks"),
            repo.join("tmp"),
            repo.join("target"),
        ] {
            fs::create_dir_all(alias.parent().expect("alias parent")).expect("parent");
            symlink(&repo, &alias).expect("source alias");
            reject_before_actions(&repo, container.join("target"), container.join("build"));
            assert_eq!(
                fs::read(repo.join("source.rs")).expect("source retained"),
                b"source sentinel"
            );
            assert!(!container.join("CACHEDIR.TAG").exists());
            fs::remove_file(alias).expect("remove fixture alias");
        }
    }

    #[test]
    fn safe_missing_artifact_descendants_resolve_and_prepare() {
        let fixture = tempdir().expect("fixture");
        let repo = fixture.path().join("source");
        fs::create_dir(&repo).expect("source");
        for container in [repo.join("artifacts"), repo.join("../artifacts")] {
            crate::plan::with_cargo_artifact_dir_overrides_for_tests(
                container.join("target"),
                container.join("build"),
                || {
                    prepare_artifact_layout(&repo, CommandArtifactLayout::ManagedWorkspace)
                        .expect("safe bootstrap");
                    hygiene_report(&repo).expect("safe report");
                    assert!(container.join("target/CACHEDIR.TAG").is_file());
                },
            );
        }
        assert_eq!(
            resolve_artifact_path(&repo.join("missing/../../..")).expect("parent folding"),
            fs::canonicalize(fixture.path().parent().expect("fixture parent"))
                .expect("physical parent")
        );
    }

    #[test]
    fn real_resolution_io_failure_is_preserved() {
        let fixture = tempdir().expect("fixture");
        let path = fixture.path().join("invalid\0path");
        assert_eq!(
            resolve_artifact_path(&path)
                .expect_err("invalid native path")
                .kind(),
            io::ErrorKind::InvalidInput
        );
        crate::plan::with_cargo_artifact_dir_overrides_for_tests(path.clone(), path, || {
            let message = ensure_source_boundary(fixture.path())
                .expect_err("real IO failure")
                .to_string();
            assert!(message.contains("failed to resolve hygiene artifact path"));
        });
        assert!(ensure_source_boundary(&fixture.path().join("missing-source")).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn non_directory_prefix_is_a_real_resolution_failure() {
        let fixture = tempdir().expect("fixture");
        fs::write(fixture.path().join("file"), "regular file").expect("file");
        assert_eq!(
            resolve_artifact_path(&fixture.path().join("file/child"))
                .expect_err("not a directory")
                .kind(),
            io::ErrorKind::NotADirectory
        );
    }
}
