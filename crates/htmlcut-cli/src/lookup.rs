use std::collections::BTreeSet;

use htmlcut_core::OperationId;
use htmlcut_core::cli_contract::{CliHelpDocument, OperationCliContract};

use crate::error::{CliError, internal_error, usage_error};

const MAX_SUGGESTIONS: usize = 3;

pub(crate) fn unknown_operation_id_error(requested: &str) -> CliError {
    let candidates = htmlcut_core::operation_catalog()
        .iter()
        .map(|descriptor| descriptor.id.as_str())
        .collect::<Vec<_>>();
    let catalog_command = htmlcut_core::cli_contract::cli_aux_command_display_command(
        htmlcut_core::cli_contract::CliAuxCommandId::Catalog,
    );
    usage_error(
        "CLI_OPERATION_ID_UNKNOWN",
        format!(
            "Unknown operation ID: {requested}.{} Use `htmlcut {catalog_command}` to list the valid operation IDs.",
            suggestion_suffix(requested, candidates),
        ),
    )
}

pub(crate) fn operation_contract(
    operation_id: OperationId,
) -> Result<&'static OperationCliContract, CliError> {
    require_operation_value(
        operation_id,
        "operation contract",
        htmlcut_core::cli_contract::cli_operation_contract(operation_id),
    )
}

pub(crate) fn operation_display_command(operation_id: OperationId) -> Result<String, CliError> {
    require_operation_value(
        operation_id,
        "display command",
        htmlcut_core::cli_contract::cli_operation_display_command(operation_id),
    )
}

pub(crate) fn operation_report_command(operation_id: OperationId) -> Result<String, CliError> {
    require_operation_value(
        operation_id,
        "report command",
        htmlcut_core::cli_contract::cli_operation_report_command(operation_id),
    )
}

pub(crate) fn operation_help_document(
    operation_id: OperationId,
) -> Result<CliHelpDocument, CliError> {
    require_operation_value(
        operation_id,
        "help document",
        htmlcut_core::cli_contract::cli_operation_help_document(operation_id),
    )
}

pub(crate) fn unknown_schema_error(
    requested_name: &str,
    requested_version: Option<u32>,
    available_schemas: &[crate::model::SchemaDocumentReport],
) -> CliError {
    let requested = requested_version
        .map(|version| format!("{requested_name}@{version}"))
        .unwrap_or_else(|| requested_name.to_owned());
    let matching_versions = available_schema_versions(requested_name, available_schemas);
    let name_candidates = available_schemas
        .iter()
        .map(|schema| schema.schema_name.as_str())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    let recovery = if !matching_versions.is_empty() {
        format!(
            " Available versions for `{requested_name}`: {}.",
            matching_versions
                .iter()
                .map(u32::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        )
    } else {
        suggestion_suffix(requested_name, name_candidates)
    };
    let schema_command = htmlcut_core::cli_contract::cli_aux_command_display_command(
        htmlcut_core::cli_contract::CliAuxCommandId::Schema,
    );

    usage_error(
        "CLI_SCHEMA_UNKNOWN",
        format!(
            "Unknown schema: {requested}.{recovery} Use `htmlcut {schema_command}` to list the valid schemas.",
        ),
    )
}

fn available_schema_versions(
    requested_name: &str,
    available_schemas: &[crate::model::SchemaDocumentReport],
) -> Vec<u32> {
    let mut versions = available_schemas
        .iter()
        .filter(|schema| schema.schema_name == requested_name)
        .map(|schema| schema.schema_version)
        .collect::<Vec<_>>();
    versions.sort_unstable();
    versions.dedup();
    versions
}

fn require_operation_value<T>(
    operation_id: OperationId,
    surface: &'static str,
    value: Option<T>,
) -> Result<T, CliError> {
    value.ok_or_else(|| missing_operation_contract_error(operation_id, surface))
}

pub(crate) fn missing_operation_contract_error(
    operation_id: OperationId,
    surface: &'static str,
) -> CliError {
    internal_error(
        "CLI_CONTRACT_MISSING",
        format!(
            "Internal HTMLCut CLI contract error: missing {surface} for {}.",
            operation_id.as_str()
        ),
    )
}

fn suggestion_suffix<'a>(requested: &str, candidates: impl IntoIterator<Item = &'a str>) -> String {
    let suggestions = suggest_nearest(requested, candidates);
    if suggestions.is_empty() {
        return String::new();
    }

    format!(
        " Did you mean {}?",
        suggestions
            .iter()
            .map(|suggestion| format!("`{suggestion}`"))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

pub(crate) fn suggest_nearest<'a>(
    requested: &str,
    candidates: impl IntoIterator<Item = &'a str>,
) -> Vec<&'a str> {
    let normalized_requested = requested.to_ascii_lowercase();
    let mut ranked = candidates
        .into_iter()
        .map(|candidate| {
            let normalized_candidate = candidate.to_ascii_lowercase();
            let prefix_match = normalized_candidate.starts_with(&normalized_requested)
                || normalized_requested.starts_with(&normalized_candidate);
            let contains_match = normalized_candidate.contains(&normalized_requested)
                || normalized_requested.contains(&normalized_candidate);
            let distance = levenshtein_distance(&normalized_requested, &normalized_candidate);
            (candidate, prefix_match, contains_match, distance)
        })
        .collect::<Vec<_>>();

    ranked.sort_by(|left, right| {
        left.3
            .cmp(&right.3)
            .then(right.1.cmp(&left.1))
            .then(right.2.cmp(&left.2))
            .then(left.0.cmp(right.0))
    });

    ranked
        .into_iter()
        .filter(|candidate| {
            candidate.1
                || candidate.2
                || candidate.3
                    <= normalized_requested
                        .len()
                        .max(candidate.0.len())
                        .saturating_div(3)
                        .max(2)
        })
        .map(|candidate| candidate.0)
        .take(MAX_SUGGESTIONS)
        .collect()
}

fn levenshtein_distance(left: &str, right: &str) -> usize {
    let left_chars = left.chars().collect::<Vec<_>>();
    let right_chars = right.chars().collect::<Vec<_>>();
    if left_chars.is_empty() {
        return right_chars.len();
    }
    if right_chars.is_empty() {
        return left_chars.len();
    }

    let mut previous = (0..=right_chars.len()).collect::<Vec<_>>();
    let mut current = vec![0; right_chars.len() + 1];

    for (left_index, left_char) in left_chars.iter().enumerate() {
        current[0] = left_index + 1;
        for (right_index, right_char) in right_chars.iter().enumerate() {
            let substitution = usize::from(left_char != right_char);
            current[right_index + 1] = (previous[right_index + 1] + 1)
                .min(current[right_index] + 1)
                .min(previous[right_index] + substitution);
        }
        std::mem::swap(&mut previous, &mut current);
    }

    previous[right_chars.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_schema(name: &str, version: u32) -> crate::model::SchemaDocumentReport {
        crate::model::SchemaDocumentReport {
            schema_name: name.to_owned(),
            schema_version: version,
            owner_surface: "htmlcut-cli".to_owned(),
            rust_shape: "Fixture".to_owned(),
            stability: htmlcut_core::SchemaStability::Versioned,
            json_schema: serde_json::json!({"type": "object"}),
        }
    }

    #[test]
    fn suggest_nearest_covers_empty_strings_and_ranking_rules() {
        assert_eq!(suggest_nearest("", ["schema"]), vec!["schema"]);
        assert_eq!(suggest_nearest("schema", [""]), vec![""]);
        assert_eq!(
            suggest_nearest("tract", ["extract", "schema"]),
            vec!["extract"]
        );
        assert_eq!(
            suggest_nearest(
                "selct.extract",
                ["select.extract", "slice.extract", "schema"]
            ),
            vec!["select.extract", "slice.extract"]
        );
        assert!(suggest_nearest("totally-unrelated", ["schema", "catalog"]).is_empty());
    }

    #[test]
    fn unknown_schema_error_prefers_matching_versions_over_name_suggestions() {
        let schemas = vec![
            fixture_schema("htmlcut.result", 1),
            fixture_schema("htmlcut.result", 3),
            fixture_schema("htmlcut.catalog_report", 4),
        ];

        let version_error = unknown_schema_error("htmlcut.result", Some(2), &schemas);
        assert!(
            version_error
                .message
                .contains("Available versions for `htmlcut.result`: 1, 3.")
        );

        let typo_error = unknown_schema_error("htmlcut.reslt", None, &schemas);
        assert!(
            typo_error
                .message
                .contains("Did you mean `htmlcut.result`?")
        );
    }

    #[test]
    fn unknown_operation_id_error_omits_suggestions_when_nothing_is_close() {
        let error = unknown_operation_id_error("zzzzzzzzzz");

        assert!(!error.message.contains("Did you mean"));
    }

    #[test]
    fn operation_lookup_helpers_surface_internal_contract_drift() {
        let error = missing_operation_contract_error(OperationId::SelectExtract, "report command");

        assert_eq!(error.code, "CLI_CONTRACT_MISSING");
        assert!(
            error
                .message
                .contains("missing report command for select.extract")
        );
    }

    #[test]
    fn operation_lookup_helpers_resolve_live_cli_metadata() {
        let contract = operation_contract(OperationId::SelectExtract).expect("operation contract");
        let display =
            operation_display_command(OperationId::SelectExtract).expect("display command");
        let report = operation_report_command(OperationId::SelectExtract).expect("report command");
        let help = operation_help_document(OperationId::SelectExtract).expect("help document");

        assert_eq!(contract.operation_id, OperationId::SelectExtract);
        assert_eq!(display, "select");
        assert_eq!(report, "select");
        assert!(!help.examples.is_empty());
    }
}
