use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

use regex::Regex;

pub(super) fn identifier_errors(
    display_path: &str,
    text: &str,
    pattern: &Regex,
    allowed: &BTreeSet<&'static str>,
    label: &str,
) -> Vec<String> {
    let unknown_identifiers = pattern
        .find_iter(text)
        .map(|matched| matched.as_str())
        .filter(|identifier| !allowed.contains(identifier))
        .collect::<BTreeSet<_>>();

    unknown_identifiers
        .into_iter()
        .map(|identifier| format!("{display_path} references unknown {label}: {identifier}"))
        .collect()
}

pub(super) fn operation_identifier_errors(
    display_path: &str,
    text: &str,
    operation_ids: &BTreeSet<&'static str>,
) -> Vec<String> {
    let pattern =
        Regex::new(r"\b[a-z][a-z0-9]*(?:\.[a-z][a-z0-9]*)+\b").expect("valid operation regex");
    let known_prefixes = operation_ids
        .iter()
        .filter_map(|operation_id| operation_id.split_once('.').map(|(prefix, _)| prefix))
        .collect::<BTreeSet<_>>();
    let known_suffixes = operation_ids
        .iter()
        .filter_map(|operation_id| operation_id.rsplit_once('.').map(|(_, suffix)| suffix))
        .collect::<BTreeSet<_>>();

    let unknown_identifiers = pattern
        .find_iter(text)
        .map(|matched| matched.as_str())
        .filter(|identifier| !operation_ids.contains(identifier))
        .filter(|identifier| {
            identifier
                .split_once('.')
                .is_some_and(|(prefix, _)| known_prefixes.contains(prefix))
                || identifier
                    .rsplit_once('.')
                    .is_some_and(|(_, suffix)| known_suffixes.contains(suffix))
        })
        .collect::<BTreeSet<_>>();

    unknown_identifiers
        .into_iter()
        .map(|identifier| format!("{display_path} references unknown operation ID: {identifier}"))
        .collect()
}

pub(super) fn inventory_errors(
    repo_root: &Path,
    display_path: &str,
    text: &str,
    schema_names: &BTreeSet<&'static str>,
    operation_ids: &BTreeSet<&'static str>,
) -> Vec<String> {
    let mut errors = Vec::new();

    if display_path == "docs/schema.md" {
        let documented = documented_schema_names(text);
        let missing = schema_names
            .iter()
            .filter(|schema_name| !documented.contains(**schema_name))
            .copied()
            .collect::<Vec<_>>();
        if !missing.is_empty() {
            errors.push(format!(
                "docs/schema.md is missing schema names from the registry: {}",
                missing.join(", ")
            ));
        }
    }

    if display_path == "docs/operations.md" {
        let documented = documented_operation_ids(text, operation_ids);
        let missing = operation_ids
            .iter()
            .filter(|operation_id| !documented.contains(**operation_id))
            .copied()
            .collect::<Vec<_>>();
        if !missing.is_empty() {
            errors.push(format!(
                "docs/operations.md is missing operation IDs from the catalog: {}",
                missing.join(", ")
            ));
        }
    }

    if display_path == "docs/workspace-layout.md" {
        match crate::manifest::workspace_members(repo_root) {
            Ok(workspace_members) => {
                let documented = documented_workspace_members(text);
                let expected = workspace_members.into_iter().collect::<BTreeSet<_>>();
                let missing = expected
                    .difference(&documented)
                    .cloned()
                    .collect::<Vec<_>>();
                if !missing.is_empty() {
                    errors.push(format!(
                        "docs/workspace-layout.md is missing workspace members from Cargo.toml: {}",
                        missing.join(", ")
                    ));
                }

                let extra = documented
                    .difference(&expected)
                    .cloned()
                    .collect::<Vec<_>>();
                if !extra.is_empty() {
                    errors.push(format!(
                        "docs/workspace-layout.md documents workspace members not present in Cargo.toml: {}",
                        extra.join(", ")
                    ));
                }
            }
            Err(error) => errors.push(format!(
                "docs/workspace-layout.md could not load workspace members from Cargo.toml: {error}"
            )),
        }
    }

    if display_path == "docs/README.md" {
        match documented_docs_index_entries(repo_root) {
            Ok(expected) => {
                let documented = documented_docs_index_paths(text);
                let missing = expected
                    .difference(&documented)
                    .cloned()
                    .collect::<Vec<_>>();
                if !missing.is_empty() {
                    errors.push(format!(
                        "docs/README.md is missing Markdown docs from docs/: {}",
                        missing.join(", ")
                    ));
                }
            }
            Err(error) => errors.push(format!(
                "docs/README.md could not load Markdown docs from docs/: {error}"
            )),
        }
    }

    errors
}

fn documented_schema_names(text: &str) -> BTreeSet<String> {
    let pattern = Regex::new(r"\bhtmlcut\.[a-z_]+\b").expect("valid schema regex");
    pattern
        .find_iter(text)
        .map(|matched| matched.as_str().to_owned())
        .collect()
}

fn documented_operation_ids(
    text: &str,
    operation_ids: &BTreeSet<&'static str>,
) -> BTreeSet<String> {
    operation_ids
        .iter()
        .copied()
        .filter(|operation_id| text.contains(operation_id))
        .map(ToOwned::to_owned)
        .collect()
}

fn documented_workspace_members(text: &str) -> BTreeSet<String> {
    let pattern =
        Regex::new(r"`((?:crates/[a-z0-9-]+)|fuzz|xtask)`").expect("valid workspace member regex");
    pattern
        .captures_iter(text)
        .filter_map(|captures| captures.get(1).map(|matched| matched.as_str().to_owned()))
        .collect()
}

fn documented_docs_index_paths(text: &str) -> BTreeSet<String> {
    let pattern =
        Regex::new(r"\[[^\]]+\]\(([^)#]+\.md)\)").expect("valid docs index markdown link regex");
    pattern
        .captures_iter(text)
        .filter_map(|captures| captures.get(1).map(|matched| matched.as_str()))
        .filter_map(normalize_docs_index_target)
        .collect()
}

fn normalize_docs_index_target(target: &str) -> Option<String> {
    let mut normalized = PathBuf::new();
    for component in Path::new(target).components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => normalized.push(part),
            Component::ParentDir | Component::Prefix(_) | Component::RootDir => return None,
        }
    }

    (!normalized.as_os_str().is_empty()).then(|| normalized.to_string_lossy().replace('\\', "/"))
}

fn documented_docs_index_entries(repo_root: &Path) -> crate::model::DynResult<BTreeSet<String>> {
    Ok(super::markdown_doc_paths(repo_root)?
        .into_iter()
        .filter_map(|path| path.strip_prefix(repo_root).ok().map(Path::to_path_buf))
        .filter_map(|relative| {
            let display = relative.to_string_lossy().replace('\\', "/");
            display
                .strip_prefix("docs/")
                .filter(|path| *path != "README.md")
                .map(ToOwned::to_owned)
        })
        .collect())
}

pub(super) fn known_schema_names() -> BTreeSet<&'static str> {
    let mut names = htmlcut_core::schema_catalog()
        .iter()
        .map(|descriptor| descriptor.schema_ref.schema_name)
        .collect::<BTreeSet<_>>();
    names.insert(htmlcut_cli::CATALOG_REPORT_SCHEMA_NAME);
    names.insert(htmlcut_cli::SCHEMA_COMMAND_REPORT_SCHEMA_NAME);
    names.insert(htmlcut_cli::EXTRACTION_COMMAND_REPORT_SCHEMA_NAME);
    names.insert(htmlcut_cli::SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_NAME);
    names.insert(htmlcut_cli::ERROR_COMMAND_REPORT_SCHEMA_NAME);
    names.insert(crate::gate_report::GATE_RUN_REPORT_SCHEMA_NAME);
    names
}

pub(super) fn known_operation_ids() -> BTreeSet<&'static str> {
    htmlcut_core::operation_catalog()
        .iter()
        .map(|descriptor| descriptor.id.as_str())
        .collect()
}
