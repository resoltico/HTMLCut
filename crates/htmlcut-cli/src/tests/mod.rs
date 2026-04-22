//! Internal behavior tests for `htmlcut-cli`'s parsing, preparation, rendering, and execution seams.

use super::*;
use clap::builder::TypedValueParser;
use clap::{CommandFactory, Parser};
use htmlcut_core::{
    AttributeName, DEFAULT_FETCH_TIMEOUT_MS, DEFAULT_INSPECTION_SAMPLE_LIMIT, DEFAULT_MAX_BYTES,
    DEFAULT_PREVIEW_CHARS, DEFAULT_REGEX_FLAGS, Diagnostic, DiagnosticLevel, ExtractionDefinition,
    ExtractionRequest, ExtractionResult, ExtractionSpec, ExtractionStrategy, FetchPreflightMode,
    PatternMode, SelectionSpec, SelectorQuery, SourceKind, SourceLoadAction, SourceLoadOutcome,
    SourceLoadStep, SourceMetadata, SourceRequest, ValueSpec, ValueType, WhitespaceMode,
    result::{
        DelimiterPairMatchMetadata, DocumentInspection, ExtractionMatch, ExtractionMatchMetadata,
        ExtractionStats, HeadingInspection, InspectionCount, LinkInspection, Range,
        SelectorMatchMetadata,
    },
};
use htmlcut_tempdir::tempdir;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};

fn run_vec(args: Vec<String>) -> (i32, String, String) {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit_code = run(args, &mut stdout, &mut stderr);
    (
        exit_code,
        String::from_utf8(stdout).expect("stdout utf8"),
        String::from_utf8(stderr).expect("stderr utf8"),
    )
}

fn write_fixture_file(dir: &Path, name: &str, contents: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, contents).expect("write fixture file");
    path
}

fn parser_value_names<P>(parser: P) -> Vec<String>
where
    P: TypedValueParser,
{
    parser
        .possible_values()
        .expect("parser possible values")
        .map(|value| value.get_name().to_owned())
        .collect()
}

fn shell_words(command: &str) -> Vec<String> {
    shell_words::split(command).expect("shell words")
}

fn option_value<'a>(tokens: &'a [String], flag: &str) -> Option<&'a str> {
    tokens.iter().enumerate().find_map(|(index, token)| {
        token.strip_prefix(&format!("{flag}=")).or_else(|| {
            if token == flag {
                tokens.get(index + 1).map(String::as_str)
            } else {
                None
            }
        })
    })
}

fn known_schema_names() -> std::collections::BTreeSet<String> {
    htmlcut_core::schema_catalog()
        .iter()
        .map(|descriptor| descriptor.schema_ref.schema_name.to_owned())
        .chain([
            crate::model::CATALOG_REPORT_SCHEMA_NAME.to_owned(),
            crate::model::EXTRACTION_COMMAND_REPORT_SCHEMA_NAME.to_owned(),
            crate::model::SCHEMA_COMMAND_REPORT_SCHEMA_NAME.to_owned(),
            crate::model::SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_NAME.to_owned(),
        ])
        .collect()
}

fn known_operation_ids() -> std::collections::BTreeSet<String> {
    htmlcut_core::operation_catalog()
        .iter()
        .map(|descriptor| descriptor.id.as_str().to_owned())
        .collect()
}

fn render_long_help(command: &mut clap::Command) -> String {
    let mut buffer = Vec::new();
    command
        .write_long_help(&mut buffer)
        .expect("render long help");
    String::from_utf8(buffer).expect("help utf8")
}

fn parameter_allowed_values(
    contract: &htmlcut_core::OperationCliContract,
    parameter: htmlcut_core::CliParameterId,
) -> Vec<String> {
    contract
        .parameters
        .iter()
        .find(|descriptor| descriptor.id == parameter)
        .map(|descriptor| {
            descriptor
                .allowed_values
                .iter()
                .copied()
                .map(htmlcut_core::render_cli_value)
                .collect()
        })
        .unwrap_or_default()
}

fn parameter_default_value(
    contract: &htmlcut_core::OperationCliContract,
    parameter: htmlcut_core::CliParameterId,
) -> Option<String> {
    contract
        .parameters
        .iter()
        .find(|descriptor| descriptor.id == parameter)
        .and_then(|descriptor| descriptor.default.map(htmlcut_core::render_cli_value))
}

fn assert_command_path_registered(command: &clap::Command, command_path: &[&str]) {
    let Some((head, tail)) = command_path.split_first() else {
        panic!("expected non-empty command path");
    };
    let subcommand = command
        .get_subcommands()
        .find(|subcommand| subcommand.get_name() == *head)
        .unwrap_or_else(|| panic!("missing clap command path segment {head:?}"));
    if !tail.is_empty() {
        assert_command_path_registered(subcommand, tail);
    }
}

fn write_definition_file(dir: &Path, name: &str, definition: &ExtractionDefinition) -> PathBuf {
    let path = dir.join(name);
    fs::write(
        &path,
        serde_json::to_string_pretty(definition).expect("serialize definition"),
    )
    .expect("write definition file");
    path
}

fn expect_cli_error<T>(result: Result<T, CliError>, label: &str) -> CliError {
    match result {
        Ok(_) => panic!("expected cli error: {label}"),
        Err(error) => error,
    }
}

fn workspace_package_field(manifest: &str, field: &str) -> Option<String> {
    let mut in_workspace_package = false;

    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_workspace_package = trimmed == "[workspace.package]";
            continue;
        }

        if !in_workspace_package {
            continue;
        }

        if let Some(value) = trimmed.strip_prefix(&format!("{field} = \""))
            && let Some(value) = value.strip_suffix('"')
        {
            return Some(value.to_owned());
        }
    }

    None
}

fn fixture_result(value: Value, value_type: ValueType) -> ExtractionResult {
    let value_spec = match value_type {
        ValueType::Text => ValueSpec::Text,
        ValueType::InnerHtml => ValueSpec::InnerHtml,
        ValueType::OuterHtml => ValueSpec::OuterHtml,
        ValueType::Attribute => ValueSpec::Attribute {
            name: AttributeName::new("href").expect("attribute name"),
        },
        ValueType::Structured => ValueSpec::Structured,
    };

    ExtractionResult {
        operation_id: htmlcut_core::OperationId::SelectExtract,
        schema_name: htmlcut_core::CORE_RESULT_SCHEMA_NAME.to_owned(),
        schema_version: htmlcut_core::CORE_RESULT_SCHEMA_VERSION,
        ok: true,
        source: SourceMetadata {
            kind: SourceKind::File,
            value: "/tmp/input.html".to_owned(),
            input_base_url: Some("https://example.com/docs/start.html".to_owned()),
            effective_base_url: Some("https://example.com/docs/start.html".to_owned()),
            bytes_read: 42,
            load_steps: Vec::new(),
            text: None,
        },
        document_title: Some("Fixture".to_owned()),
        extraction: ExtractionSpec::selector(SelectorQuery::new("article").expect("selector"))
            .with_selection(SelectionSpec::default())
            .with_value(value_spec),
        stats: ExtractionStats {
            duration_ms: 5,
            candidate_count: 2,
            match_count: 1,
        },
        matches: vec![ExtractionMatch {
            index: 1,
            path: Some("article:nth-of-type(1)".to_owned()),
            value_type,
            value,
            html: Some("<article>Hello</article>".to_owned()),
            text: Some("Hello".to_owned()),
            preview: "Hello".to_owned(),
            metadata: selector_metadata(2, 1, "article:nth-of-type(1)", "article", &[]),
        }],
        diagnostics: Vec::new(),
    }
}

fn attribute_map(entries: &[(&str, &str)]) -> BTreeMap<String, String> {
    entries
        .iter()
        .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
        .collect()
}

fn selector_metadata(
    candidate_count: usize,
    candidate_index: usize,
    path: &str,
    tag_name: &str,
    attributes: &[(&str, &str)],
) -> ExtractionMatchMetadata {
    ExtractionMatchMetadata::Selector(SelectorMatchMetadata {
        candidate_count,
        candidate_index,
        path: path.to_owned(),
        tag_name: tag_name.to_owned(),
        attributes: attribute_map(attributes),
    })
}

fn delimiter_metadata(
    candidate_count: usize,
    candidate_index: usize,
    selected_range: (usize, usize),
    inner_range: (usize, usize),
    outer_range: (usize, usize),
    include_start: bool,
    include_end: bool,
) -> ExtractionMatchMetadata {
    ExtractionMatchMetadata::DelimiterPair(DelimiterPairMatchMetadata {
        candidate_count,
        candidate_index,
        selected_range: Range {
            start: selected_range.0,
            end: selected_range.1,
        },
        inner_range: Range {
            start: inner_range.0,
            end: inner_range.1,
        },
        outer_range: Range {
            start: outer_range.0,
            end: outer_range.1,
        },
        include_start,
        include_end,
        matched_start: "<article>".to_owned(),
        matched_end: "</article>".to_owned(),
    })
}

fn fixture_inspection() -> SourceInspectionCommandReport {
    SourceInspectionCommandReport {
        tool: TOOL_NAME.to_owned(),
        engine: ENGINE_NAME.to_owned(),
        version: HTMLCUT_VERSION.to_owned(),
        schema_name: SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_NAME.to_owned(),
        schema_version: SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_VERSION,
        command: "inspect-source".to_owned(),
        operation_id: htmlcut_core::OperationId::SourceInspect,
        ok: true,
        source: SourceMetadata {
            kind: SourceKind::File,
            value: "/tmp/input.html".to_owned(),
            input_base_url: Some("https://example.com/docs/start.html".to_owned()),
            effective_base_url: Some("https://example.com/docs/start.html".to_owned()),
            bytes_read: 123,
            load_steps: Vec::new(),
            text: None,
        },
        document: Some(DocumentInspection {
            title: Some("Fixture".to_owned()),
            root_tag: "html".to_owned(),
            element_count: 12,
            text_char_count: 24,
            link_count: 2,
            image_count: 1,
            form_count: 0,
            table_count: 1,
            script_count: 0,
            style_count: 0,
            document_base_href: Some("../content/".to_owned()),
            top_tags: vec![InspectionCount {
                name: "a".to_owned(),
                count: 2,
            }],
            top_classes: vec![InspectionCount {
                name: "card".to_owned(),
                count: 2,
            }],
            headings: vec![HeadingInspection {
                level: 1,
                text: "Hello".to_owned(),
                path: "article:nth-of-type(1) > h1:nth-of-type(1)".to_owned(),
            }],
            links: vec![LinkInspection {
                text: "Guide".to_owned(),
                href: Some("../guide.html".to_owned()),
                resolved_href: Some("https://example.com/guide.html".to_owned()),
                path: "article:nth-of-type(1) > a:nth-of-type(1)".to_owned(),
            }],
        }),
        diagnostics: Vec::new(),
    }
}

mod contracts;
mod execution;
mod preparation;
mod rendering;
