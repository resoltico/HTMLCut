use std::fs;
use std::path::Path;

use regex::Regex;

use crate::model::DynResult;
use crate::plan::workspace_version;

mod commands;
mod identifiers;
mod links;
mod metadata;
mod paths;

pub use paths::markdown_doc_paths;

#[cfg(test)]
pub(crate) use metadata::{MetadataStyle, metadata_version};
#[cfg(test)]
use std::collections::BTreeSet;

/// Validates Markdown metadata and local links for the maintained docs set.
pub fn markdown_contract_errors(repo_root: &Path) -> DynResult<Vec<String>> {
    let workspace_version = workspace_version(repo_root)?;
    let link_pattern = Regex::new(r"\[[^\]]+\]\(([^)]+)\)")?;
    let schema_name_pattern = Regex::new(r"\bhtmlcut\.[a-z_]+\b")?;
    let operation_id_pattern =
        Regex::new(r"\b(?:document|source|select|slice)\.(?:parse|inspect|preview|extract)\b")?;
    let updated_pattern = Regex::new(r"^\d{4}-\d{2}-\d{2}$")?;
    let schema_names = identifiers::known_schema_names();
    let operation_ids = identifiers::known_operation_ids();
    let mut errors = Vec::new();

    for path in markdown_doc_paths(repo_root)? {
        let display_path = paths::repo_relative_display(repo_root, &path);
        let text = fs::read_to_string(&path)?;
        let metadata_style = metadata::expected_metadata_style(repo_root, &path);
        let version = metadata::metadata_version(&text, metadata_style);

        match version {
            Some(version) if version == workspace_version => {}
            Some(version) => errors.push(format!(
                "{display_path} metadata version is {version}, expected {workspace_version}"
            )),
            None => errors.push(format!(
                "{display_path} is missing the expected {} metadata version entry",
                metadata_style.label()
            )),
        }

        errors.extend(metadata::metadata_contract_errors(
            &display_path,
            &text,
            metadata_style,
            &updated_pattern,
        ));
        errors.extend(links::local_link_errors(
            repo_root,
            &path,
            &text,
            &link_pattern,
        ));
        errors.extend(identifiers::identifier_errors(
            &display_path,
            &text,
            &schema_name_pattern,
            &schema_names,
            "schema name",
        ));
        errors.extend(identifiers::identifier_errors(
            &display_path,
            &text,
            &operation_id_pattern,
            &operation_ids,
            "operation ID",
        ));
        errors.extend(commands::command_example_errors(
            &display_path,
            &text,
            &schema_names,
            &operation_ids,
        ));
        errors.extend(identifiers::inventory_errors(
            &display_path,
            &text,
            &schema_names,
            &operation_ids,
        ));
    }

    Ok(errors)
}

#[cfg(test)]
pub(crate) fn extract_htmlcut_examples_for_tests(text: &str) -> Vec<String> {
    commands::extract_htmlcut_examples(text)
}

#[cfg(test)]
pub(crate) fn shell_words_for_tests(command: &str) -> Vec<String> {
    commands::shell_words(command).expect("shell words")
}

#[cfg(test)]
pub(crate) fn command_path_for_tests(tokens: &[String]) -> Vec<String> {
    commands::command_path(tokens)
        .into_iter()
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
pub(crate) fn command_reference_error_for_tests(
    display_path: &str,
    tokens: &[String],
    schema_names: &BTreeSet<&'static str>,
    operation_ids: &BTreeSet<&'static str>,
) -> Option<String> {
    commands::command_reference_error(display_path, tokens, schema_names, operation_ids)
}

#[cfg(test)]
pub(crate) fn command_example_errors_for_tests(
    display_path: &str,
    text: &str,
    schema_names: &BTreeSet<&'static str>,
    operation_ids: &BTreeSet<&'static str>,
) -> Vec<String> {
    commands::command_example_errors(display_path, text, schema_names, operation_ids)
}

#[cfg(test)]
pub(crate) fn metadata_contract_errors_for_tests(
    display_path: &str,
    text: &str,
    style: MetadataStyle,
    updated_pattern: &Regex,
) -> Vec<String> {
    metadata::metadata_contract_errors(display_path, text, style, updated_pattern)
}

#[cfg(test)]
pub(crate) fn expected_metadata_style_for_tests(repo_root: &Path, path: &Path) -> MetadataStyle {
    metadata::expected_metadata_style(repo_root, path)
}

#[cfg(test)]
pub(crate) fn should_skip_dir_for_tests(repo_root: &Path, path: &Path) -> bool {
    paths::should_skip_dir_for_tests(repo_root, path)
}
