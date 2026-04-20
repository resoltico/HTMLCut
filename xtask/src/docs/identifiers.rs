use std::collections::BTreeSet;

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

pub(super) fn inventory_errors(
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
        let documented = documented_operation_ids(text);
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

    errors
}

fn documented_schema_names(text: &str) -> BTreeSet<String> {
    let pattern = Regex::new(r"\bhtmlcut\.[a-z_]+\b").expect("valid schema regex");
    pattern
        .find_iter(text)
        .map(|matched| matched.as_str().to_owned())
        .collect()
}

fn documented_operation_ids(text: &str) -> BTreeSet<String> {
    let pattern =
        Regex::new(r"\b(?:document|source|select|slice)\.(?:parse|inspect|preview|extract)\b")
            .expect("valid operation regex");
    pattern
        .find_iter(text)
        .map(|matched| matched.as_str().to_owned())
        .collect()
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
    names
}

pub(super) fn known_operation_ids() -> BTreeSet<&'static str> {
    htmlcut_core::operation_catalog()
        .iter()
        .map(|descriptor| descriptor.id.as_str())
        .collect()
}
