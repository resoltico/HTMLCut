use super::*;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

fn comparable_path_identity(path: &Path) -> (PathBuf, Vec<OsString>) {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir().expect("current dir").join(path)
    };

    let mut existing_prefix = absolute.as_path();
    let mut tail = Vec::new();
    while !existing_prefix.exists() {
        let component = existing_prefix
            .file_name()
            .expect("path without existing prefix should keep a file name")
            .to_owned();
        tail.push(component);
        existing_prefix = existing_prefix
            .parent()
            .expect("absolute path should retain an existing ancestor");
    }
    tail.reverse();

    let canonical_prefix = existing_prefix
        .canonicalize()
        .expect("canonical existing prefix");
    (canonical_prefix, tail)
}

fn assert_same_path_identity(actual: &Path, expected: &Path) {
    assert_eq!(
        comparable_path_identity(actual),
        comparable_path_identity(expected)
    );
}

mod errors_and_outcomes;
mod human_rendering;
mod output_and_paths;
