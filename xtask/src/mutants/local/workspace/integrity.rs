//! Source-restoration proof for reusable local mutation lanes.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::DynResult;

/// Content identity of one copied source workspace, excluding only disposable worker output.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct SourceTreeFingerprint(BTreeMap<PathBuf, [u8; 32]>);

impl SourceTreeFingerprint {
    pub(super) fn from_workspace(workspace_root: &Path) -> DynResult<Self> {
        let mut entries = BTreeMap::new();
        fingerprint_workspace_directory(workspace_root, workspace_root, &mut entries)?;
        Ok(Self(entries))
    }

    pub(super) fn verify(&self, workspace_root: &Path) -> DynResult<()> {
        let current = Self::from_workspace(workspace_root)?;
        if current != *self {
            let changed_paths = changed_paths(&self.0, &current.0);
            return Err(format!(
                "cargo-mutants did not restore disposable source workspace {} before its next partition; changed paths: {}",
                workspace_root.display(),
                changed_paths
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", "),
            )
            .into());
        }
        Ok(())
    }
}

fn fingerprint_workspace_directory(
    workspace_root: &Path,
    directory: &Path,
    entries: &mut BTreeMap<PathBuf, [u8; 32]>,
) -> DynResult<()> {
    let mut directory_entries = fs::read_dir(directory)?.collect::<Result<Vec<_>, _>>()?;
    directory_entries.sort_by_key(|entry| entry.file_name());
    for entry in directory_entries {
        let path = entry.path();
        let relative = path.strip_prefix(workspace_root).map_err(|error| {
            format!(
                "mutation workspace source escaped root {}: {error}",
                path.display()
            )
        })?;
        if super::materialization::mutation_workspace_path_is_excluded(relative) {
            continue;
        }
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            entries.insert(
                relative.to_path_buf(),
                Sha256::digest(b"directory\0").into(),
            );
            fingerprint_workspace_directory(workspace_root, &path, entries)?;
        } else if file_type.is_file() {
            let contents = fs::read(&path)?;
            let mut hasher = Sha256::new();
            hasher.update(b"file\0");
            hasher.update((contents.len() as u64).to_le_bytes());
            hasher.update(contents);
            entries.insert(relative.to_path_buf(), hasher.finalize().into());
        } else {
            return Err(format!(
                "mutation workspace source is not a regular file or directory: {}",
                path.display()
            )
            .into());
        }
    }
    Ok(())
}

fn changed_paths(
    expected: &BTreeMap<PathBuf, [u8; 32]>,
    actual: &BTreeMap<PathBuf, [u8; 32]>,
) -> Vec<PathBuf> {
    expected
        .keys()
        .chain(actual.keys())
        .filter(|path| expected.get(*path) != actual.get(*path))
        .cloned()
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .take(8)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use htmlcut_tempdir::tempdir;

    #[test]
    fn fingerprint_rejects_a_directory_outside_the_workspace_root() {
        let workspace = tempdir().expect("workspace root");
        let outside = tempdir().expect("outside root");
        fs::write(outside.path().join("outside.rs"), "pub fn outside() {}\n")
            .expect("write outside source");
        let mut entries = BTreeMap::new();
        let error = fingerprint_workspace_directory(workspace.path(), outside.path(), &mut entries)
            .expect_err("outside source must not be fingerprinted");
        assert!(error.to_string().contains("escaped root"));
    }

    #[cfg(unix)]
    #[test]
    fn fingerprint_rejects_symbolic_link_sources() {
        let workspace = tempdir().expect("workspace root");
        let target = workspace.path().join("target.rs");
        fs::write(&target, "pub fn target() {}\n").expect("write target");
        std::os::unix::fs::symlink(&target, workspace.path().join("linked.rs"))
            .expect("create source link");

        let error = SourceTreeFingerprint::from_workspace(workspace.path())
            .expect_err("symbolic link source must be rejected");
        assert!(
            error
                .to_string()
                .contains("not a regular file or directory")
        );
    }

    #[test]
    fn changed_path_diagnostics_are_deduplicated_sorted_and_bounded() {
        let expected = (0..10)
            .map(|index| (PathBuf::from(format!("path-{index}")), [0; 32]))
            .collect::<BTreeMap<_, _>>();
        let actual = (0..10)
            .map(|index| (PathBuf::from(format!("path-{index}")), [1; 32]))
            .collect::<BTreeMap<_, _>>();

        assert_eq!(changed_paths(&expected, &actual).len(), 8);
    }

    #[test]
    fn fingerprint_detects_a_same_length_source_content_change() {
        let workspace = tempdir().expect("workspace root");
        let source = workspace.path().join("src/lib.rs");
        fs::create_dir_all(source.parent().expect("source parent")).expect("create source parent");
        fs::write(&source, "pub fn one() {}\n").expect("write original source");
        let fingerprint = SourceTreeFingerprint::from_workspace(workspace.path())
            .expect("fingerprint original source");

        fs::write(&source, "pub fn two() {}\n").expect("write changed source");

        assert!(fingerprint.verify(workspace.path()).is_err());
    }
}
