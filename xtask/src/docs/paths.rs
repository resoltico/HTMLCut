use std::fs;
use std::path::{Path, PathBuf};

use crate::command_exec::repo_worktree_files;
use crate::model::DynResult;

const EXCLUDED_DIR_NAMES: &[&str] = &["semver-baseline", "target", "tmp"];

/// Lists the maintained Markdown documentation files that are part of the docs contract.
pub fn markdown_doc_paths(repo_root: &Path) -> DynResult<Vec<PathBuf>> {
    let mut paths = if let Some(paths) = repo_worktree_files(repo_root)? {
        paths
            .into_iter()
            .filter(|path| is_maintained_markdown_doc(repo_root, path))
            .collect()
    } else {
        let mut paths = Vec::new();
        collect_markdown_doc_paths(repo_root, repo_root, &mut paths)?;
        paths
    };
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
    should_skip_component(path.file_name().and_then(|name| name.to_str()))
}

fn should_skip_component(name: Option<&str>) -> bool {
    let Some(name) = name else {
        return true;
    };
    if name.starts_with('.') {
        return true;
    }
    EXCLUDED_DIR_NAMES.contains(&name)
}

fn is_maintained_markdown_doc(repo_root: &Path, path: &Path) -> bool {
    if !is_markdown_doc(path) {
        return false;
    }

    let Ok(relative) = path.strip_prefix(repo_root) else {
        return false;
    };

    relative
        .parent()
        .into_iter()
        .flat_map(Path::components)
        .all(|component| !should_skip_component(component.as_os_str().to_str()))
}

#[cfg(test)]
pub(super) fn should_skip_dir_for_tests(repo_root: &Path, path: &Path) -> bool {
    should_skip_dir(repo_root, path)
}

#[cfg(test)]
pub(super) fn is_maintained_markdown_doc_for_tests(repo_root: &Path, path: &Path) -> bool {
    is_maintained_markdown_doc(repo_root, path)
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
