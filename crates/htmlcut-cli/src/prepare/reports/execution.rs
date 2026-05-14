use htmlcut_core::{Diagnostic, ExtractionResult, SourceInspectionResult, result::ExtractionMatch};
use serde_json::{Map, Value};

use crate::metadata::{ENGINE_NAME, HTMLCUT_VERSION, TOOL_NAME};
use crate::model::{
    BundlePaths, EXTRACTION_COMMAND_REPORT_SCHEMA_NAME, EXTRACTION_COMMAND_REPORT_SCHEMA_VERSION,
    ExtractionCommandReport, SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_NAME,
    SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_VERSION, SourceInspectionCommandReport,
};

pub(crate) fn build_extraction_report(
    command: impl Into<String>,
    result: ExtractionResult,
    bundle: Option<BundlePaths>,
) -> ExtractionCommandReport {
    ExtractionCommandReport {
        tool: TOOL_NAME.to_owned(),
        engine: ENGINE_NAME.to_owned(),
        version: HTMLCUT_VERSION.to_owned(),
        schema_name: EXTRACTION_COMMAND_REPORT_SCHEMA_NAME.to_owned(),
        schema_version: EXTRACTION_COMMAND_REPORT_SCHEMA_VERSION,
        command: command.into(),
        operation_id: result.operation_id,
        ok: result.ok,
        source: result.source,
        extraction: result.extraction,
        stats: result.stats,
        document_title: result.document_title,
        matches: result
            .matches
            .into_iter()
            .map(normalize_match_payloads)
            .collect(),
        diagnostics: result
            .diagnostics
            .into_iter()
            .map(normalize_diagnostic_details)
            .collect(),
        bundle,
    }
}

pub(crate) fn build_source_inspection_report(
    command: impl Into<String>,
    result: SourceInspectionResult,
) -> SourceInspectionCommandReport {
    SourceInspectionCommandReport {
        tool: TOOL_NAME.to_owned(),
        engine: ENGINE_NAME.to_owned(),
        version: HTMLCUT_VERSION.to_owned(),
        schema_name: SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_NAME.to_owned(),
        schema_version: SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_VERSION,
        command: command.into(),
        operation_id: result.operation_id,
        ok: result.ok,
        source: result.source,
        document: result.document,
        diagnostics: result
            .diagnostics
            .into_iter()
            .map(normalize_diagnostic_details)
            .collect(),
    }
}

fn normalize_match_payloads(mut matched: ExtractionMatch) -> ExtractionMatch {
    matched.value = normalize_public_json_value(matched.value);
    matched
}

fn normalize_diagnostic_details(mut diagnostic: Diagnostic) -> Diagnostic {
    diagnostic.details = diagnostic.details.map(normalize_public_json_value);
    diagnostic
}

fn normalize_public_json_value(value: Value) -> Value {
    match value {
        Value::Object(object) => Value::Object(
            object
                .into_iter()
                .map(|(key, value)| (snake_case_key(&key), normalize_public_json_value(value)))
                .collect::<Map<String, Value>>(),
        ),
        Value::Array(values) => Value::Array(
            values
                .into_iter()
                .map(normalize_public_json_value)
                .collect(),
        ),
        other => other,
    }
}

fn snake_case_key(key: &str) -> String {
    let mut normalized = String::with_capacity(key.len() + 4);
    let mut previous_was_underscore = false;

    for character in key.chars() {
        if character == '-' || character == ' ' {
            if !previous_was_underscore {
                normalized.push('_');
                previous_was_underscore = true;
            }
            continue;
        }

        if character.is_uppercase() {
            if !normalized.is_empty() && !previous_was_underscore {
                normalized.push('_');
            }
            for lowercase in character.to_lowercase() {
                normalized.push(lowercase);
            }
            previous_was_underscore = false;
            continue;
        }

        normalized.push(character);
        previous_was_underscore = character == '_';
    }

    normalized
}

#[cfg(test)]
mod tests {
    use super::*;
    use htmlcut_core::{
        CORE_RESULT_SCHEMA_NAME, CORE_RESULT_SCHEMA_VERSION, CORE_SOURCE_INSPECTION_SCHEMA_NAME,
        CORE_SOURCE_INSPECTION_SCHEMA_VERSION, DiagnosticCode, DiagnosticLevel, ExtractionSpec,
        OperationId, SourceKind, SourceMetadata, ValueType,
        result::{ExtractionMatchMetadata, ExtractionStats, SelectorMatchMetadata},
    };
    use serde_json::json;
    use std::collections::BTreeMap;

    fn source_metadata() -> SourceMetadata {
        SourceMetadata {
            kind: SourceKind::Memory,
            value: "<article>Alpha</article>".to_owned(),
            input_base_url: None,
            effective_base_url: None,
            bytes_read: 24,
            load_steps: Vec::new(),
            text: None,
        }
    }

    #[test]
    fn extraction_and_inspection_reports_normalize_public_json_payload_keys() {
        let extraction = build_extraction_report(
            "select",
            ExtractionResult {
                schema_name: CORE_RESULT_SCHEMA_NAME.to_owned(),
                schema_version: CORE_RESULT_SCHEMA_VERSION,
                operation_id: OperationId::SelectExtract,
                ok: true,
                source: source_metadata(),
                extraction: ExtractionSpec::selector(
                    htmlcut_core::SelectorQuery::new("article").expect("selector"),
                ),
                stats: ExtractionStats {
                    duration_ms: 1,
                    candidate_count: 1,
                    match_count: 1,
                },
                document_title: None,
                matches: vec![htmlcut_core::result::ExtractionMatch {
                    index: 1,
                    path: Some("article".to_owned()),
                    value_type: ValueType::Structured,
                    value: json!({
                        "A": 7,
                        "candidateCount": 1,
                        "dash-- space": "Alpha",
                        "mixed Key-Name": [{ "innerHtmlOutput": "<p>Alpha</p>" }],
                        "snake__Case": true
                    }),
                    html: Some("<article>Alpha</article>".to_owned()),
                    text: Some("Alpha".to_owned()),
                    preview: "Alpha".to_owned(),
                    metadata: ExtractionMatchMetadata::Selector(SelectorMatchMetadata {
                        candidate_count: 1,
                        candidate_index: 1,
                        path: "article".to_owned(),
                        tag_name: "article".to_owned(),
                        attributes: BTreeMap::new(),
                    }),
                }],
                diagnostics: vec![Diagnostic {
                    level: DiagnosticLevel::Warning,
                    code: DiagnosticCode::MultipleMatches,
                    message: "warning".to_owned(),
                    details: Some(json!({
                        "resolvedHref": "https://example.test/guide",
                        "nestedValue": [{ "tagName": "article" }]
                    })),
                }],
            },
            None,
        );
        assert_eq!(extraction.matches[0].value["candidate_count"], 1);
        assert_eq!(extraction.matches[0].value["a"], 7);
        assert_eq!(extraction.matches[0].value["dash_space"], "Alpha");
        assert_eq!(
            extraction.matches[0].value["mixed_key_name"][0]["inner_html_output"],
            "<p>Alpha</p>"
        );
        assert_eq!(extraction.matches[0].value["snake__case"], true);
        assert_eq!(
            extraction.diagnostics[0].details.as_ref().expect("details")["resolved_href"],
            "https://example.test/guide"
        );
        assert_eq!(
            extraction.diagnostics[0].details.as_ref().expect("details")["nested_value"][0]["tag_name"],
            "article"
        );

        let inspection = build_source_inspection_report(
            "inspect-source",
            SourceInspectionResult {
                schema_name: CORE_SOURCE_INSPECTION_SCHEMA_NAME.to_owned(),
                schema_version: CORE_SOURCE_INSPECTION_SCHEMA_VERSION,
                operation_id: OperationId::SourceInspect,
                ok: true,
                source: source_metadata(),
                document: None,
                diagnostics: vec![Diagnostic {
                    level: DiagnosticLevel::Warning,
                    code: DiagnosticCode::SourceLoadFailed,
                    message: "warning".to_owned(),
                    details: Some(json!({
                        "contentType": "text/html",
                        "tagNames": [{ "primaryHeading": "h1" }]
                    })),
                }],
            },
        );
        assert_eq!(
            inspection.diagnostics[0].details.as_ref().expect("details")["content_type"],
            "text/html"
        );
        assert_eq!(
            inspection.diagnostics[0].details.as_ref().expect("details")["tag_names"][0]["primary_heading"],
            "h1"
        );
    }
}
