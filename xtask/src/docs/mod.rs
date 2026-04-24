use std::fs;
use std::path::Path;

use regex::Regex;

use crate::model::DynResult;
use crate::workspace_version;

pub(crate) mod commands;
mod identifiers;
mod legal;
mod links;
mod metadata;
mod paths;
mod release;

pub use paths::markdown_doc_paths;

#[cfg(test)]
pub(crate) use metadata::{MetadataStyle, metadata_version};
/// Validates Markdown metadata and local links for the maintained docs set.
pub fn markdown_contract_errors(repo_root: &Path) -> DynResult<Vec<String>> {
    let workspace_version = workspace_version(repo_root)?;
    let expected_afad_version = metadata::expected_afad_version(repo_root)?;
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
            &expected_afad_version,
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
            repo_root,
            &display_path,
            &text,
            &schema_names,
            &operation_ids,
        ));
        errors.extend(release::release_doc_errors(repo_root, &display_path, &text));
        errors.extend(legal::legal_doc_errors(repo_root, &display_path, &text));
    }

    Ok(errors)
}
#[cfg(test)]
pub(crate) fn metadata_contract_errors_for_tests(
    display_path: &str,
    text: &str,
    style: MetadataStyle,
    updated_pattern: &Regex,
    expected_afad_version: &str,
) -> Vec<String> {
    metadata::metadata_contract_errors(
        display_path,
        text,
        style,
        updated_pattern,
        expected_afad_version,
    )
}

#[cfg(test)]
pub(crate) fn expected_metadata_style_for_tests(repo_root: &Path, path: &Path) -> MetadataStyle {
    metadata::expected_metadata_style(repo_root, path)
}

#[cfg(test)]
pub(crate) fn expected_afad_version_for_tests(repo_root: &Path) -> DynResult<String> {
    metadata::expected_afad_version(repo_root)
}

#[cfg(test)]
pub(crate) fn should_skip_dir_for_tests(repo_root: &Path, path: &Path) -> bool {
    paths::should_skip_dir_for_tests(repo_root, path)
}

#[cfg(test)]
pub(crate) fn is_maintained_markdown_doc_for_tests(repo_root: &Path, path: &Path) -> bool {
    paths::is_maintained_markdown_doc_for_tests(repo_root, path)
}

#[cfg(test)]
pub(crate) fn inventory_errors_for_tests(
    repo_root: &Path,
    display_path: &str,
    text: &str,
) -> Vec<String> {
    let schema_names = identifiers::known_schema_names();
    let operation_ids = identifiers::known_operation_ids();
    identifiers::inventory_errors(repo_root, display_path, text, &schema_names, &operation_ids)
}
