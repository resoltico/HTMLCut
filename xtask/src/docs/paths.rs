use std::fs;
use std::path::{Path, PathBuf};

use crate::model::DynResult;

const EXCLUDED_DIR_NAMES: &[&str] = &["semver-baseline", "target", "tmp"];

/// Lists the maintained Markdown documentation files that are part of the docs contract.
pub fn markdown_doc_paths(repo_root: &Path) -> DynResult<Vec<PathBuf>> {
    let mut paths = Vec::new();
    collect_markdown_doc_paths(repo_root, repo_root, &mut paths)?;
    paths.sort();
    Ok(paths)
}

fn collect_markdown_doc_paths(
    repo_root: &Path,
    current_dir: &Path,
    paths: &mut Vec<PathBuf>,
) -> DynResult<()> {
    for entry in fs::read_dir(current_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if should_skip_dir(repo_root, &path) {
                continue;
            }
            collect_markdown_doc_paths(repo_root, &path, paths)?;
            continue;
        }

        if is_markdown_doc(&path) {
            paths.push(path);
        }
    }

    Ok(())
}

fn should_skip_dir(_repo_root: &Path, path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return true;
    };
    if name.starts_with('.') {
        return true;
    }
    EXCLUDED_DIR_NAMES.contains(&name)
}

#[cfg(test)]
pub(super) fn should_skip_dir_for_tests(repo_root: &Path, path: &Path) -> bool {
    should_skip_dir(repo_root, path)
}

fn is_markdown_doc(path: &Path) -> bool {
    if path.extension().is_none_or(|extension| extension != "md") {
        return false;
    }

    !path
        .file_name()
        .is_some_and(|name| name == "CHANGELOG.md" || name == "changelog.md")
}

pub(super) fn repo_relative_display(repo_root: &Path, path: &Path) -> String {
    path.strip_prefix(repo_root)
        .expect("path should stay inside repo root")
        .to_string_lossy()
        .into_owned()
}
