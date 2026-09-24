//! Small internal temporary-directory helper shared across the HTMLCut workspace.
#![forbid(unsafe_code)]
#![deny(missing_docs)]

use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_TEMP_DIR_ID: AtomicU64 = AtomicU64::new(0);
const MAX_CREATE_ATTEMPTS: usize = 128;

/// Creates one unique temporary directory under the process temp root.
pub fn tempdir() -> io::Result<TempDir> {
    TempDir::new()
}

/// Creates one unique temporary directory beneath `base`.
///
/// The directory is removed recursively when the returned [`TempDir`] drops. The base directory
/// is created when it does not yet exist and is otherwise retained.
pub fn tempdir_in(base: impl AsRef<Path>) -> io::Result<TempDir> {
    TempDir::new_in(base)
}

/// Temporary directory that is deleted recursively when it drops.
#[derive(Debug)]
pub struct TempDir {
    path: PathBuf,
}

impl TempDir {
    /// Creates one unique temporary directory under the system temp root.
    pub fn new() -> io::Result<Self> {
        Self::new_in(env::temp_dir())
    }

    /// Creates one unique temporary directory beneath `base`.
    pub fn new_in(base: impl AsRef<Path>) -> io::Result<Self> {
        let base = base.as_ref();
        fs::create_dir_all(base)?;
        let pid = process::id();

        for _ in 0..MAX_CREATE_ATTEMPTS {
            let counter = NEXT_TEMP_DIR_ID.fetch_add(1, Ordering::Relaxed);
            let timestamp_nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let candidate = base.join(format!("htmlcut-{pid}-{timestamp_nanos}-{counter}"));

            match fs::create_dir(&candidate) {
                Ok(()) => return Ok(Self { path: candidate }),
                Err(error) => retry_after_directory_name_collision(error)?,
            }
        }

        Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "failed to create a unique temporary directory",
        ))
    }

    /// Returns the filesystem path of this temporary directory.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

fn retry_after_directory_name_collision(error: io::Error) -> io::Result<()> {
    (error.kind() == io::ErrorKind::AlreadyExists)
        .then_some(())
        .ok_or(error)
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tempdir_creates_distinct_directories() {
        let first = tempdir().expect("first tempdir");
        let second = tempdir().expect("second tempdir");

        assert_ne!(first.path(), second.path());
        assert!(first.path().is_dir());
        assert!(second.path().is_dir());
    }

    #[test]
    fn tempdir_cleans_up_on_drop() {
        let path = {
            let dir = tempdir().expect("tempdir");
            let file = dir.path().join("fixture.txt");
            fs::write(&file, "fixture").expect("write fixture");
            assert!(file.is_file());
            dir.path().to_path_buf()
        };

        assert!(!path.exists());
    }

    #[test]
    fn tempdir_in_creates_and_cleans_a_child_without_removing_its_parent() {
        let outer = tempdir().expect("outer tempdir");
        let base = outer.path().join("nested").join("scratch");
        let path = {
            let dir = tempdir_in(&base).expect("nested tempdir");
            assert!(dir.path().starts_with(&base));
            dir.path().to_path_buf()
        };

        assert!(!path.exists());
        assert!(base.is_dir());
    }

    #[test]
    fn only_directory_name_collisions_are_retried() {
        assert!(
            retry_after_directory_name_collision(io::Error::from(io::ErrorKind::AlreadyExists))
                .is_ok()
        );
        assert_eq!(
            retry_after_directory_name_collision(io::Error::from(io::ErrorKind::PermissionDenied))
                .expect_err("permission error must not be retried")
                .kind(),
            io::ErrorKind::PermissionDenied
        );
    }
}
