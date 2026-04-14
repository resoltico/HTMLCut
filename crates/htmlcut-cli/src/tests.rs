//! Internal behavior tests for `htmlcut-cli`'s parsing, preparation, rendering, and execution seams.

use super::*;
use clap::Parser;
use htmlcut_core::{
    AttributeName, DEFAULT_FETCH_TIMEOUT_MS, DEFAULT_INSPECTION_SAMPLE_LIMIT, DEFAULT_MAX_BYTES,
    DEFAULT_PREVIEW_CHARS, DEFAULT_REGEX_FLAGS, Diagnostic, DiagnosticLevel, ExtractionDefinition,
    ExtractionRequest, ExtractionResult, ExtractionSpec, ExtractionStrategy, FetchPreflightMode,
    SelectionSpec, SelectorQuery, SourceKind, SourceMetadata, SourceRequest, ValueSpec, ValueType,
    WhitespaceMode,
    result::{
        DelimiterPairMatchMetadata, DocumentInspection, ExtractionMatch, ExtractionMatchMetadata,
        ExtractionStats, HeadingInspection, InspectionCount, LinkInspection, Range,
        SelectorMatchMetadata,
    },
};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

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

#[test]
fn parse_byte_size_accepts_units() {
    assert_eq!(parse_byte_size("1kb").expect("byte size"), 1024);
    assert_eq!(parse_byte_size("1.5mb").expect("byte size"), 1_572_864);
    assert_eq!(parse_byte_size("1gb").expect("byte size"), 1_073_741_824);
    assert!(parse_byte_size("banana").is_err());
    assert!(parse_byte_size("1tb").is_err());
    assert!(parse_byte_size("0").is_err());
}

#[test]
fn preview_and_manifest_helpers_cover_remaining_branches() {
    assert_eq!(
        validate_preview_chars(32).expect("preview chars"),
        NonZeroUsize::new(32).expect("preview chars")
    );
    assert!(validate_preview_chars(0).is_err());
    assert_eq!(render_text_preview("short", 32), "short");
    assert_eq!(render_text_preview("preview", 3), "pre...");
    assert_eq!(
        workspace_package_field("[workspace.package]\nversion = \"3.0.0\"\n", "description"),
        None
    );
    assert_eq!(
        workspace_package_field(
            "[package]\ndescription = \"wrong\"\n[workspace.package]\ndescription = \"right\"\n",
            "description"
        ),
        Some("right".to_owned())
    );
    assert_eq!(
        workspace_package_field(
            "[workspace.package]\ndescription = \"broken\n",
            "description"
        ),
        None
    );

    let mut input_only = fixture_inspection();
    input_only.source.effective_base_url = None;
    let rendered = render_source_inspection_text(&input_only, DEFAULT_PREVIEW_CHARS);
    assert!(rendered.contains("Input base URL: https://example.com/docs/start.html"));
    assert!(!rendered.contains("Effective base URL: https://example.com/docs/start.html"));
}

#[test]
fn catalog_and_preview_renderers_cover_remaining_branches() {
    let empty_catalog = CatalogCommandReport {
        tool: TOOL_NAME.to_owned(),
        version: HTMLCUT_VERSION.to_owned(),
        schema_name: CATALOG_REPORT_SCHEMA_NAME.to_owned(),
        schema_version: crate::model::CATALOG_SCHEMA_VERSION,
        schema_profile: htmlcut_core::HTMLCUT_JSON_SCHEMA_PROFILE.to_owned(),
        description: HTMLCUT_DESCRIPTION.to_owned(),
        command: "catalog".to_owned(),
        operations: Vec::new(),
    };
    assert_eq!(
        render_catalog_text(&empty_catalog),
        format!("{TOOL_NAME} {HTMLCUT_VERSION}\n{HTMLCUT_DESCRIPTION}")
    );
    assert_eq!(
        render_catalog_surface(None, &CatalogAvailability::Cli),
        "cli".to_owned()
    );

    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "catalog".to_owned(),
        "--output".to_owned(),
        "text".to_owned(),
    ]);
    assert_eq!(exit_code, 0);
    assert!(stdout.contains("Operations:"));
    assert!(stderr.is_empty());

    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "catalog".to_owned(),
        "--operation".to_owned(),
        "slice.extract".to_owned(),
    ]);
    assert_eq!(exit_code, 0);
    assert!(stdout.contains("Operation:"));
    assert!(stdout.contains("core: extract(ExtractionRequest{kind=slice}, RuntimeOptions)"));
    assert!(stdout.contains("request: ExtractionRequest + RuntimeOptions"));
    assert!(
        stdout.contains("request schemas: htmlcut.extraction_request@4, htmlcut.runtime_options@4")
    );
    assert!(stdout.contains("result: ExtractionResult"));
    assert!(stdout.contains("result schemas: htmlcut.extraction_result@4"));
    assert!(stdout.contains("usage: htmlcut slice [OPTIONS] --from <FROM> --to <TO> [INPUT]"));
    assert!(stdout.contains("default output: text"));
    assert!(stdout.contains("default output overrides:"));
    assert!(stdout.contains("when --value is structured => json"));
    assert!(stdout.contains("constraints:"));
    assert!(stdout.contains("requires --bundle when --output is none"));
    assert!(stdout.contains("restricts --output to json, none when --value is structured"));
    assert!(stdout.contains("parameters:"));
    assert!(stdout.contains("option --request-file <PATH> | optional"));
    assert!(stdout.contains("option --fetch-preflight <FETCH_PREFLIGHT> | optional"));
    assert!(
        stdout
            .contains("positional <INPUT> | conditional (required unless --request-file is used)")
    );
    assert!(
        stdout.contains(
            "option --from <FROM> | conditional (required unless --request-file is used)"
        )
    );
    assert!(stdout.contains("option --regex-flags <REGEX_FLAGS> | conditional (allowed only when --pattern regex is used)"));
    assert!(stdout.contains("option --output-file <PATH> | optional"));
    assert!(stdout.contains(
        "For --value outer-html, HTMLCut returns the full outer matched range including both boundaries."
    ));
    assert!(stderr.is_empty());

    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "catalog".to_owned(),
        "--output".to_owned(),
        "json".to_owned(),
        "--operation".to_owned(),
        "unknown.operation".to_owned(),
    ]);
    assert_eq!(exit_code, EXIT_CODE_USAGE);
    assert!(stdout.contains("\"code\": \"CLI_OPERATION_ID_UNKNOWN\""));
    assert!(stderr.is_empty());

    let select_preview_lines = render_preview_match_lines(
        htmlcut_core::OperationId::SelectPreview,
        &ExtractionMatch {
            index: 2,
            path: None,
            value_type: ValueType::Structured,
            value: serde_json::json!({}),
            html: None,
            text: None,
            preview: "structured preview".to_owned(),
            metadata: selector_metadata(1, 1, "article:nth-of-type(1)", "article", &[]),
        },
    );
    assert!(
        select_preview_lines
            .iter()
            .any(|line| line == "2. (no path)")
    );
    assert!(
        select_preview_lines
            .iter()
            .any(|line| line == "   preview: structured preview")
    );

    let rich_select_preview_lines = render_preview_match_lines(
        htmlcut_core::OperationId::SelectPreview,
        &ExtractionMatch {
            index: 4,
            path: Some("article:nth-of-type(1)".to_owned()),
            value_type: ValueType::Structured,
            value: serde_json::json!({}),
            html: None,
            text: Some("Alpha beta".to_owned()),
            preview: "unused".to_owned(),
            metadata: selector_metadata(
                3,
                2,
                "article:nth-of-type(1)",
                "article",
                &[("class", "card featured")],
            ),
        },
    );
    assert!(
        rich_select_preview_lines
            .iter()
            .any(|line| line == "   attributes: class=\"card featured\"")
    );
    assert!(
        rich_select_preview_lines
            .iter()
            .any(|line| line == "   text: Alpha beta")
    );

    let slice_preview_lines = render_preview_match_lines(
        htmlcut_core::OperationId::SlicePreview,
        &ExtractionMatch {
            index: 3,
            path: None,
            value_type: ValueType::Structured,
            value: serde_json::json!({}),
            html: None,
            text: None,
            preview: "slice preview".to_owned(),
            metadata: delimiter_metadata(9, 7, (1, 12), (4, 9), (1, 12), true, true),
        },
    );
    assert!(
        slice_preview_lines
            .iter()
            .any(|line| line == "3. range 1..12")
    );
    assert!(
        slice_preview_lines
            .iter()
            .any(|line| line == "   candidate index: 7")
    );
    assert!(
        slice_preview_lines
            .iter()
            .any(|line| line == "   selected range: 1..12")
    );
    assert!(
        slice_preview_lines
            .iter()
            .any(|line| line == "   inner range: 4..9")
    );
    assert!(
        slice_preview_lines
            .iter()
            .any(|line| line == "   outer range: 1..12")
    );
    assert!(
        slice_preview_lines
            .iter()
            .any(|line| line == "   include start: true")
    );
    assert!(
        slice_preview_lines
            .iter()
            .any(|line| line == "   include end: true")
    );
    assert!(
        slice_preview_lines
            .iter()
            .any(|line| line == "   preview: slice preview")
    );

    let rich_slice_preview_lines = render_preview_match_lines(
        htmlcut_core::OperationId::SlicePreview,
        &ExtractionMatch {
            index: 5,
            path: None,
            value_type: ValueType::Structured,
            value: serde_json::json!({}),
            html: None,
            text: Some("alpha beta".to_owned()),
            preview: "unused".to_owned(),
            metadata: delimiter_metadata(10, 8, (2, 7), (2, 7), (1, 8), false, false),
        },
    );
    assert!(
        rich_slice_preview_lines
            .iter()
            .any(|line| line == "   candidate index: 8")
    );
    assert!(
        rich_slice_preview_lines
            .iter()
            .any(|line| line == "   include start: false")
    );
    assert!(
        rich_slice_preview_lines
            .iter()
            .any(|line| line == "   include end: false")
    );
    assert!(
        rich_slice_preview_lines
            .iter()
            .any(|line| line == "   inner range: 2..7")
    );
    assert!(
        rich_slice_preview_lines
            .iter()
            .any(|line| line == "   outer range: 1..8")
    );
    assert!(
        rich_slice_preview_lines
            .iter()
            .any(|line| line == "   text: alpha beta")
    );

    let fragment_signal_slice_preview_lines = render_preview_match_lines(
        htmlcut_core::OperationId::SlicePreview,
        &ExtractionMatch {
            index: 8,
            path: None,
            value_type: ValueType::Structured,
            value: serde_json::json!({}),
            html: Some("START::Alpha::END".to_owned()),
            text: Some(String::new()),
            preview: "unused".to_owned(),
            metadata: delimiter_metadata(2, 1, (12, 12), (12, 12), (5, 17), false, false),
        },
    );
    assert!(
        fragment_signal_slice_preview_lines
            .iter()
            .any(|line| line == "   fragment: START::Alpha::END")
    );
    assert!(
        fragment_signal_slice_preview_lines
            .iter()
            .any(|line| line == "   text: ")
    );

    let sparse_slice_preview_lines = render_preview_match_lines(
        htmlcut_core::OperationId::SlicePreview,
        &ExtractionMatch {
            index: 6,
            path: None,
            value_type: ValueType::Structured,
            value: serde_json::json!({}),
            html: None,
            text: Some("fallback branch coverage".to_owned()),
            preview: "unused".to_owned(),
            metadata: delimiter_metadata(1, 1, (10, 20), (10, 20), (9, 21), false, false),
        },
    );
    assert!(
        sparse_slice_preview_lines
            .iter()
            .any(|line| line == "6. range 10..20")
    );
    assert!(
        sparse_slice_preview_lines
            .iter()
            .all(|line| !line.contains("source index:"))
    );
    assert!(
        sparse_slice_preview_lines
            .iter()
            .any(|line| line == "   candidate index: 1")
    );
    assert!(
        sparse_slice_preview_lines
            .iter()
            .any(|line| line == "   selected range: 10..20")
    );
    assert!(
        sparse_slice_preview_lines
            .iter()
            .any(|line| line == "   text: fallback branch coverage")
    );
    assert_eq!(
        render_preview_location(
            htmlcut_core::OperationId::SlicePreview,
            &ExtractionMatch {
                index: 7,
                path: None,
                value_type: ValueType::Structured,
                value: serde_json::json!({}),
                html: None,
                text: None,
                preview: "unused".to_owned(),
                metadata: selector_metadata(1, 1, "article:nth-of-type(1)", "article", &[]),
            }
        ),
        "(no path)".to_owned()
    );

    let fallback_preview_lines = render_preview_match_lines(
        htmlcut_core::OperationId::SelectExtract,
        &ExtractionMatch {
            index: 1,
            path: None,
            value_type: ValueType::Structured,
            value: serde_json::json!({}),
            html: None,
            text: None,
            preview: "fallback".to_owned(),
            metadata: selector_metadata(1, 1, "article:nth-of-type(1)", "article", &[]),
        },
    );
    assert!(
        fallback_preview_lines
            .iter()
            .any(|line| line == "   preview: fallback")
    );

    assert_eq!(render_attribute_summary(&attribute_map(&[])), None);
    assert_eq!(
        render_attribute_summary(&attribute_map(&[("count", "1")])),
        Some("count=\"1\"".to_owned())
    );
    assert_eq!(
        render_attribute_summary(&attribute_map(&[("class", "card")])),
        Some("class=\"card\"".to_owned())
    );
    assert_eq!(
        render_range_summary(Some(&Range { start: 9, end: 12 })),
        Some("9..12".to_owned())
    );
    assert_eq!(render_range_summary(None), None);
    assert_eq!(
        compact_inline_preview("alpha beta gamma", 5),
        "alpha...".to_owned()
    );
}

#[test]
fn schema_and_catalog_renderers_cover_optional_surfaces() {
    let core_only_operation = CatalogOperationReport {
        operation_id: htmlcut_core::OperationId::DocumentParse,
        command: None,
        availability: CatalogAvailability::CoreOnly,
        summary: "Core-only parse".to_owned(),
        core_surface: "parse_document(SourceRequest, RuntimeOptions)".to_owned(),
        request_contract: CatalogContractSurface {
            rust_shape: "SourceRequest + RuntimeOptions".to_owned(),
            schema_refs: Vec::new(),
        },
        result_contract: CatalogContractSurface {
            rust_shape: "ParseDocumentResult".to_owned(),
            schema_refs: vec![SchemaRefReport {
                schema_name: "htmlcut.parse_document_result".to_owned(),
                schema_version: 1,
            }],
        },
        command_contract: None,
    };
    let contract_operation = CatalogOperationReport {
        operation_id: htmlcut_core::OperationId::SelectExtract,
        command: Some("select".to_owned()),
        availability: CatalogAvailability::Cli,
        summary: "Synthetic contract".to_owned(),
        core_surface: "extract(ExtractionRequest{kind=selector}, RuntimeOptions)".to_owned(),
        request_contract: CatalogContractSurface {
            rust_shape: "ExtractionRequest + RuntimeOptions".to_owned(),
            schema_refs: vec![SchemaRefReport {
                schema_name: "htmlcut.extraction_request".to_owned(),
                schema_version: 2,
            }],
        },
        result_contract: CatalogContractSurface {
            rust_shape: "ExtractionResult".to_owned(),
            schema_refs: vec![SchemaRefReport {
                schema_name: "htmlcut.extraction_result".to_owned(),
                schema_version: 3,
            }],
        },
        command_contract: Some(CatalogCommandContract {
            invocation: "htmlcut select [OPTIONS] --css <CSS> [INPUT]".to_owned(),
            inputs: vec!["file".to_owned(), "url".to_owned(), "stdin".to_owned()],
            default_match: Some("first".to_owned()),
            selection_modes: vec!["single".to_owned(), "all".to_owned()],
            default_value: Some("text".to_owned()),
            value_modes: vec!["text".to_owned(), "structured".to_owned()],
            default_output: Some("text".to_owned()),
            default_output_overrides: vec![CatalogConditionalDefault {
                value: "json".to_owned(),
                when: CatalogCondition {
                    parameter: "--value".to_owned(),
                    values: vec!["structured".to_owned()],
                },
            }],
            output_modes: vec!["text".to_owned(), "json".to_owned(), "none".to_owned()],
            constraints: vec![
                CatalogConstraint::RequiresParameter {
                    parameter: "--attribute".to_owned(),
                    when: CatalogCondition {
                        parameter: "--value".to_owned(),
                        values: vec!["attribute".to_owned()],
                    },
                },
                CatalogConstraint::AllowedOnlyWhen {
                    parameter: "--regex-flags".to_owned(),
                    when: CatalogCondition {
                        parameter: "--pattern".to_owned(),
                        values: vec!["regex".to_owned()],
                    },
                },
                CatalogConstraint::RestrictsParameterValues {
                    parameter: "--output".to_owned(),
                    allowed_values: vec!["json".to_owned(), "none".to_owned()],
                    when: CatalogCondition {
                        parameter: "--value".to_owned(),
                        values: vec!["structured".to_owned()],
                    },
                },
            ],
            notes: vec!["Synthetic note".to_owned()],
            examples: vec!["htmlcut select ./page.html --css article".to_owned()],
            parameters: vec![
                CatalogParameterSpec {
                    section: "Source".to_owned(),
                    name: "<INPUT>".to_owned(),
                    kind: CatalogParameterKind::Positional,
                    requirement: CatalogParameterRequirement::Conditional,
                    requirement_note: Some("required unless --request-file is used".to_owned()),
                    value_hint: None,
                    default: None,
                    allowed_values: Vec::new(),
                    summary: "HTML input source.".to_owned(),
                },
                CatalogParameterSpec {
                    section: "Extraction".to_owned(),
                    name: "--value".to_owned(),
                    kind: CatalogParameterKind::Option,
                    requirement: CatalogParameterRequirement::Optional,
                    requirement_note: None,
                    value_hint: Some("VALUE".to_owned()),
                    default: Some("text".to_owned()),
                    allowed_values: vec!["text".to_owned(), "structured".to_owned()],
                    summary: "Choose the extracted value.".to_owned(),
                },
                CatalogParameterSpec {
                    section: "Extraction".to_owned(),
                    name: "--attribute".to_owned(),
                    kind: CatalogParameterKind::Option,
                    requirement: CatalogParameterRequirement::Conditional,
                    requirement_note: Some("required when --value attribute is used".to_owned()),
                    value_hint: Some("ATTRIBUTE".to_owned()),
                    default: None,
                    allowed_values: Vec::new(),
                    summary: "Attribute name.".to_owned(),
                },
            ],
        }),
    };
    let report = CatalogCommandReport {
        tool: TOOL_NAME.to_owned(),
        version: HTMLCUT_VERSION.to_owned(),
        schema_name: CATALOG_REPORT_SCHEMA_NAME.to_owned(),
        schema_version: crate::model::CATALOG_SCHEMA_VERSION,
        schema_profile: htmlcut_core::HTMLCUT_JSON_SCHEMA_PROFILE.to_owned(),
        description: HTMLCUT_DESCRIPTION.to_owned(),
        command: "catalog".to_owned(),
        operations: vec![core_only_operation, contract_operation],
    };
    let rendered_catalog = render_catalog_text(&report);
    assert!(rendered_catalog.contains("Core-only parse"));
    assert!(!rendered_catalog.contains("inputs: file | url | stdin"));

    let single_operation_catalog = CatalogCommandReport {
        operations: vec![CatalogOperationReport {
            operation_id: htmlcut_core::OperationId::SelectExtract,
            command: Some("select".to_owned()),
            availability: CatalogAvailability::Cli,
            summary: "Synthetic contract".to_owned(),
            core_surface: "extract(ExtractionRequest{kind=selector}, RuntimeOptions)".to_owned(),
            request_contract: CatalogContractSurface {
                rust_shape: "ExtractionRequest + RuntimeOptions".to_owned(),
                schema_refs: vec![SchemaRefReport {
                    schema_name: "htmlcut.extraction_request".to_owned(),
                    schema_version: 2,
                }],
            },
            result_contract: CatalogContractSurface {
                rust_shape: "ExtractionResult".to_owned(),
                schema_refs: vec![SchemaRefReport {
                    schema_name: "htmlcut.extraction_result".to_owned(),
                    schema_version: 3,
                }],
            },
            command_contract: Some(CatalogCommandContract {
                invocation: "htmlcut select [OPTIONS] --css <CSS> [INPUT]".to_owned(),
                inputs: vec!["file".to_owned(), "url".to_owned(), "stdin".to_owned()],
                default_match: Some("first".to_owned()),
                selection_modes: vec!["single".to_owned(), "all".to_owned()],
                default_value: Some("text".to_owned()),
                value_modes: vec!["text".to_owned(), "structured".to_owned()],
                default_output: Some("text".to_owned()),
                default_output_overrides: vec![CatalogConditionalDefault {
                    value: "json".to_owned(),
                    when: CatalogCondition {
                        parameter: "--value".to_owned(),
                        values: vec!["structured".to_owned()],
                    },
                }],
                output_modes: vec!["text".to_owned(), "json".to_owned(), "none".to_owned()],
                constraints: vec![
                    CatalogConstraint::RequiresParameter {
                        parameter: "--attribute".to_owned(),
                        when: CatalogCondition {
                            parameter: "--value".to_owned(),
                            values: vec!["attribute".to_owned()],
                        },
                    },
                    CatalogConstraint::AllowedOnlyWhen {
                        parameter: "--regex-flags".to_owned(),
                        when: CatalogCondition {
                            parameter: "--pattern".to_owned(),
                            values: vec!["regex".to_owned()],
                        },
                    },
                    CatalogConstraint::RestrictsParameterValues {
                        parameter: "--output".to_owned(),
                        allowed_values: vec!["json".to_owned(), "none".to_owned()],
                        when: CatalogCondition {
                            parameter: "--value".to_owned(),
                            values: vec!["structured".to_owned()],
                        },
                    },
                ],
                notes: vec!["Synthetic note".to_owned()],
                examples: vec!["htmlcut select ./page.html --css article".to_owned()],
                parameters: vec![
                    CatalogParameterSpec {
                        section: "Source".to_owned(),
                        name: "<INPUT>".to_owned(),
                        kind: CatalogParameterKind::Positional,
                        requirement: CatalogParameterRequirement::Required,
                        requirement_note: None,
                        value_hint: None,
                        default: None,
                        allowed_values: Vec::new(),
                        summary: "HTML input source.".to_owned(),
                    },
                    CatalogParameterSpec {
                        section: "Extraction".to_owned(),
                        name: "--value".to_owned(),
                        kind: CatalogParameterKind::Option,
                        requirement: CatalogParameterRequirement::Optional,
                        requirement_note: None,
                        value_hint: Some("VALUE".to_owned()),
                        default: Some("text".to_owned()),
                        allowed_values: vec!["text".to_owned(), "structured".to_owned()],
                        summary: "Choose the extracted value.".to_owned(),
                    },
                    CatalogParameterSpec {
                        section: "Extraction".to_owned(),
                        name: "--attribute".to_owned(),
                        kind: CatalogParameterKind::Option,
                        requirement: CatalogParameterRequirement::Conditional,
                        requirement_note: Some(
                            "required when --value attribute is used".to_owned(),
                        ),
                        value_hint: Some("ATTRIBUTE".to_owned()),
                        default: None,
                        allowed_values: Vec::new(),
                        summary: "Attribute name.".to_owned(),
                    },
                ],
            }),
        }],
        ..report
    };
    let rendered_single_catalog = render_catalog_text(&single_operation_catalog);
    assert!(rendered_single_catalog.contains("inputs: file | url | stdin"));
    assert!(rendered_single_catalog.contains("default match: first"));
    assert!(rendered_single_catalog.contains("match modes: single, all"));
    assert!(rendered_single_catalog.contains("default value: text"));
    assert!(rendered_single_catalog.contains("value modes: text, structured"));
    assert!(rendered_single_catalog.contains("default output: text"));
    assert!(rendered_single_catalog.contains("default output overrides:"));
    assert!(rendered_single_catalog.contains("requires --attribute when --value is attribute"));
    assert!(rendered_single_catalog.contains("allows --regex-flags only when --pattern is regex"));
    assert!(
        rendered_single_catalog
            .contains("restricts --output to json, none when --value is structured")
    );
    assert!(rendered_single_catalog.contains("option --attribute <ATTRIBUTE> | conditional"));
    assert!(rendered_single_catalog.contains("default: text"));
    assert!(rendered_single_catalog.contains("values: text, structured"));

    let single_schema = SchemaCommandReport {
        tool: TOOL_NAME.to_owned(),
        version: HTMLCUT_VERSION.to_owned(),
        schema_name: SCHEMA_COMMAND_REPORT_SCHEMA_NAME.to_owned(),
        schema_version: SCHEMA_COMMAND_REPORT_SCHEMA_VERSION,
        schema_profile: htmlcut_core::HTMLCUT_JSON_SCHEMA_PROFILE.to_owned(),
        description: HTMLCUT_DESCRIPTION.to_owned(),
        command: "schema".to_owned(),
        schemas: vec![SchemaDocumentReport {
            schema_name: "synthetic.single".to_owned(),
            schema_version: 7,
            owner_surface: "tests".to_owned(),
            rust_shape: "Synthetic".to_owned(),
            stability: htmlcut_core::SchemaStability::Frozen,
            json_schema: Value::String("not an object".to_owned()),
        }],
    };
    let rendered_single_schema = render_schema_text(&single_schema);
    assert!(rendered_single_schema.contains("Schema:"));
    assert!(rendered_single_schema.contains("synthetic.single@7 | tests | frozen"));
    assert!(rendered_single_schema.contains("json schema keys: (not-an-object)"));

    let multi_schema = SchemaCommandReport {
        schemas: vec![
            SchemaDocumentReport {
                schema_name: "synthetic.a".to_owned(),
                schema_version: 1,
                owner_surface: "tests".to_owned(),
                rust_shape: "A".to_owned(),
                stability: htmlcut_core::SchemaStability::Versioned,
                json_schema: serde_json::json!({ "type": "object" }),
            },
            SchemaDocumentReport {
                schema_name: "synthetic.b".to_owned(),
                schema_version: 2,
                owner_surface: "tests".to_owned(),
                rust_shape: "B".to_owned(),
                stability: htmlcut_core::SchemaStability::Frozen,
                json_schema: serde_json::json!({ "type": "object" }),
            },
        ],
        ..single_schema
    };
    let rendered_multi_schema = render_schema_text(&multi_schema);
    assert!(rendered_multi_schema.contains("Schemas:"));
    assert!(rendered_multi_schema.contains("synthetic.a@1 | tests | versioned"));
    assert!(rendered_multi_schema.contains("synthetic.b@2 | tests | frozen"));
    assert!(!rendered_multi_schema.contains("json schema keys:"));
}

#[test]
fn direct_render_helpers_cover_empty_optional_branches() {
    let minimal_contract = CatalogCommandContract {
        invocation: "htmlcut select <INPUT>".to_owned(),
        inputs: Vec::new(),
        default_match: None,
        selection_modes: Vec::new(),
        default_value: None,
        value_modes: Vec::new(),
        default_output: None,
        default_output_overrides: Vec::new(),
        output_modes: Vec::new(),
        constraints: Vec::new(),
        notes: Vec::new(),
        examples: Vec::new(),
        parameters: Vec::new(),
    };
    let minimal_report = CatalogCommandReport {
        tool: TOOL_NAME.to_owned(),
        version: HTMLCUT_VERSION.to_owned(),
        schema_name: CATALOG_REPORT_SCHEMA_NAME.to_owned(),
        schema_version: crate::model::CATALOG_SCHEMA_VERSION,
        schema_profile: htmlcut_core::HTMLCUT_JSON_SCHEMA_PROFILE.to_owned(),
        description: HTMLCUT_DESCRIPTION.to_owned(),
        command: "catalog".to_owned(),
        operations: vec![CatalogOperationReport {
            operation_id: htmlcut_core::OperationId::DocumentParse,
            command: Some("select".to_owned()),
            availability: CatalogAvailability::Cli,
            summary: "Minimal".to_owned(),
            core_surface: "BareCoreSurface".to_owned(),
            request_contract: CatalogContractSurface {
                rust_shape: "BareShape".to_owned(),
                schema_refs: Vec::new(),
            },
            result_contract: CatalogContractSurface {
                rust_shape: "BareResult".to_owned(),
                schema_refs: Vec::new(),
            },
            command_contract: Some(minimal_contract),
        }],
    };
    let minimal_render = render_catalog_text(&minimal_report);
    assert!(minimal_render.contains("usage: htmlcut select <INPUT>"));
    assert!(minimal_render.contains("request: BareShape"));
    assert!(minimal_render.contains("result: BareResult"));
    assert!(!minimal_render.contains("inputs:"));
    assert!(!minimal_render.contains("default output:"));
    assert!(!minimal_render.contains("constraints:"));
    assert!(!minimal_render.contains("parameters:"));

    let focused_render = render_catalog_text(&CatalogCommandReport {
        operations: vec![CatalogOperationReport {
            operation_id: htmlcut_core::OperationId::SelectExtract,
            command: Some("select".to_owned()),
            availability: CatalogAvailability::Cli,
            summary: "Focused".to_owned(),
            core_surface: "FocusedCoreSurface".to_owned(),
            request_contract: CatalogContractSurface {
                rust_shape: "FocusedRequest".to_owned(),
                schema_refs: Vec::new(),
            },
            result_contract: CatalogContractSurface {
                rust_shape: "FocusedResult".to_owned(),
                schema_refs: Vec::new(),
            },
            command_contract: Some(CatalogCommandContract {
                invocation: "htmlcut select <INPUT>".to_owned(),
                inputs: vec!["file".to_owned(), "url".to_owned()],
                default_match: None,
                selection_modes: Vec::new(),
                default_value: None,
                value_modes: Vec::new(),
                default_output: Some("text".to_owned()),
                default_output_overrides: Vec::new(),
                output_modes: Vec::new(),
                constraints: vec![CatalogConstraint::RequiresParameter {
                    parameter: "--thing".to_owned(),
                    when: CatalogCondition {
                        parameter: "--mode".to_owned(),
                        values: Vec::new(),
                    },
                }],
                notes: Vec::new(),
                examples: Vec::new(),
                parameters: vec![
                    CatalogParameterSpec {
                        section: "Synthetic".to_owned(),
                        name: "--flag".to_owned(),
                        kind: CatalogParameterKind::Flag,
                        requirement: CatalogParameterRequirement::Optional,
                        requirement_note: None,
                        value_hint: Some("IGNORED".to_owned()),
                        default: None,
                        allowed_values: Vec::new(),
                        summary: "Synthetic flag.".to_owned(),
                    },
                    CatalogParameterSpec {
                        section: "Synthetic".to_owned(),
                        name: "--conditional".to_owned(),
                        kind: CatalogParameterKind::Option,
                        requirement: CatalogParameterRequirement::Conditional,
                        requirement_note: None,
                        value_hint: Some("VALUE".to_owned()),
                        default: None,
                        allowed_values: Vec::new(),
                        summary: "Synthetic conditional.".to_owned(),
                    },
                ],
            }),
        }],
        ..minimal_report
    });
    assert!(focused_render.contains("inputs: file | url"));
    assert!(focused_render.contains("default output: text"));
    assert!(focused_render.contains("requires --thing when --mode"));
    assert!(focused_render.contains("flag --flag | optional"));
    assert!(
        focused_render.contains("option --conditional <VALUE> | conditional (see command notes)")
    );

    let empty_schema_report = SchemaCommandReport {
        tool: TOOL_NAME.to_owned(),
        version: HTMLCUT_VERSION.to_owned(),
        schema_name: SCHEMA_COMMAND_REPORT_SCHEMA_NAME.to_owned(),
        schema_version: SCHEMA_COMMAND_REPORT_SCHEMA_VERSION,
        schema_profile: htmlcut_core::HTMLCUT_JSON_SCHEMA_PROFILE.to_owned(),
        description: HTMLCUT_DESCRIPTION.to_owned(),
        command: "schema".to_owned(),
        schemas: Vec::new(),
    };
    let empty_schema_text = render_schema_text(&empty_schema_report);
    assert!(!empty_schema_text.contains("Schema:"));
    assert!(!empty_schema_text.contains("Schemas:"));
    assert!(empty_schema_text.contains("Schema profile:"));
}

#[test]
fn preview_helpers_cover_metadata_mismatches_and_empty_reports() {
    let empty_preview = build_extraction_report(
        "inspect-select",
        fixture_result(
            serde_json::json!({"tagName":"article"}),
            ValueType::Structured,
        ),
        None,
    );
    let mut empty_preview = empty_preview;
    empty_preview.matches.clear();
    empty_preview.diagnostics.clear();
    let empty_preview_text = render_preview_text(&empty_preview);
    assert!(!empty_preview_text.contains("Diagnostics:"));
    assert!(!empty_preview_text.contains("Matches:"));

    let select_preview_with_slice_metadata = render_preview_match_lines(
        htmlcut_core::OperationId::SelectPreview,
        &ExtractionMatch {
            index: 1,
            path: Some("explicit-path".to_owned()),
            value_type: ValueType::Structured,
            value: serde_json::json!({}),
            html: None,
            text: None,
            preview: "fallback select preview".to_owned(),
            metadata: delimiter_metadata(1, 1, (1, 3), (1, 3), (1, 3), false, false),
        },
    );
    assert_eq!(select_preview_with_slice_metadata[0], "1. explicit-path");
    assert!(
        select_preview_with_slice_metadata
            .iter()
            .any(|line| line == "   preview: fallback select preview")
    );
    assert!(
        select_preview_with_slice_metadata
            .iter()
            .all(|line| !line.contains("tag:"))
    );

    let slice_preview_with_selector_metadata = render_preview_match_lines(
        htmlcut_core::OperationId::SlicePreview,
        &ExtractionMatch {
            index: 2,
            path: Some("slice-path".to_owned()),
            value_type: ValueType::Structured,
            value: serde_json::json!({}),
            html: Some("same".to_owned()),
            text: Some("same".to_owned()),
            preview: "unused".to_owned(),
            metadata: selector_metadata(1, 1, "article:nth-of-type(1)", "article", &[]),
        },
    );
    assert_eq!(slice_preview_with_selector_metadata[0], "2. slice-path");
    assert!(
        slice_preview_with_selector_metadata
            .iter()
            .any(|line| line == "   text: same")
    );
    assert!(
        slice_preview_with_selector_metadata
            .iter()
            .all(|line| !line.contains("candidate index:"))
    );
    assert!(
        slice_preview_with_selector_metadata
            .iter()
            .all(|line| !line.contains("fragment:"))
    );
}

#[test]
fn schema_execution_and_prepare_helpers_cover_remaining_branches() {
    let catalog_report = build_catalog_report(None).expect("full catalog");
    assert!(
        catalog_report
            .operations
            .iter()
            .any(|operation| operation.availability == CatalogAvailability::Cli)
    );
    assert!(
        catalog_report
            .operations
            .iter()
            .any(|operation| operation.availability == CatalogAvailability::CoreOnly)
    );

    let text_outcome = run_schema(SchemaArgs {
        output: CliSchemaOutputMode::Text,
        output_file: None,
        name: Some("htmlcut.result".to_owned()),
        schema_version: Some(1),
    });
    assert_eq!(text_outcome.exit_code, 0);
    assert!(
        text_outcome
            .stdout
            .as_deref()
            .is_some_and(|stdout| stdout.contains("Schema:"))
    );

    let json_error_outcome = run_schema(SchemaArgs {
        output: CliSchemaOutputMode::Json,
        output_file: None,
        name: Some("synthetic.missing".to_owned()),
        schema_version: Some(99),
    });
    assert_eq!(json_error_outcome.exit_code, EXIT_CODE_USAGE);
    assert!(
        json_error_outcome
            .stdout
            .as_deref()
            .is_some_and(|stdout| stdout.contains("\"code\": \"CLI_SCHEMA_UNKNOWN\""))
    );

    let text_error_outcome = run_schema(SchemaArgs {
        output: CliSchemaOutputMode::Text,
        output_file: None,
        name: None,
        schema_version: Some(1),
    });
    assert_eq!(text_error_outcome.exit_code, EXIT_CODE_USAGE);
    assert!(
        text_error_outcome
            .stderr
            .iter()
            .any(|line| line.contains("`--schema-version` requires `--name`."))
    );

    let source = build_source_request(&SourceArgs {
        input: Some("https://example.com/docs/page.html".to_owned()),
        base_url: Some("https://base.example/root/".to_owned()),
        max_bytes: DEFAULT_MAX_BYTES.to_string(),
        fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
        fetch_preflight: CliFetchPreflightMode::HeadFirst,
    })
    .expect("url source request");
    assert!(matches!(
        source.input,
        htmlcut_core::SourceInput::Url { .. }
    ));
    assert_eq!(
        source.base_url.as_ref().map(ToString::to_string).as_deref(),
        Some("https://base.example/root/")
    );
    let http_source = build_source_request(&SourceArgs {
        input: Some("http://example.com/docs/page.html".to_owned()),
        base_url: None,
        max_bytes: DEFAULT_MAX_BYTES.to_string(),
        fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
        fetch_preflight: CliFetchPreflightMode::HeadFirst,
    })
    .expect("http url source request");
    assert!(matches!(
        http_source.input,
        htmlcut_core::SourceInput::Url { .. }
    ));

    let invalid_base_url = build_source_request(&SourceArgs {
        input: Some("-".to_owned()),
        base_url: Some("ftp://example.com".to_owned()),
        max_bytes: DEFAULT_MAX_BYTES.to_string(),
        fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
        fetch_preflight: CliFetchPreflightMode::HeadFirst,
    })
    .expect_err("invalid base url");
    assert_eq!(invalid_base_url.code, "CLI_BASE_URL_SCHEME_INVALID");

    assert!(
        !build_schema_report(None, None)
            .expect("full schema catalog")
            .schemas
            .is_empty()
    );
    assert_eq!(
        build_schema_report(Some("htmlcut.result"), Some(1))
            .expect("filtered schema")
            .schemas
            .len(),
        1
    );
    assert_eq!(
        build_schema_report(Some("synthetic.missing"), None)
            .expect_err("missing schema by name")
            .code,
        "CLI_SCHEMA_UNKNOWN"
    );
    assert_eq!(
        build_schema_report(Some("synthetic.missing"), Some(99))
            .expect_err("missing schema by name and version")
            .code,
        "CLI_SCHEMA_UNKNOWN"
    );

    assert_eq!(
        resolve_selection_spec(&SelectionArgs {
            r#match: CliMatchMode::All,
            index: None,
        })
        .expect("all selection"),
        SelectionSpec::All
    );
    assert_eq!(
        resolve_selection_spec(&SelectionArgs {
            r#match: CliMatchMode::Single,
            index: Some(1),
        })
        .expect_err("single index conflict")
        .code,
        "CLI_MATCH_INDEX_CONFLICT"
    );
    assert_eq!(
        resolve_selection_spec(&SelectionArgs {
            r#match: CliMatchMode::All,
            index: Some(1),
        })
        .expect_err("all index conflict")
        .code,
        "CLI_MATCH_INDEX_CONFLICT"
    );
    assert_eq!(
        resolve_selection_spec(&SelectionArgs {
            r#match: CliMatchMode::Nth,
            index: Some(0),
        })
        .expect_err("zero index invalid")
        .code,
        "CLI_MATCH_INDEX_INVALID"
    );
}

#[test]
fn raw_args_prefers_json_tracks_output_and_inspect_modes() {
    assert!(raw_args_prefers_json(&[
        "htmlcut".to_owned(),
        "inspect".to_owned(),
        "source".to_owned(),
        "page.html".to_owned(),
    ]));
    assert!(raw_args_prefers_json(&[
        "htmlcut".to_owned(),
        "select".to_owned(),
        "page.html".to_owned(),
        "--value".to_owned(),
        "structured".to_owned(),
    ]));
    assert!(!raw_args_prefers_json(&[
        "htmlcut".to_owned(),
        "inspect".to_owned(),
        "source".to_owned(),
        "page.html".to_owned(),
        "--output".to_owned(),
        "text".to_owned(),
    ]));
}

#[test]
fn raw_arg_helpers_detect_global_help_and_version_anywhere() {
    assert!(raw_args_requests_version(&[
        "htmlcut".to_owned(),
        "select".to_owned(),
        "--version".to_owned(),
    ]));
    assert!(raw_args_requests_version(&[
        "htmlcut".to_owned(),
        "inspect".to_owned(),
        "source".to_owned(),
        "-V".to_owned(),
    ]));
    assert!(raw_args_requests_help(&[
        "htmlcut".to_owned(),
        "slice".to_owned(),
        "--help".to_owned(),
    ]));
    assert!(!raw_args_requests_help(&[
        "htmlcut".to_owned(),
        "catalog".to_owned(),
    ]));
    assert!(!raw_args_requests_version(&[
        "htmlcut".to_owned(),
        "--".to_owned(),
        "--version".to_owned(),
    ]));
}

#[test]
fn command_name_from_raw_args_recognizes_nested_commands() {
    assert_eq!(
        command_name_from_raw_args(&[
            "htmlcut".to_owned(),
            "inspect".to_owned(),
            "source".to_owned(),
        ]),
        "inspect-source"
    );
    assert_eq!(
        command_name_from_raw_args(&[
            "htmlcut".to_owned(),
            "inspect".to_owned(),
            "select".to_owned(),
        ]),
        "inspect-select"
    );
    assert_eq!(
        command_name_from_raw_args(&[
            "htmlcut".to_owned(),
            "inspect".to_owned(),
            "slice".to_owned(),
        ]),
        "inspect-slice"
    );
    assert_eq!(
        command_name_from_raw_args(&["htmlcut".to_owned(), "select".to_owned()]),
        "select"
    );
    assert_eq!(
        command_name_from_raw_args(&["htmlcut".to_owned(), "slice".to_owned()]),
        "slice"
    );
}

#[test]
fn resolve_selection_spec_validates_index_rules() {
    assert_eq!(
        resolve_selection_spec(&SelectionArgs {
            r#match: CliMatchMode::Single,
            index: None,
        })
        .expect("selection"),
        SelectionSpec::single()
    );
    assert!(
        resolve_selection_spec(&SelectionArgs {
            r#match: CliMatchMode::Nth,
            index: None,
        })
        .is_err()
    );
    assert!(
        resolve_selection_spec(&SelectionArgs {
            r#match: CliMatchMode::First,
            index: Some(1),
        })
        .is_err()
    );
    assert_eq!(
        resolve_selection_spec(&SelectionArgs {
            r#match: CliMatchMode::Nth,
            index: Some(2),
        })
        .expect("selection")
        .index()
        .map(NonZeroUsize::get),
        Some(2usize)
    );
}

#[test]
fn resolve_value_spec_validates_attribute_usage() {
    assert!(resolve_value_spec(CliValueMode::Attribute, None).is_err());
    assert!(resolve_value_spec(CliValueMode::Text, Some("href".to_owned())).is_err());
    assert_eq!(
        resolve_value_spec(CliValueMode::Attribute, Some("href".to_owned()))
            .expect("attribute value")
            .attribute_name()
            .map(|name| name.as_str()),
        Some("href")
    );
    assert_eq!(
        resolve_value_spec(CliValueMode::Text, None)
            .expect("text value")
            .value_type(),
        ValueType::Text
    );
    assert_eq!(
        resolve_value_spec(CliValueMode::Structured, None)
            .expect("value")
            .value_type(),
        ValueType::Structured
    );
}

#[test]
fn resolve_extract_output_mode_enforces_value_and_bundle_rules() {
    assert!(resolve_extract_output_mode(None, &ValueType::Text, None).is_ok());
    assert_eq!(
        resolve_extract_output_mode(
            Some(CliOutputMode::None),
            &ValueType::Text,
            Some(Path::new("/tmp/bundle"))
        )
        .expect("none with bundle"),
        CliOutputMode::None
    );
    assert_eq!(
        resolve_extract_output_mode(Some(CliOutputMode::Html), &ValueType::InnerHtml, None)
            .expect("html for html"),
        CliOutputMode::Html
    );
    assert_eq!(
        resolve_extract_output_mode(Some(CliOutputMode::Html), &ValueType::OuterHtml, None)
            .expect("html for outer html"),
        CliOutputMode::Html
    );
    assert_eq!(
        resolve_extract_output_mode(Some(CliOutputMode::Json), &ValueType::Structured, None)
            .expect("structured json"),
        CliOutputMode::Json
    );
    assert_eq!(
        resolve_extract_output_mode(
            Some(CliOutputMode::None),
            &ValueType::Structured,
            Some(Path::new("/tmp/bundle"))
        )
        .expect("structured none"),
        CliOutputMode::None
    );
    assert!(
        resolve_extract_output_mode(Some(CliOutputMode::None), &ValueType::Text, None).is_err()
    );
    assert!(
        resolve_extract_output_mode(
            Some(CliOutputMode::Html),
            &ValueType::Text,
            Some(Path::new("/tmp/bundle"))
        )
        .is_err()
    );
    assert!(
        resolve_extract_output_mode(
            Some(CliOutputMode::Text),
            &ValueType::Structured,
            Some(Path::new("/tmp/bundle"))
        )
        .is_err()
    );
}

#[test]
fn resolve_regex_flags_rejects_literal_mode_overrides() {
    assert_eq!(
        resolve_regex_flags(CliPatternMode::Regex, Some("us".to_owned())).expect("flags"),
        Some("us".to_owned())
    );
    assert_eq!(
        resolve_regex_flags(CliPatternMode::Regex, None).expect("default regex flags"),
        Some(DEFAULT_REGEX_FLAGS.to_owned())
    );
    assert!(resolve_regex_flags(CliPatternMode::Literal, Some("u".to_owned())).is_err());
    assert_eq!(
        resolve_regex_flags(CliPatternMode::Literal, None).expect("flags"),
        None
    );
}

#[test]
fn extract_prefers_json_matches_default_structured_behavior() {
    assert!(extract_prefers_json(&ExtractOutputArgs {
        value: CliValueMode::Structured,
        attribute: None,
        whitespace: CliWhitespaceMode::Preserve,
        rewrite_urls: false,
        output: None,
        bundle: None,
        preview_chars: DEFAULT_PREVIEW_CHARS,
        include_source_text: false,
        output_file: None,
    }));
    assert!(!extract_prefers_json(&ExtractOutputArgs {
        value: CliValueMode::Text,
        attribute: None,
        whitespace: CliWhitespaceMode::Preserve,
        rewrite_urls: false,
        output: Some(CliOutputMode::Text),
        bundle: None,
        preview_chars: DEFAULT_PREVIEW_CHARS,
        include_source_text: false,
        output_file: None,
    }));
}

#[test]
fn bundle_document_title_prefers_core_and_then_falls_back() {
    let titled_report = build_extraction_report(
        "select",
        fixture_result(Value::String("Hello".to_owned()), ValueType::Text),
        None,
    );
    assert_eq!(bundle_document_title(&titled_report), "Fixture");

    let mut fallback_host = build_extraction_report(
        "select",
        fixture_result(Value::String("Hello".to_owned()), ValueType::Text),
        None,
    );
    fallback_host.document_title = None;
    fallback_host.source.effective_base_url =
        Some("https://example.net/docs/start.html".to_owned());
    assert_eq!(bundle_document_title(&fallback_host), "example.net");

    let mut fallback_path = build_extraction_report(
        "select",
        fixture_result(Value::String("Hello".to_owned()), ValueType::Text),
        None,
    );
    fallback_path.document_title = None;
    fallback_path.source.input_base_url = None;
    fallback_path.source.effective_base_url = None;
    fallback_path.source.value = "/tmp/sample name.html".to_owned();
    assert_eq!(bundle_document_title(&fallback_path), "sample name");

    let mut invalid_url = build_extraction_report(
        "select",
        fixture_result(Value::String("Hello".to_owned()), ValueType::Text),
        None,
    );
    invalid_url.document_title = None;
    invalid_url.source.effective_base_url = Some("not a url".to_owned());
    invalid_url.source.value = "/tmp/sample name.html".to_owned();
    assert_eq!(bundle_document_title(&invalid_url), "sample name");
}

#[test]
fn render_output_helpers_cover_text_html_json_and_none() {
    let text_report = build_extraction_report(
        "select",
        fixture_result(Value::String("Hello".to_owned()), ValueType::Text),
        None,
    );
    assert_eq!(
        render_extraction_output(&text_report, CliOutputMode::Text).expect("text output"),
        "Hello"
    );

    let html_report = build_extraction_report(
        "select",
        fixture_result(
            Value::String("<p>Hello</p>".to_owned()),
            ValueType::InnerHtml,
        ),
        None,
    );
    assert!(
        render_extraction_output(&html_report, CliOutputMode::Html)
            .expect("html output")
            .contains("<p>Hello</p>")
    );
    assert!(
        render_extraction_output(&text_report, CliOutputMode::Json)
            .expect("json output")
            .contains("\"command\": \"select\"")
    );
    assert!(render_extraction_output(&text_report, CliOutputMode::None).is_none());
}

#[test]
fn render_preview_and_source_inspection_text_are_human_readable() {
    let mut preview = build_extraction_report(
        "inspect-select",
        fixture_result(
            serde_json::json!({"tagName":"article"}),
            ValueType::Structured,
        ),
        None,
    );
    preview.operation_id = htmlcut_core::OperationId::SelectPreview;
    let preview_text = render_preview_text(&preview);
    assert!(preview_text.contains("Command: inspect-select"));
    assert!(preview_text.contains("Matches:"));
    assert!(preview_text.contains("tag: article"));
    assert!(preview_text.contains("text: Hello"));

    let mut slice_preview = build_extraction_report(
        "inspect-slice",
        fixture_result(
            serde_json::json!({"range":{"start":1,"end":18}}),
            ValueType::Structured,
        ),
        None,
    );
    slice_preview.operation_id = htmlcut_core::OperationId::SlicePreview;
    slice_preview.matches[0].path = None;
    slice_preview.matches[0].html = Some("<article>Hello</article>".to_owned());
    slice_preview.matches[0].text = Some("Hello".to_owned());
    slice_preview.matches[0].metadata =
        delimiter_metadata(1, 1, (1, 24), (10, 15), (1, 24), true, true);
    let slice_preview_text = render_preview_text(&slice_preview);
    assert!(slice_preview_text.contains("fragment: <article>Hello</article>"));
    assert!(slice_preview_text.contains("text: Hello"));
    assert!(slice_preview_text.contains("include start: true"));

    let inspection_text =
        render_source_inspection_text(&fixture_inspection(), DEFAULT_PREVIEW_CHARS);
    assert!(inspection_text.contains("Top tags: a (2)"));
    assert!(inspection_text.contains("Link previews:"));
    assert!(inspection_text.contains("Document <base href>: ../content/"));

    let mut untitled = fixture_inspection();
    untitled.source.input_base_url = None;
    untitled.source.effective_base_url = None;
    let document = untitled.document.as_mut().expect("document");
    document.title = None;
    document.document_base_href = None;
    document.top_tags.clear();
    document.top_classes.clear();
    document.headings.clear();
    document.links.clear();
    let untitled_text = render_source_inspection_text(&untitled, DEFAULT_PREVIEW_CHARS);
    assert!(!untitled_text.contains("Input base URL:"));
    assert!(!untitled_text.contains("Effective base URL:"));
    assert!(!untitled_text.contains("Title:"));
    assert!(!untitled_text.contains("Document <base href>:"));
    assert!(!untitled_text.contains("Top tags:"));
    assert!(!untitled_text.contains("Top classes:"));
    assert!(!untitled_text.contains("Headings:"));
    assert!(!untitled_text.contains("Link previews:"));
}

#[test]
fn wrap_html_document_and_match_renderers_cover_remaining_paths() {
    let report = build_extraction_report(
        "select",
        fixture_result(
            Value::String("<!DOCTYPE html><html><body>Hello</body></html>".to_owned()),
            ValueType::OuterHtml,
        ),
        None,
    );
    assert!(wrap_html_document(&report).starts_with("<!DOCTYPE html>"));

    let json_match = ExtractionMatch {
        index: 1,
        path: None,
        value_type: ValueType::Structured,
        value: serde_json::json!({"hello":"world"}),
        html: None,
        text: None,
        preview: "preview".to_owned(),
        metadata: selector_metadata(1, 1, "article:nth-of-type(1)", "article", &[]),
    };
    assert!(render_match_as_text(&json_match).contains("\"hello\""));
    assert!(render_match_as_html(&json_match).contains("<pre>"));

    let text_match = ExtractionMatch {
        index: 1,
        path: None,
        value_type: ValueType::Text,
        value: Value::String("Hello".to_owned()),
        html: Some("<article>Hello</article>".to_owned()),
        text: Some("Hello".to_owned()),
        preview: "Hello".to_owned(),
        metadata: selector_metadata(1, 1, "article:nth-of-type(1)", "article", &[]),
    };
    assert_eq!(
        render_match_as_html(&text_match),
        "<article>Hello</article>"
    );

    let wrapped = build_extraction_report(
        "select",
        fixture_result(
            Value::String("<p>Hello</p>".to_owned()),
            ValueType::InnerHtml,
        ),
        None,
    );
    assert!(wrap_html_document(&wrapped).contains("<section data-match-index=\"1\">"));
    assert!(!looks_like_document("<section>Hello</section>"));
}

#[test]
fn verbose_and_diagnostic_renderers_cover_branching_paths() {
    let report = build_extraction_report(
        "select",
        fixture_result(Value::String("Hello".to_owned()), ValueType::Text),
        Some(BundlePaths {
            dir: "/tmp/bundle".to_owned(),
            html: "/tmp/bundle/selection.html".to_owned(),
            text: "/tmp/bundle/selection.txt".to_owned(),
            report: "/tmp/bundle/report.json".to_owned(),
        }),
    );
    let verbose = build_verbose_lines(&report, 2);
    assert!(verbose[0].contains("selected 1 match"));
    assert!(verbose[1].contains("scanned 2 candidates"));
    assert!(build_verbose_lines(&report, 0).is_empty());
    assert_eq!(build_verbose_lines(&report, 1).len(), 1);
    let warning_stderr = build_human_diagnostic_stderr_lines(&[Diagnostic {
        level: DiagnosticLevel::Warning,
        code: "EFFECTIVE_BASE_URL_UNRESOLVED".to_owned(),
        message: "warning".to_owned(),
        details: None,
    }]);
    assert_eq!(warning_stderr.len(), 1);
    assert!(warning_stderr[0].contains("htmlcut: warning EFFECTIVE_BASE_URL_UNRESOLVED"));
    assert_eq!(render_diagnostic_level(DiagnosticLevel::Warning), "warning");
    assert_eq!(render_source_kind(&SourceKind::Url), "url");
}

#[test]
fn error_helpers_and_outcomes_cover_json_and_human_modes() {
    let error = usage_error("CLI_USAGE", "bad input");
    assert_eq!(exit_code_for_error(&error), EXIT_CODE_USAGE);
    let generated_diagnostics = json_error_diagnostics(&error);
    assert_eq!(generated_diagnostics.len(), 1);
    assert_eq!(generated_diagnostics[0].code, "CLI_USAGE");

    let json = error_outcome("select".to_owned(), true, None, error);
    assert_eq!(json.exit_code, EXIT_CODE_USAGE);
    assert!(json.stdout.expect("json stdout").contains("\"ok\": false"));

    let human = error_outcome(
        "select".to_owned(),
        false,
        None,
        output_error("CLI_OUTPUT", "could not write"),
    );
    assert!(human.stderr[0].contains("could not write"));

    let json_with_diagnostics = error_outcome(
        "select".to_owned(),
        true,
        None,
        usage_error_with_diagnostics(
            "CLI_USAGE",
            "bad input",
            vec![Diagnostic {
                level: DiagnosticLevel::Error,
                code: "CLI_USAGE".to_owned(),
                message: "bad input".to_owned(),
                details: None,
            }],
        ),
    );
    let existing_diagnostics = json_error_diagnostics(&usage_error_with_diagnostics(
        "CLI_USAGE",
        "bad input",
        vec![Diagnostic {
            level: DiagnosticLevel::Error,
            code: "CLI_USAGE".to_owned(),
            message: "bad input".to_owned(),
            details: None,
        }],
    ));
    assert_eq!(existing_diagnostics.len(), 1);
    assert!(
        json_with_diagnostics
            .stdout
            .expect("json stdout")
            .contains("\"diagnostics\"")
    );
    let direct_json = json_error_outcome(
        "select".to_owned(),
        None,
        usage_error("CLI_USAGE", "bad input"),
    );
    assert_eq!(direct_json.exit_code, EXIT_CODE_USAGE);
    assert!(
        direct_json
            .stdout
            .expect("json stdout")
            .contains("\"error\"")
    );
    let direct_human = human_error_outcome(output_error("CLI_OUTPUT", "could not write"));
    assert_eq!(direct_human.exit_code, EXIT_CODE_OUTPUT);
    assert!(direct_human.stderr[0].contains("could not write"));

    let core_error = primary_extraction_error(&[Diagnostic {
        level: DiagnosticLevel::Error,
        code: "NO_MATCH".to_owned(),
        message: "No matches".to_owned(),
        details: None,
    }]);
    assert_eq!(exit_code_for_error(&core_error), EXIT_CODE_EXTRACTION);
}

#[test]
fn clap_error_message_prefers_the_primary_error_line() {
    let error =
        Cli::try_parse_from(["htmlcut", "select", "page.html"]).expect_err("parse error expected");
    assert!(clap_error_message(&error).contains("required arguments"));

    let help = Cli::try_parse_from(["htmlcut", "--help"]).expect_err("help expected");
    assert!(clap_error_message(&help).contains("Usage: htmlcut [OPTIONS] <COMMAND>"));
}

#[test]
fn global_verbose_parses_before_or_after_subcommand() {
    let before = Cli::try_parse_from(["htmlcut", "-vv", "select", "page.html", "--css", "article"])
        .expect("parse");
    assert_eq!(before.global.verbose, 2);

    let after = Cli::try_parse_from(["htmlcut", "select", "-vv", "page.html", "--css", "article"])
        .expect("parse");
    assert_eq!(after.global.verbose, 2);
}

#[test]
fn cargo_manifest_drives_the_public_metadata_constants() {
    let workspace_manifest = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("crate dir")
            .parent()
            .expect("workspace root")
            .join("Cargo.toml"),
    )
    .expect("workspace manifest");
    let workspace_version =
        workspace_package_field(&workspace_manifest, "version").expect("workspace version");
    let workspace_description =
        workspace_package_field(&workspace_manifest, "description").expect("workspace description");

    assert_eq!(HTMLCUT_VERSION, workspace_version);
    assert_eq!(HTMLCUT_DESCRIPTION, workspace_description);
}

#[test]
fn run_covers_root_help_help_version_and_parse_error_modes() {
    let (exit_code, stdout, stderr) = run_vec(vec!["htmlcut".to_owned()]);
    assert_eq!(exit_code, 0);
    assert!(stdout.contains("Usage: htmlcut [OPTIONS] <COMMAND>"));
    assert!(stderr.is_empty());

    let (exit_code, stdout, _) = run_vec(vec!["htmlcut".to_owned(), "--help".to_owned()]);
    assert_eq!(exit_code, 0);
    assert!(stdout.contains("inspect"));

    let (exit_code, stdout, _) = run_vec(vec!["htmlcut".to_owned(), "--version".to_owned()]);
    assert_eq!(exit_code, 0);
    assert_eq!(stdout, format!("{}\n", version_banner()));

    let (exit_code, stdout, _) = run_vec(vec![
        "htmlcut".to_owned(),
        "select".to_owned(),
        "--version".to_owned(),
    ]);
    assert_eq!(exit_code, 0);
    assert_eq!(stdout, format!("{}\n", version_banner()));

    let (exit_code, stdout, _) = run_vec(vec![
        "htmlcut".to_owned(),
        "inspect".to_owned(),
        "source".to_owned(),
        "-V".to_owned(),
    ]);
    assert_eq!(exit_code, 0);
    assert_eq!(stdout, format!("{}\n", version_banner()));

    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "select".to_owned(),
        "--version".to_owned(),
        "--help".to_owned(),
    ]);
    assert_eq!(exit_code, 0);
    assert!(stdout.contains("Usage: htmlcut select [OPTIONS] [INPUT]"));
    assert!(stdout.contains("-V, --version"));
    assert!(stderr.is_empty());

    let (exit_code, _, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "select".to_owned(),
        "page.html".to_owned(),
        "--bogus".to_owned(),
    ]);
    assert_eq!(exit_code, EXIT_CODE_USAGE);
    assert!(stderr.contains("unexpected argument '--bogus'"));

    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "select".to_owned(),
        "page.html".to_owned(),
        "--output".to_owned(),
        "json".to_owned(),
        "--bogus".to_owned(),
    ]);
    assert_eq!(exit_code, EXIT_CODE_USAGE);
    assert!(stdout.contains("\"category\": \"usage\""));
    assert!(stdout.contains("\"command\": \"select\""));
    assert!(stderr.is_empty());
}

#[test]
fn catalog_report_and_text_surface_core_operation_catalog() {
    let report = build_catalog_report(None).expect("catalog report");
    assert_eq!(report.tool, TOOL_NAME);
    assert_eq!(report.version, HTMLCUT_VERSION);
    assert_eq!(report.schema_name, CATALOG_REPORT_SCHEMA_NAME);
    assert_eq!(report.schema_version, crate::model::CATALOG_SCHEMA_VERSION);
    assert_eq!(
        report.schema_profile,
        htmlcut_core::HTMLCUT_JSON_SCHEMA_PROFILE
    );
    assert_eq!(report.description, HTMLCUT_DESCRIPTION);
    assert_eq!(report.command, "catalog");
    assert_eq!(
        report.operations.len(),
        htmlcut_core::operation_catalog().len()
    );
    assert_eq!(
        report.operations[0].operation_id,
        htmlcut_core::operation_catalog()[0].id
    );

    let text = render_catalog_text(&report);
    assert!(text.contains("Operations:"));
    assert!(text.contains("source.inspect | inspect source"));
    assert!(text.contains("document.parse | core only"));

    let filtered = build_catalog_report(Some("select.preview")).expect("filtered catalog");
    assert_eq!(filtered.operations.len(), 1);
    assert_eq!(
        filtered.operations[0].operation_id,
        htmlcut_core::OperationId::SelectPreview
    );
    assert_eq!(
        filtered.operations[0].core_surface,
        "preview_extraction(ExtractionRequest{kind=selector}, RuntimeOptions)"
    );
    assert_eq!(
        filtered.operations[0].request_contract.rust_shape,
        "ExtractionRequest + RuntimeOptions"
    );
    assert_eq!(
        filtered.operations[0].request_contract.schema_refs,
        vec![
            SchemaRefReport {
                schema_name: htmlcut_core::EXTRACTION_REQUEST_SCHEMA_NAME.to_owned(),
                schema_version: htmlcut_core::CORE_REQUEST_SCHEMA_VERSION,
            },
            SchemaRefReport {
                schema_name: htmlcut_core::RUNTIME_OPTIONS_SCHEMA_NAME.to_owned(),
                schema_version: htmlcut_core::CORE_REQUEST_SCHEMA_VERSION,
            },
        ]
    );
    assert_eq!(
        filtered.operations[0].result_contract.rust_shape,
        "ExtractionResult"
    );
    assert_eq!(
        filtered.operations[0].result_contract.schema_refs,
        vec![SchemaRefReport {
            schema_name: htmlcut_core::CORE_RESULT_SCHEMA_NAME.to_owned(),
            schema_version: htmlcut_core::CORE_RESULT_SCHEMA_VERSION,
        }]
    );
    let contract = filtered.operations[0]
        .command_contract
        .as_ref()
        .expect("filtered cli operation should expose a contract");
    assert_eq!(
        contract.invocation,
        "htmlcut inspect select [OPTIONS] --css <CSS> [INPUT]"
    );
    assert_eq!(contract.default_match.as_deref(), Some("first"));
    assert_eq!(contract.default_value.as_deref(), Some("structured"));
    assert_eq!(contract.default_output.as_deref(), Some("json"));
    assert!(contract.parameters.iter().any(|parameter| {
        parameter.name == "--css"
            && parameter.kind == crate::model::CatalogParameterKind::Option
            && parameter.requirement == crate::model::CatalogParameterRequirement::Conditional
            && parameter.requirement_note.as_deref()
                == Some("required unless --request-file is used")
    }));
    assert!(contract.parameters.iter().any(|parameter| {
        parameter.name == "--request-file"
            && parameter.kind == crate::model::CatalogParameterKind::Option
            && parameter.requirement == crate::model::CatalogParameterRequirement::Optional
    }));
    assert!(contract.parameters.iter().any(|parameter| {
        parameter.name == "--index"
            && parameter.requirement == crate::model::CatalogParameterRequirement::Conditional
            && parameter.requirement_note.as_deref() == Some("required when --match nth is used")
    }));

    let error = build_catalog_report(Some("unknown.operation")).expect_err("unknown op");
    assert_eq!(error.code, "CLI_OPERATION_ID_UNKNOWN");
}

#[test]
fn schema_report_surfaces_core_cli_and_interop_contracts() {
    let report = build_schema_report(None, None).expect("schema report");
    assert_eq!(report.tool, TOOL_NAME);
    assert_eq!(report.version, HTMLCUT_VERSION);
    assert_eq!(report.schema_name, SCHEMA_COMMAND_REPORT_SCHEMA_NAME);
    assert_eq!(report.schema_version, SCHEMA_COMMAND_REPORT_SCHEMA_VERSION);
    assert_eq!(
        report.schema_profile,
        htmlcut_core::HTMLCUT_JSON_SCHEMA_PROFILE
    );
    assert!(report.schemas.iter().any(|schema| {
        schema.schema_name == htmlcut_core::EXTRACTION_REQUEST_SCHEMA_NAME
            && schema.schema_version == htmlcut_core::CORE_REQUEST_SCHEMA_VERSION
            && schema.owner_surface == "htmlcut-core"
    }));
    assert!(report.schemas.iter().any(|schema| {
        schema.schema_name == htmlcut_core::interop::v1::PLAN_SCHEMA_NAME
            && schema.owner_surface == "htmlcut_core::interop::v1"
            && schema.stability == htmlcut_core::SchemaStability::Frozen
    }));
    assert!(report.schemas.iter().any(|schema| {
        schema.schema_name == CATALOG_REPORT_SCHEMA_NAME && schema.owner_surface == "htmlcut-cli"
    }));

    let filtered = build_schema_report(Some("htmlcut.result"), Some(1)).expect("filtered schema");
    assert_eq!(filtered.schemas.len(), 1);
    assert_eq!(filtered.schemas[0].schema_name, "htmlcut.result");

    let error = build_schema_report(None, Some(1)).expect_err("version without name");
    assert_eq!(error.code, "CLI_SCHEMA_VERSION_REQUIRES_NAME");
}

#[test]
fn run_covers_inspection_text_failure_and_preview_modes() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture_file(
        tempdir.path(),
        "input.html",
        "<html><body><article><h1>Hello</h1><a href=\"/guide\">Guide</a></article></body></html>",
    );
    let input = input_path.to_string_lossy().into_owned();
    let missing = tempdir
        .path()
        .join("missing.html")
        .to_string_lossy()
        .into_owned();

    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "inspect".to_owned(),
        "source".to_owned(),
        input.clone(),
        "--output".to_owned(),
        "text".to_owned(),
    ]);
    assert_eq!(exit_code, 0);
    assert!(stdout.contains("Root tag: html"));
    assert!(stderr.is_empty());

    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "inspect".to_owned(),
        "source".to_owned(),
        input.clone(),
        "--base-url".to_owned(),
        "ftp://example.com".to_owned(),
    ]);
    assert_eq!(exit_code, EXIT_CODE_USAGE);
    assert!(stdout.contains("\"code\": \"CLI_BASE_URL_SCHEME_INVALID\""));
    assert!(stderr.is_empty());

    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "inspect".to_owned(),
        "source".to_owned(),
        missing.clone(),
        "--output".to_owned(),
        "json".to_owned(),
    ]);
    assert_eq!(exit_code, EXIT_CODE_SOURCE);
    assert!(stdout.contains("\"command\": \"inspect-source\""));
    assert!(stderr.is_empty());

    let (exit_code, _, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "inspect".to_owned(),
        "source".to_owned(),
        missing,
        "--output".to_owned(),
        "text".to_owned(),
    ]);
    assert_eq!(exit_code, EXIT_CODE_SOURCE);
    assert!(stderr.contains("Could not access file"));

    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "inspect".to_owned(),
        "select".to_owned(),
        input.clone(),
        "--css".to_owned(),
        "article".to_owned(),
        "--match".to_owned(),
        "nth".to_owned(),
    ]);
    assert_eq!(exit_code, EXIT_CODE_USAGE);
    assert!(stdout.contains("\"code\": \"CLI_MATCH_INDEX_REQUIRED\""));
    assert!(stderr.is_empty());

    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "inspect".to_owned(),
        "slice".to_owned(),
        input.clone(),
        "--from".to_owned(),
        "<article>".to_owned(),
        "--to".to_owned(),
        "</article>".to_owned(),
        "--regex-flags".to_owned(),
        "u".to_owned(),
    ]);
    assert_eq!(exit_code, EXIT_CODE_USAGE);
    assert!(stdout.contains("\"code\": \"CLI_REGEX_FLAGS_CONFLICT\""));
    assert!(stderr.is_empty());

    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "inspect".to_owned(),
        "select".to_owned(),
        input,
        "--css".to_owned(),
        "article".to_owned(),
        "--output".to_owned(),
        "text".to_owned(),
    ]);
    assert_eq!(exit_code, 0);
    assert!(stdout.contains("Command: inspect-select"));
    assert!(stderr.is_empty());
}

#[test]
fn run_covers_extraction_error_json_and_bundle_failure_modes() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture_file(
        tempdir.path(),
        "input.html",
        "<article><p>Hello</p></article>",
    );
    let input = input_path.to_string_lossy().into_owned();
    let bundle_path = tempdir.path().join("not-a-dir");
    fs::write(&bundle_path, "file").expect("bundle sentinel");

    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "select".to_owned(),
        input.clone(),
        "--css".to_owned(),
        "[".to_owned(),
        "--output".to_owned(),
        "json".to_owned(),
    ]);
    assert_eq!(exit_code, EXIT_CODE_USAGE);
    assert!(stdout.contains("\"code\": \"INVALID_SELECTOR\""));
    assert!(stdout.contains("Invalid selector"));
    assert!(stderr.is_empty());

    let (exit_code, _, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "slice".to_owned(),
        input.clone(),
        "--from".to_owned(),
        "<article>".to_owned(),
        "--to".to_owned(),
        "</article>".to_owned(),
        "--regex-flags".to_owned(),
        "u".to_owned(),
    ]);
    assert_eq!(exit_code, EXIT_CODE_USAGE);
    assert!(stderr.contains("--regex-flags can only be used with --pattern regex."));

    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "select".to_owned(),
        input.clone(),
        "--css".to_owned(),
        "article".to_owned(),
        "--output".to_owned(),
        "json".to_owned(),
        "--bundle".to_owned(),
        bundle_path.to_string_lossy().into_owned(),
    ]);
    assert_eq!(exit_code, EXIT_CODE_OUTPUT);
    assert!(stdout.contains("\"category\": \"output\""));
    assert!(stderr.is_empty());

    let (exit_code, stdout, _) = run_vec(vec![
        "htmlcut".to_owned(),
        "inspect".to_owned(),
        "select".to_owned(),
        input.clone(),
        "--css".to_owned(),
        "[".to_owned(),
    ]);
    assert_eq!(exit_code, EXIT_CODE_USAGE);
    assert!(stdout.contains("\"command\": \"inspect-select\""));

    let (exit_code, _, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "inspect".to_owned(),
        "slice".to_owned(),
        input,
        "--from".to_owned(),
        "[".to_owned(),
        "--to".to_owned(),
        "]".to_owned(),
        "--pattern".to_owned(),
        "regex".to_owned(),
        "--output".to_owned(),
        "text".to_owned(),
    ]);
    assert_eq!(exit_code, EXIT_CODE_USAGE);
    assert!(stderr.contains("Invalid regular expression"));
}

#[test]
fn helper_branches_cover_remaining_rendering_validation_and_error_paths() {
    assert_eq!(
        default_output_for_value(&ValueType::InnerHtml),
        CliOutputMode::Html
    );
    assert_eq!(
        default_output_for_value(&ValueType::OuterHtml),
        CliOutputMode::Html
    );
    assert_eq!(
        validate_base_url(None).expect("missing base url is okay"),
        None
    );
    assert!(validate_base_url(Some("::not-a-url::")).is_err());
    assert!(validate_base_url(Some("ftp://example.com")).is_err());
    assert!(
        resolve_selection_spec(&SelectionArgs {
            r#match: CliMatchMode::Nth,
            index: Some(0),
        })
        .is_err()
    );
    assert_eq!(
        resolve_value_spec(CliValueMode::InnerHtml, None)
            .expect("html value")
            .value_type(),
        ValueType::InnerHtml
    );
    assert_eq!(
        resolve_value_spec(CliValueMode::OuterHtml, None)
            .expect("outer html value")
            .value_type(),
        ValueType::OuterHtml
    );

    let mut preview = build_extraction_report(
        "inspect-slice",
        fixture_result(
            serde_json::json!({"tagName":"article"}),
            ValueType::Structured,
        ),
        None,
    );
    preview.matches.clear();
    preview.diagnostics.push(Diagnostic {
        level: DiagnosticLevel::Info,
        code: "NOTE".to_owned(),
        message: "preview note".to_owned(),
        details: None,
    });
    let preview_text = render_preview_text(&preview);
    assert!(preview_text.contains("Diagnostics:"));
    assert!(!preview_text.contains("Matches:"));

    let mut empty_inspection = fixture_inspection();
    empty_inspection.document = None;
    empty_inspection.diagnostics.push(Diagnostic {
        level: DiagnosticLevel::Warning,
        code: "WARN".to_owned(),
        message: "watch out".to_owned(),
        details: None,
    });
    let inspection_text = render_source_inspection_text(&empty_inspection, DEFAULT_PREVIEW_CHARS);
    assert!(inspection_text.contains("Effective base URL:"));
    assert!(inspection_text.contains("Diagnostics:"));
    assert!(!inspection_text.contains("Headings:"));

    let mut link_variants = fixture_inspection();
    link_variants.document.as_mut().expect("document").links = vec![
        LinkInspection {
            text: "Docs".to_owned(),
            href: Some("https://example.com/docs".to_owned()),
            resolved_href: Some("https://example.com/docs".to_owned()),
            path: "a:nth-of-type(1)".to_owned(),
        },
        LinkInspection {
            text: "Bare".to_owned(),
            href: None,
            resolved_href: None,
            path: "a:nth-of-type(2)".to_owned(),
        },
    ];
    let link_text = render_source_inspection_text(&link_variants, DEFAULT_PREVIEW_CHARS);
    assert!(link_text.contains("- Docs [https://example.com/docs] [a:nth-of-type(1)]"));
    assert!(link_text.contains("- Bare [a:nth-of-type(2)]"));

    let mut plural_report = build_extraction_report(
        "select",
        fixture_result(Value::String("Hello".to_owned()), ValueType::Text),
        None,
    );
    plural_report.stats.match_count = 2;
    let verbose = build_verbose_lines(&plural_report, 2);
    assert!(verbose[0].contains("selected 2 matches"));
    assert_eq!(render_diagnostic_level(DiagnosticLevel::Error), "error");
    assert_eq!(render_diagnostic_level(DiagnosticLevel::Info), "info");
    assert_eq!(render_source_kind(&SourceKind::File), "file");
    assert_eq!(render_source_kind(&SourceKind::Stdin), "stdin");
    assert_eq!(render_source_kind(&SourceKind::Memory), "memory");

    let mut wrapped = build_extraction_report(
        "select",
        fixture_result(
            Value::String("<p>Hello</p>".to_owned()),
            ValueType::InnerHtml,
        ),
        None,
    );
    wrapped.document_title = None;
    wrapped.source.effective_base_url = Some("https://example.net/docs/start.html".to_owned());
    assert!(wrap_html_document(&wrapped).contains("<title>example.net</title>"));

    let source = primary_extraction_error(&[Diagnostic {
        level: DiagnosticLevel::Error,
        code: "SOURCE_LOAD_FAILED".to_owned(),
        message: "boom".to_owned(),
        details: None,
    }]);
    assert_eq!(exit_code_for_error(&source), EXIT_CODE_SOURCE);

    let usage = primary_extraction_error(&[Diagnostic {
        level: DiagnosticLevel::Error,
        code: "INVALID_REQUEST".to_owned(),
        message: "bad".to_owned(),
        details: None,
    }]);
    assert_eq!(exit_code_for_error(&usage), EXIT_CODE_USAGE);

    let extraction = primary_extraction_error(&[Diagnostic {
        level: DiagnosticLevel::Error,
        code: "AMBIGUOUS_MATCH".to_owned(),
        message: "too many".to_owned(),
        details: None,
    }]);
    assert_eq!(exit_code_for_error(&extraction), EXIT_CODE_EXTRACTION);

    let internal = primary_extraction_error(&[Diagnostic {
        level: DiagnosticLevel::Error,
        code: "SURPRISE".to_owned(),
        message: "unexpected".to_owned(),
        details: None,
    }]);
    assert_eq!(exit_code_for_error(&internal), EXIT_CODE_INTERNAL);
    assert_eq!(
        exit_code_for_error(&primary_extraction_error(&[])),
        EXIT_CODE_INTERNAL
    );

    let inspection_source = primary_source_inspection_error(&[Diagnostic {
        level: DiagnosticLevel::Error,
        code: "SOURCE_LOAD_FAILED".to_owned(),
        message: "missing".to_owned(),
        details: None,
    }]);
    assert_eq!(exit_code_for_error(&inspection_source), EXIT_CODE_SOURCE);
    assert_eq!(
        exit_code_for_error(&primary_source_inspection_error(&[Diagnostic {
            level: DiagnosticLevel::Error,
            code: "OTHER".to_owned(),
            message: "other".to_owned(),
            details: None,
        }])),
        EXIT_CODE_INTERNAL
    );
    assert_eq!(
        exit_code_for_error(&primary_source_inspection_error(&[])),
        EXIT_CODE_INTERNAL
    );

    assert_eq!(render_error_category(CliErrorCategory::Usage), "usage");
    assert_eq!(render_error_category(CliErrorCategory::Source), "source");
    assert_eq!(
        render_error_category(CliErrorCategory::Extraction),
        "extraction"
    );
    assert_eq!(render_error_category(CliErrorCategory::Output), "output");
    assert_eq!(
        render_error_category(CliErrorCategory::Internal),
        "internal"
    );

    let human = error_outcome(
        "select".to_owned(),
        false,
        None,
        source_error("SRC", "could not load", Vec::new()),
    );
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit_code = write_outcome(human, &mut stdout, &mut stderr);
    assert_eq!(exit_code, EXIT_CODE_SOURCE);
    assert!(stdout.is_empty());
    assert!(
        String::from_utf8(stderr)
            .expect("stderr")
            .contains("could not load")
    );
}

#[test]
fn write_bundle_reports_each_output_failure() {
    let report = build_extraction_report(
        "select",
        fixture_result(Value::String("Hello".to_owned()), ValueType::Text),
        None,
    );

    let create_dir_temp = tempdir().expect("tempdir");
    let create_dir_path = create_dir_temp.path().join("bundle");
    fs::write(&create_dir_path, "file").expect("write file");
    assert_eq!(
        write_bundle(
            &report,
            &BundlePaths {
                dir: create_dir_path.to_string_lossy().into_owned(),
                html: create_dir_path
                    .join("selection.html")
                    .to_string_lossy()
                    .into_owned(),
                text: create_dir_path
                    .join("selection.txt")
                    .to_string_lossy()
                    .into_owned(),
                report: create_dir_path
                    .join("report.json")
                    .to_string_lossy()
                    .into_owned(),
            },
        )
        .expect_err("directory creation should fail")
        .code,
        "CLI_BUNDLE_DIRECTORY_CREATE_FAILED"
    );

    let html_temp = tempdir().expect("tempdir");
    fs::create_dir(html_temp.path().join("selection.html")).expect("html dir");
    assert_eq!(
        write_bundle(
            &report,
            &BundlePaths {
                dir: html_temp.path().to_string_lossy().into_owned(),
                html: html_temp
                    .path()
                    .join("selection.html")
                    .to_string_lossy()
                    .into_owned(),
                text: html_temp
                    .path()
                    .join("selection.txt")
                    .to_string_lossy()
                    .into_owned(),
                report: html_temp
                    .path()
                    .join("report.json")
                    .to_string_lossy()
                    .into_owned(),
            },
        )
        .expect_err("html write should fail")
        .code,
        "CLI_BUNDLE_HTML_WRITE_FAILED"
    );

    let text_temp = tempdir().expect("tempdir");
    fs::create_dir(text_temp.path().join("selection.txt")).expect("text dir");
    assert_eq!(
        write_bundle(
            &report,
            &BundlePaths {
                dir: text_temp.path().to_string_lossy().into_owned(),
                html: text_temp
                    .path()
                    .join("selection.html")
                    .to_string_lossy()
                    .into_owned(),
                text: text_temp
                    .path()
                    .join("selection.txt")
                    .to_string_lossy()
                    .into_owned(),
                report: text_temp
                    .path()
                    .join("report.json")
                    .to_string_lossy()
                    .into_owned(),
            },
        )
        .expect_err("text write should fail")
        .code,
        "CLI_BUNDLE_TEXT_WRITE_FAILED"
    );

    let report_temp = tempdir().expect("tempdir");
    fs::create_dir(report_temp.path().join("report.json")).expect("report dir");
    assert_eq!(
        write_bundle(
            &report,
            &BundlePaths {
                dir: report_temp.path().to_string_lossy().into_owned(),
                html: report_temp
                    .path()
                    .join("selection.html")
                    .to_string_lossy()
                    .into_owned(),
                text: report_temp
                    .path()
                    .join("selection.txt")
                    .to_string_lossy()
                    .into_owned(),
                report: report_temp
                    .path()
                    .join("report.json")
                    .to_string_lossy()
                    .into_owned(),
            },
        )
        .expect_err("report write should fail")
        .code,
        "CLI_BUNDLE_REPORT_WRITE_FAILED"
    );
}

#[test]
fn prepared_builders_and_helper_edges_cover_remaining_branches() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture_file(tempdir.path(), "input.html", "<article>Hello</article>");
    let input = input_path.to_string_lossy().into_owned();

    let select = PreparedExtraction::from_select(SelectArgs {
        definition: DefinitionArgs { request_file: None },
        source: SourceArgs {
            input: Some(input.clone()),
            base_url: None,
            max_bytes: DEFAULT_MAX_BYTES.to_string(),
            fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
            fetch_preflight: CliFetchPreflightMode::HeadFirst,
        },
        css: Some("article".to_owned()),
        selection: SelectionArgs {
            r#match: CliMatchMode::First,
            index: None,
        },
        output: ExtractOutputArgs {
            value: CliValueMode::Text,
            attribute: None,
            whitespace: CliWhitespaceMode::Preserve,
            rewrite_urls: false,
            output: None,
            bundle: None,
            preview_chars: DEFAULT_PREVIEW_CHARS,
            include_source_text: false,
            output_file: None,
        },
    })
    .expect("select builder");
    assert_eq!(select.command, "select");

    let slice = PreparedExtraction::from_slice(SliceArgs {
        definition: DefinitionArgs { request_file: None },
        source: SourceArgs {
            input: Some(input.clone()),
            base_url: None,
            max_bytes: DEFAULT_MAX_BYTES.to_string(),
            fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
            fetch_preflight: CliFetchPreflightMode::HeadFirst,
        },
        from: Some("<article>".to_owned()),
        to: Some("</article>".to_owned()),
        pattern: CliPatternMode::Literal,
        regex_flags: None,
        include_start: false,
        include_end: false,
        selection: SelectionArgs {
            r#match: CliMatchMode::First,
            index: None,
        },
        output: ExtractOutputArgs {
            value: CliValueMode::Text,
            attribute: None,
            whitespace: CliWhitespaceMode::Preserve,
            rewrite_urls: false,
            output: Some(CliOutputMode::Json),
            bundle: None,
            preview_chars: DEFAULT_PREVIEW_CHARS,
            include_source_text: false,
            output_file: None,
        },
    })
    .expect("slice builder");
    assert_eq!(slice.command, "slice");

    let preview = PreparedPreview::from_select(InspectSelectArgs {
        definition: DefinitionArgs { request_file: None },
        source: SourceArgs {
            input: Some(input.clone()),
            base_url: None,
            max_bytes: DEFAULT_MAX_BYTES.to_string(),
            fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
            fetch_preflight: CliFetchPreflightMode::HeadFirst,
        },
        css: Some("article".to_owned()),
        selection: SelectionArgs {
            r#match: CliMatchMode::First,
            index: None,
        },
        whitespace: CliWhitespaceMode::Normalize,
        rewrite_urls: false,
        output: InspectOutputArgs {
            output: CliInspectOutputMode::Text,
            preview_chars: DEFAULT_PREVIEW_CHARS,
            include_source_text: false,
            output_file: None,
        },
    })
    .expect("preview builder");
    assert_eq!(
        preview.request.normalization.whitespace,
        WhitespaceMode::Normalize
    );
    assert!(
        PreparedExtraction::from_select(SelectArgs {
            definition: DefinitionArgs { request_file: None },
            source: SourceArgs {
                input: Some(input.clone()),
                base_url: None,
                max_bytes: "banana".to_owned(),
                fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
                fetch_preflight: CliFetchPreflightMode::HeadFirst,
            },
            css: Some("article".to_owned()),
            selection: SelectionArgs {
                r#match: CliMatchMode::First,
                index: None,
            },
            output: ExtractOutputArgs {
                value: CliValueMode::Text,
                attribute: None,
                whitespace: CliWhitespaceMode::Preserve,
                rewrite_urls: false,
                output: None,
                bundle: None,
                preview_chars: DEFAULT_PREVIEW_CHARS,
                include_source_text: false,
                output_file: None,
            },
        })
        .is_err()
    );
    assert!(
        PreparedExtraction::from_select(SelectArgs {
            definition: DefinitionArgs { request_file: None },
            source: SourceArgs {
                input: Some(input.clone()),
                base_url: Some("ftp://example.com".to_owned()),
                max_bytes: DEFAULT_MAX_BYTES.to_string(),
                fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
                fetch_preflight: CliFetchPreflightMode::HeadFirst,
            },
            css: Some("article".to_owned()),
            selection: SelectionArgs {
                r#match: CliMatchMode::First,
                index: None,
            },
            output: ExtractOutputArgs {
                value: CliValueMode::Text,
                attribute: None,
                whitespace: CliWhitespaceMode::Preserve,
                rewrite_urls: false,
                output: None,
                bundle: None,
                preview_chars: DEFAULT_PREVIEW_CHARS,
                include_source_text: false,
                output_file: None,
            },
        })
        .is_err()
    );
    assert!(
        PreparedExtraction::from_select(SelectArgs {
            definition: DefinitionArgs { request_file: None },
            source: SourceArgs {
                input: Some(input.clone()),
                base_url: None,
                max_bytes: DEFAULT_MAX_BYTES.to_string(),
                fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
                fetch_preflight: CliFetchPreflightMode::HeadFirst,
            },
            css: Some("article".to_owned()),
            selection: SelectionArgs {
                r#match: CliMatchMode::First,
                index: None,
            },
            output: ExtractOutputArgs {
                value: CliValueMode::Attribute,
                attribute: None,
                whitespace: CliWhitespaceMode::Preserve,
                rewrite_urls: false,
                output: None,
                bundle: None,
                preview_chars: DEFAULT_PREVIEW_CHARS,
                include_source_text: false,
                output_file: None,
            },
        })
        .is_err()
    );
    assert!(
        PreparedExtraction::from_slice(SliceArgs {
            definition: DefinitionArgs { request_file: None },
            source: SourceArgs {
                input: Some(input.clone()),
                base_url: None,
                max_bytes: "banana".to_owned(),
                fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
                fetch_preflight: CliFetchPreflightMode::HeadFirst,
            },
            from: Some("<article>".to_owned()),
            to: Some("</article>".to_owned()),
            pattern: CliPatternMode::Literal,
            regex_flags: None,
            include_start: false,
            include_end: false,
            selection: SelectionArgs {
                r#match: CliMatchMode::First,
                index: None,
            },
            output: ExtractOutputArgs {
                value: CliValueMode::Text,
                attribute: None,
                whitespace: CliWhitespaceMode::Preserve,
                rewrite_urls: false,
                output: None,
                bundle: None,
                preview_chars: DEFAULT_PREVIEW_CHARS,
                include_source_text: false,
                output_file: None,
            },
        })
        .is_err()
    );
    assert!(
        PreparedExtraction::from_slice(SliceArgs {
            definition: DefinitionArgs { request_file: None },
            source: SourceArgs {
                input: Some(input.clone()),
                base_url: Some("ftp://example.com".to_owned()),
                max_bytes: DEFAULT_MAX_BYTES.to_string(),
                fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
                fetch_preflight: CliFetchPreflightMode::HeadFirst,
            },
            from: Some("<article>".to_owned()),
            to: Some("</article>".to_owned()),
            pattern: CliPatternMode::Literal,
            regex_flags: None,
            include_start: false,
            include_end: false,
            selection: SelectionArgs {
                r#match: CliMatchMode::First,
                index: None,
            },
            output: ExtractOutputArgs {
                value: CliValueMode::Text,
                attribute: None,
                whitespace: CliWhitespaceMode::Preserve,
                rewrite_urls: false,
                output: None,
                bundle: None,
                preview_chars: DEFAULT_PREVIEW_CHARS,
                include_source_text: false,
                output_file: None,
            },
        })
        .is_err()
    );
    assert!(
        PreparedExtraction::from_slice(SliceArgs {
            definition: DefinitionArgs { request_file: None },
            source: SourceArgs {
                input: Some(input.clone()),
                base_url: None,
                max_bytes: DEFAULT_MAX_BYTES.to_string(),
                fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
                fetch_preflight: CliFetchPreflightMode::HeadFirst,
            },
            from: Some("<article>".to_owned()),
            to: Some("</article>".to_owned()),
            pattern: CliPatternMode::Literal,
            regex_flags: None,
            include_start: false,
            include_end: false,
            selection: SelectionArgs {
                r#match: CliMatchMode::First,
                index: None,
            },
            output: ExtractOutputArgs {
                value: CliValueMode::Attribute,
                attribute: None,
                whitespace: CliWhitespaceMode::Preserve,
                rewrite_urls: false,
                output: None,
                bundle: None,
                preview_chars: DEFAULT_PREVIEW_CHARS,
                include_source_text: false,
                output_file: None,
            },
        })
        .is_err()
    );
    assert!(
        PreparedSourceInspection::new(InspectSourceArgs {
            source: SourceArgs {
                input: Some(input.clone()),
                base_url: None,
                max_bytes: DEFAULT_MAX_BYTES.to_string(),
                fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
                fetch_preflight: CliFetchPreflightMode::HeadFirst,
            },
            sample_limit: DEFAULT_INSPECTION_SAMPLE_LIMIT,
            output: CliInspectOutputMode::Text,
            include_source_text: false,
            output_file: None,
            preview_chars: 0,
        })
        .is_err()
    );
    assert!(
        PreparedSourceInspection::new(InspectSourceArgs {
            source: SourceArgs {
                input: Some(input.clone()),
                base_url: None,
                max_bytes: "banana".to_owned(),
                fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
                fetch_preflight: CliFetchPreflightMode::HeadFirst,
            },
            sample_limit: DEFAULT_INSPECTION_SAMPLE_LIMIT,
            output: CliInspectOutputMode::Text,
            include_source_text: false,
            output_file: None,
            preview_chars: DEFAULT_PREVIEW_CHARS,
        })
        .is_err()
    );
    assert!(
        PreparedPreview::from_select(InspectSelectArgs {
            definition: DefinitionArgs { request_file: None },
            source: SourceArgs {
                input: Some(input.clone()),
                base_url: None,
                max_bytes: "banana".to_owned(),
                fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
                fetch_preflight: CliFetchPreflightMode::HeadFirst,
            },
            css: Some("article".to_owned()),
            selection: SelectionArgs {
                r#match: CliMatchMode::First,
                index: None,
            },
            whitespace: CliWhitespaceMode::Normalize,
            rewrite_urls: false,
            output: InspectOutputArgs {
                output: CliInspectOutputMode::Text,
                preview_chars: DEFAULT_PREVIEW_CHARS,
                include_source_text: false,
                output_file: None,
            },
        })
        .is_err()
    );
    assert!(
        PreparedPreview::from_slice(InspectSliceArgs {
            definition: DefinitionArgs { request_file: None },
            source: SourceArgs {
                input: Some(input.clone()),
                base_url: None,
                max_bytes: "banana".to_owned(),
                fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
                fetch_preflight: CliFetchPreflightMode::HeadFirst,
            },
            from: Some("<article>".to_owned()),
            to: Some("</article>".to_owned()),
            pattern: CliPatternMode::Literal,
            regex_flags: None,
            include_start: false,
            include_end: false,
            selection: SelectionArgs {
                r#match: CliMatchMode::First,
                index: None,
            },
            whitespace: CliWhitespaceMode::Normalize,
            rewrite_urls: false,
            output: InspectOutputArgs {
                output: CliInspectOutputMode::Text,
                preview_chars: DEFAULT_PREVIEW_CHARS,
                include_source_text: false,
                output_file: None,
            },
        })
        .is_err()
    );

    assert!(!raw_args_prefers_json(&[
        "htmlcut".to_owned(),
        "select".to_owned(),
        "page.html".to_owned(),
        "--output=text".to_owned(),
    ]));
    assert!(raw_args_prefers_json(&[
        "htmlcut".to_owned(),
        "select".to_owned(),
        "page.html".to_owned(),
        "--output".to_owned(),
        "json".to_owned(),
    ]));
    assert!(!raw_args_prefers_json(&[
        "htmlcut".to_owned(),
        "select".to_owned(),
        "page.html".to_owned(),
        "--output".to_owned(),
        "none".to_owned(),
    ]));
    assert!(!raw_args_prefers_json(&[
        "htmlcut".to_owned(),
        "select".to_owned(),
        "page.html".to_owned(),
    ]));
    assert_eq!(
        command_name_from_raw_args(&[
            "htmlcut".to_owned(),
            "inspect".to_owned(),
            "mystery".to_owned(),
        ]),
        "inspect"
    );
    assert_eq!(
        command_name_from_raw_args(&["htmlcut".to_owned(), "--help".to_owned()]),
        "htmlcut"
    );

    let report = build_extraction_report(
        "select",
        fixture_result(Value::String("Hello".to_owned()), ValueType::Text),
        None,
    );
    assert_eq!(build_verbose_lines(&report, 1).len(), 1);

    let mut minimal_inspection = fixture_inspection();
    let document = minimal_inspection.document.as_mut().expect("document");
    document.top_tags.clear();
    document.top_classes.clear();
    document.headings.clear();
    document.links.clear();
    let rendered = render_source_inspection_text(&minimal_inspection, DEFAULT_PREVIEW_CHARS);
    assert!(!rendered.contains("Top tags:"));
    assert!(!rendered.contains("Top classes:"));
    assert!(!rendered.contains("Headings:"));
    assert!(!rendered.contains("Link previews:"));
}

#[test]
fn request_file_builders_and_output_file_edges_cover_remaining_branches() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture_file(tempdir.path(), "input.html", "<article>Hello</article>");
    let input = input_path.to_string_lossy().into_owned();

    let selector_definition = ExtractionDefinition::new(ExtractionRequest::new(
        SourceRequest::file(&input_path),
        ExtractionSpec::selector(SelectorQuery::new("article").expect("selector"))
            .with_selection(SelectionSpec::single())
            .with_value(ValueSpec::Text),
    ));
    let selector_definition_path = write_definition_file(
        tempdir.path(),
        "selector-request.json",
        &selector_definition,
    );

    let slice_definition = ExtractionDefinition::new(ExtractionRequest::new(
        SourceRequest::file(&input_path),
        ExtractionSpec::slice(
            htmlcut_core::SliceSpec::new(
                htmlcut_core::SliceBoundary::new("<article>").expect("slice boundary"),
                htmlcut_core::SliceBoundary::new("</article>").expect("slice boundary"),
            )
            .with_boundary_inclusion(true, true),
        )
        .with_selection(SelectionSpec::single())
        .with_value(ValueSpec::Text),
    ));
    let slice_definition_path =
        write_definition_file(tempdir.path(), "slice-request.json", &slice_definition);
    let request_file_output_path = tempdir.path().join("request-file-output.json");

    let get_only_runtime = build_runtime(&SourceArgs {
        input: Some(input.clone()),
        base_url: None,
        max_bytes: DEFAULT_MAX_BYTES.to_string(),
        fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
        fetch_preflight: CliFetchPreflightMode::GetOnly,
    })
    .expect("runtime");
    assert_eq!(
        get_only_runtime.fetch_preflight,
        FetchPreflightMode::GetOnly
    );

    assert_eq!(
        build_source_request(&SourceArgs {
            input: None,
            base_url: None,
            max_bytes: DEFAULT_MAX_BYTES.to_string(),
            fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
            fetch_preflight: CliFetchPreflightMode::HeadFirst,
        })
        .expect_err("missing input")
        .code,
        "CLI_REQUIRED_PARAMETER_MISSING"
    );

    assert_eq!(
        expect_cli_error(
            load_extraction_definition_for_tests(
                &tempdir.path().join("missing-request.json"),
                ExtractionStrategy::Selector,
                "select",
            ),
            "missing request file",
        )
        .code,
        "CLI_REQUEST_FILE_READ_FAILED"
    );

    let invalid_json_path = write_fixture_file(tempdir.path(), "invalid-request.json", "{not json");
    assert_eq!(
        expect_cli_error(
            load_extraction_definition_for_tests(
                &invalid_json_path,
                ExtractionStrategy::Selector,
                "select",
            ),
            "invalid request file json",
        )
        .code,
        "CLI_REQUEST_FILE_INVALID"
    );

    let mut unsupported_schema =
        serde_json::to_value(&selector_definition).expect("definition json");
    unsupported_schema["schema_name"] = Value::String("synthetic.request".to_owned());
    unsupported_schema["schema_version"] = Value::from(99);
    let unsupported_schema_path = tempdir.path().join("unsupported-schema.json");
    fs::write(
        &unsupported_schema_path,
        serde_json::to_string_pretty(&unsupported_schema).expect("serialize unsupported schema"),
    )
    .expect("write unsupported schema");
    assert_eq!(
        expect_cli_error(
            load_extraction_definition_for_tests(
                &unsupported_schema_path,
                ExtractionStrategy::Selector,
                "select",
            ),
            "unsupported schema",
        )
        .code,
        "CLI_REQUEST_FILE_SCHEMA_UNSUPPORTED"
    );

    let mut unsupported_version =
        serde_json::to_value(&selector_definition).expect("definition json");
    unsupported_version["schema_version"] = Value::from(99);
    let unsupported_version_path = tempdir.path().join("unsupported-version.json");
    fs::write(
        &unsupported_version_path,
        serde_json::to_string_pretty(&unsupported_version).expect("serialize unsupported version"),
    )
    .expect("write unsupported version");
    assert_eq!(
        expect_cli_error(
            load_extraction_definition_for_tests(
                &unsupported_version_path,
                ExtractionStrategy::Selector,
                "select",
            ),
            "unsupported version",
        )
        .code,
        "CLI_REQUEST_FILE_SCHEMA_UNSUPPORTED"
    );

    assert_eq!(
        expect_cli_error(
            load_extraction_definition_for_tests(
                &selector_definition_path,
                ExtractionStrategy::Slice,
                "slice",
            ),
            "strategy mismatch",
        )
        .code,
        "CLI_REQUEST_FILE_STRATEGY_MISMATCH"
    );
    assert_eq!(
        expect_cli_error(
            load_extraction_definition_for_tests(
                &slice_definition_path,
                ExtractionStrategy::Selector,
                "select",
            ),
            "slice strategy mismatch",
        )
        .code,
        "CLI_REQUEST_FILE_STRATEGY_MISMATCH"
    );

    let prepared_slice = PreparedExtraction::from_slice_with_logging(
        SliceArgs {
            definition: DefinitionArgs {
                request_file: Some(slice_definition_path.clone()),
            },
            source: SourceArgs {
                input: None,
                base_url: None,
                max_bytes: DEFAULT_MAX_BYTES.to_string(),
                fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
                fetch_preflight: CliFetchPreflightMode::HeadFirst,
            },
            from: None,
            to: None,
            pattern: CliPatternMode::Literal,
            regex_flags: None,
            include_start: false,
            include_end: false,
            selection: SelectionArgs {
                r#match: CliMatchMode::First,
                index: None,
            },
            output: ExtractOutputArgs {
                value: CliValueMode::Text,
                attribute: None,
                whitespace: CliWhitespaceMode::Preserve,
                rewrite_urls: false,
                output: None,
                bundle: None,
                output_file: Some(request_file_output_path.clone()),
                preview_chars: DEFAULT_PREVIEW_CHARS,
                include_source_text: false,
            },
        },
        0,
        false,
    )
    .expect("slice request file");
    assert_eq!(
        prepared_slice.request.extraction.strategy(),
        ExtractionStrategy::Slice
    );
    assert_eq!(
        prepared_slice.output_file.as_deref(),
        Some(request_file_output_path.as_path())
    );

    let preview_select = PreparedPreview::from_select(InspectSelectArgs {
        definition: DefinitionArgs {
            request_file: Some(selector_definition_path.clone()),
        },
        source: SourceArgs {
            input: None,
            base_url: None,
            max_bytes: DEFAULT_MAX_BYTES.to_string(),
            fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
            fetch_preflight: CliFetchPreflightMode::HeadFirst,
        },
        css: None,
        selection: SelectionArgs {
            r#match: CliMatchMode::First,
            index: None,
        },
        whitespace: CliWhitespaceMode::Preserve,
        rewrite_urls: false,
        output: InspectOutputArgs {
            output: CliInspectOutputMode::Json,
            preview_chars: DEFAULT_PREVIEW_CHARS,
            include_source_text: false,
            output_file: None,
        },
    })
    .expect("inspect select request file");
    assert_eq!(
        preview_select.request.extraction.value(),
        &ValueSpec::Structured
    );

    let preview_slice = PreparedPreview::from_slice(InspectSliceArgs {
        definition: DefinitionArgs {
            request_file: Some(slice_definition_path.clone()),
        },
        source: SourceArgs {
            input: None,
            base_url: None,
            max_bytes: DEFAULT_MAX_BYTES.to_string(),
            fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
            fetch_preflight: CliFetchPreflightMode::HeadFirst,
        },
        from: None,
        to: None,
        pattern: CliPatternMode::Literal,
        regex_flags: None,
        include_start: false,
        include_end: false,
        selection: SelectionArgs {
            r#match: CliMatchMode::First,
            index: None,
        },
        whitespace: CliWhitespaceMode::Preserve,
        rewrite_urls: false,
        output: InspectOutputArgs {
            output: CliInspectOutputMode::Json,
            preview_chars: DEFAULT_PREVIEW_CHARS,
            include_source_text: false,
            output_file: None,
        },
    })
    .expect("inspect slice request file");
    assert_eq!(
        preview_slice.request.extraction.value(),
        &ValueSpec::Structured
    );

    let slice_conflict = expect_cli_error(
        PreparedExtraction::from_slice_with_logging(
            SliceArgs {
                definition: DefinitionArgs {
                    request_file: Some(slice_definition_path.clone()),
                },
                source: SourceArgs {
                    input: Some(input.clone()),
                    base_url: Some("https://example.com/base/".to_owned()),
                    max_bytes: DEFAULT_MAX_BYTES.to_string(),
                    fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
                    fetch_preflight: CliFetchPreflightMode::HeadFirst,
                },
                from: Some("<article>".to_owned()),
                to: Some("</article>".to_owned()),
                pattern: CliPatternMode::Regex,
                regex_flags: Some("u".to_owned()),
                include_start: true,
                include_end: true,
                selection: SelectionArgs {
                    r#match: CliMatchMode::Nth,
                    index: Some(2),
                },
                output: ExtractOutputArgs {
                    value: CliValueMode::Structured,
                    attribute: None,
                    whitespace: CliWhitespaceMode::Normalize,
                    rewrite_urls: true,
                    output: Some(CliOutputMode::Json),
                    bundle: Some(tempdir.path().join("bundle")),
                    output_file: Some(tempdir.path().join("stdout.json")),
                    preview_chars: DEFAULT_PREVIEW_CHARS + 1,
                    include_source_text: true,
                },
            },
            0,
            false,
        ),
        "slice request file conflict",
    );
    assert_eq!(slice_conflict.code, "CLI_REQUEST_FILE_CONFLICT");
    assert!(slice_conflict.message.contains("--regex-flags"));
    assert!(!slice_conflict.message.contains("--output-file"));

    let inspect_select_conflict = expect_cli_error(
        PreparedPreview::from_select(InspectSelectArgs {
            definition: DefinitionArgs {
                request_file: Some(selector_definition_path.clone()),
            },
            source: SourceArgs {
                input: Some(input.clone()),
                base_url: Some("https://example.com/base/".to_owned()),
                max_bytes: DEFAULT_MAX_BYTES.to_string(),
                fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
                fetch_preflight: CliFetchPreflightMode::HeadFirst,
            },
            css: Some("article".to_owned()),
            selection: SelectionArgs {
                r#match: CliMatchMode::Nth,
                index: Some(2),
            },
            whitespace: CliWhitespaceMode::Normalize,
            rewrite_urls: true,
            output: InspectOutputArgs {
                output: CliInspectOutputMode::Text,
                preview_chars: DEFAULT_PREVIEW_CHARS + 1,
                include_source_text: true,
                output_file: None,
            },
        }),
        "inspect select request file conflict",
    );
    assert_eq!(inspect_select_conflict.code, "CLI_REQUEST_FILE_CONFLICT");
    assert!(inspect_select_conflict.message.contains("--whitespace"));
    assert!(inspect_select_conflict.message.contains("--preview-chars"));

    let inspect_slice_conflict = expect_cli_error(
        PreparedPreview::from_slice(InspectSliceArgs {
            definition: DefinitionArgs {
                request_file: Some(slice_definition_path),
            },
            source: SourceArgs {
                input: Some(input),
                base_url: Some("https://example.com/base/".to_owned()),
                max_bytes: DEFAULT_MAX_BYTES.to_string(),
                fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
                fetch_preflight: CliFetchPreflightMode::HeadFirst,
            },
            from: Some("<article>".to_owned()),
            to: Some("</article>".to_owned()),
            pattern: CliPatternMode::Regex,
            regex_flags: Some("u".to_owned()),
            include_start: true,
            include_end: true,
            selection: SelectionArgs {
                r#match: CliMatchMode::Nth,
                index: Some(2),
            },
            whitespace: CliWhitespaceMode::Normalize,
            rewrite_urls: true,
            output: InspectOutputArgs {
                output: CliInspectOutputMode::Text,
                preview_chars: DEFAULT_PREVIEW_CHARS + 1,
                include_source_text: true,
                output_file: None,
            },
        }),
        "inspect slice request file conflict",
    );
    assert_eq!(inspect_slice_conflict.code, "CLI_REQUEST_FILE_CONFLICT");
    assert!(inspect_slice_conflict.message.contains("--include-start"));
    assert!(
        inspect_slice_conflict
            .message
            .contains("--include-source-text")
    );

    assert_eq!(
        resolve_extract_output_mode_with_output_file(
            Some(CliOutputMode::None),
            &ValueType::Text,
            Some(tempdir.path()),
            Some(&tempdir.path().join("selection.txt")),
        )
        .expect_err("output file requires stdout payload")
        .code,
        "CLI_OUTPUT_FILE_REQUIRES_STDOUT_PAYLOAD"
    );

    let nested_output = tempdir.path().join("nested/output/selection.txt");
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit_code = write_outcome(
        ExecutionOutcome {
            stdout: Some("Hello".to_owned()),
            output_file: Some(nested_output.clone()),
            stderr: Vec::new(),
            exit_code: 0,
        },
        &mut stdout,
        &mut stderr,
    );
    assert_eq!(exit_code, 0);
    assert!(stdout.is_empty());
    assert!(stderr.is_empty());
    assert_eq!(
        fs::read_to_string(&nested_output).expect("nested output file"),
        "Hello\n"
    );

    let direct_nested_output = tempdir.path().join("direct/output/report.txt");
    write_stdout_payload_for_tests(&direct_nested_output, "Hello")
        .expect("write stdout payload with nested parent");
    assert_eq!(
        fs::read_to_string(&direct_nested_output).expect("direct nested output file"),
        "Hello\n"
    );
    let relative_output =
        PathBuf::from(format!(".htmlcut-write-payload-{}.txt", std::process::id()));
    write_stdout_payload_for_tests(&relative_output, "Hello")
        .expect("write stdout payload without parent directory");
    assert_eq!(
        fs::read_to_string(&relative_output).expect("relative output file"),
        "Hello\n"
    );
    fs::remove_file(&relative_output).expect("remove relative output file");
    assert!(
        write_stdout_payload_for_tests(Path::new("/"), "Hello")
            .expect_err("root write should fail")
            .kind()
            != std::io::ErrorKind::NotFound
    );

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit_code = write_outcome(
        ExecutionOutcome {
            stdout: Some("Hello".to_owned()),
            output_file: Some(tempdir.path().to_path_buf()),
            stderr: Vec::new(),
            exit_code: 0,
        },
        &mut stdout,
        &mut stderr,
    );
    assert_eq!(exit_code, EXIT_CODE_OUTPUT);
    assert!(stdout.is_empty());
    assert!(
        String::from_utf8(stderr)
            .expect("stderr")
            .contains("Could not write")
    );
}

#[test]
fn execution_paths_cover_direct_success_and_failure_variants() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture_file(
        tempdir.path(),
        "input.html",
        "<article><p>Hello</p></article>",
    );
    let input = input_path.to_string_lossy().into_owned();

    assert_eq!(
        validate_base_url(Some("https://example.com/docs"))
            .expect("valid base url")
            .as_ref()
            .map(|url| url.as_str()),
        Some("https://example.com/docs")
    );
    assert_eq!(
        validate_base_url(Some("http://example.com/docs"))
            .expect("valid http base url")
            .as_ref()
            .map(|url| url.as_str()),
        Some("http://example.com/docs")
    );
    assert_eq!(parse_byte_size("512").expect("plain bytes"), 512);
    assert!(parse_byte_size(&"9".repeat(400)).is_err());
    assert!(!raw_args_prefers_json(&[
        "htmlcut".to_owned(),
        "select".to_owned(),
        "page.html".to_owned(),
        "--output=html".to_owned(),
    ]));
    assert!(!raw_args_prefers_json(&[
        "htmlcut".to_owned(),
        "select".to_owned(),
        "page.html".to_owned(),
        "--value".to_owned(),
        "text".to_owned(),
    ]));
    assert!(!raw_args_prefers_json(&[
        "htmlcut".to_owned(),
        "select".to_owned(),
        "page.html".to_owned(),
        "--output".to_owned(),
    ]));

    let missing = tempdir
        .path()
        .join("missing.html")
        .to_string_lossy()
        .into_owned();
    let inspect_text = run_inspect_source(InspectSourceArgs {
        source: SourceArgs {
            input: Some(missing.clone()),
            base_url: None,
            max_bytes: DEFAULT_MAX_BYTES.to_string(),
            fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
            fetch_preflight: CliFetchPreflightMode::HeadFirst,
        },
        output: CliInspectOutputMode::Text,
        include_source_text: false,
        output_file: None,
        sample_limit: DEFAULT_INSPECTION_SAMPLE_LIMIT,
        preview_chars: DEFAULT_PREVIEW_CHARS,
    });
    assert_eq!(inspect_text.exit_code, EXIT_CODE_SOURCE);
    assert!(inspect_text.stdout.is_none());
    assert!(inspect_text.stderr[0].contains("Could not access file"));

    let inspect_json = run_inspect_source(InspectSourceArgs {
        source: SourceArgs {
            input: Some(missing),
            base_url: None,
            max_bytes: DEFAULT_MAX_BYTES.to_string(),
            fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
            fetch_preflight: CliFetchPreflightMode::HeadFirst,
        },
        output: CliInspectOutputMode::Json,
        include_source_text: false,
        output_file: None,
        sample_limit: DEFAULT_INSPECTION_SAMPLE_LIMIT,
        preview_chars: DEFAULT_PREVIEW_CHARS,
    });
    assert_eq!(inspect_json.exit_code, EXIT_CODE_SOURCE);
    assert!(
        inspect_json
            .stdout
            .as_deref()
            .is_some_and(|stdout| stdout.contains("\"command\": \"inspect-source\""))
    );

    let preview_text = execute_preview(
        PreparedPreview::from_select(InspectSelectArgs {
            definition: DefinitionArgs { request_file: None },
            source: SourceArgs {
                input: Some(input.clone()),
                base_url: None,
                max_bytes: DEFAULT_MAX_BYTES.to_string(),
                fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
                fetch_preflight: CliFetchPreflightMode::HeadFirst,
            },
            css: Some("[".to_owned()),
            selection: SelectionArgs {
                r#match: CliMatchMode::First,
                index: None,
            },
            whitespace: CliWhitespaceMode::Preserve,
            rewrite_urls: false,
            output: InspectOutputArgs {
                output: CliInspectOutputMode::Text,
                preview_chars: DEFAULT_PREVIEW_CHARS,
                include_source_text: false,
                output_file: None,
            },
        })
        .expect("preview builder"),
    );
    assert_eq!(preview_text.exit_code, EXIT_CODE_USAGE);
    assert!(preview_text.stdout.is_none());
    assert!(preview_text.stderr[0].contains("Invalid selector"));

    let preview_json = execute_preview(
        PreparedPreview::from_select(InspectSelectArgs {
            definition: DefinitionArgs { request_file: None },
            source: SourceArgs {
                input: Some(input.clone()),
                base_url: None,
                max_bytes: DEFAULT_MAX_BYTES.to_string(),
                fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
                fetch_preflight: CliFetchPreflightMode::HeadFirst,
            },
            css: Some("[".to_owned()),
            selection: SelectionArgs {
                r#match: CliMatchMode::First,
                index: None,
            },
            whitespace: CliWhitespaceMode::Preserve,
            rewrite_urls: false,
            output: InspectOutputArgs {
                output: CliInspectOutputMode::Json,
                preview_chars: DEFAULT_PREVIEW_CHARS,
                include_source_text: false,
                output_file: None,
            },
        })
        .expect("preview builder"),
    );
    assert_eq!(preview_json.exit_code, EXIT_CODE_USAGE);
    assert!(
        preview_json
            .stdout
            .as_deref()
            .is_some_and(|stdout| stdout.contains("\"ok\": false"))
    );

    let preview_success_json = execute_preview(
        PreparedPreview::from_select(InspectSelectArgs {
            definition: DefinitionArgs { request_file: None },
            source: SourceArgs {
                input: Some(input.clone()),
                base_url: None,
                max_bytes: DEFAULT_MAX_BYTES.to_string(),
                fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
                fetch_preflight: CliFetchPreflightMode::HeadFirst,
            },
            css: Some("article".to_owned()),
            selection: SelectionArgs {
                r#match: CliMatchMode::First,
                index: None,
            },
            whitespace: CliWhitespaceMode::Preserve,
            rewrite_urls: false,
            output: InspectOutputArgs {
                output: CliInspectOutputMode::Json,
                preview_chars: DEFAULT_PREVIEW_CHARS,
                include_source_text: false,
                output_file: None,
            },
        })
        .expect("preview builder"),
    );
    assert_eq!(preview_success_json.exit_code, 0);
    assert!(
        preview_success_json
            .stdout
            .as_deref()
            .is_some_and(|stdout| stdout.contains("\"command\": \"inspect-select\""))
    );

    let extract_text = execute_extraction(
        PreparedExtraction::from_select(SelectArgs {
            definition: DefinitionArgs { request_file: None },
            source: SourceArgs {
                input: Some(input.clone()),
                base_url: None,
                max_bytes: DEFAULT_MAX_BYTES.to_string(),
                fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
                fetch_preflight: CliFetchPreflightMode::HeadFirst,
            },
            css: Some("[".to_owned()),
            selection: SelectionArgs {
                r#match: CliMatchMode::First,
                index: None,
            },
            output: ExtractOutputArgs {
                value: CliValueMode::Text,
                attribute: None,
                whitespace: CliWhitespaceMode::Preserve,
                rewrite_urls: false,
                output: Some(CliOutputMode::Text),
                bundle: None,
                preview_chars: DEFAULT_PREVIEW_CHARS,
                include_source_text: false,
                output_file: None,
            },
        })
        .expect("extract builder"),
    );
    assert_eq!(extract_text.exit_code, EXIT_CODE_USAGE);
    assert!(extract_text.stdout.is_none());
    assert!(extract_text.stderr[0].contains("Invalid selector"));

    let bundle_dir = tempdir.path().join("bundle out");
    let extract_success = execute_extraction(
        PreparedExtraction::from_select_with_logging(
            SelectArgs {
                definition: DefinitionArgs { request_file: None },
                source: SourceArgs {
                    input: Some(input.clone()),
                    base_url: None,
                    max_bytes: DEFAULT_MAX_BYTES.to_string(),
                    fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
                    fetch_preflight: CliFetchPreflightMode::HeadFirst,
                },
                css: Some("article".to_owned()),
                selection: SelectionArgs {
                    r#match: CliMatchMode::First,
                    index: None,
                },
                output: ExtractOutputArgs {
                    value: CliValueMode::Text,
                    attribute: None,
                    whitespace: CliWhitespaceMode::Preserve,
                    rewrite_urls: false,
                    output: Some(CliOutputMode::Text),
                    bundle: Some(bundle_dir.clone()),
                    preview_chars: DEFAULT_PREVIEW_CHARS,
                    include_source_text: false,
                    output_file: None,
                },
            },
            1,
            false,
        )
        .expect("extract builder"),
    );
    assert_eq!(extract_success.exit_code, 0);
    assert!(
        extract_success
            .stderr
            .iter()
            .any(|line| line.contains("wrote bundle to"))
    );
    assert!(bundle_dir.join("report.json").exists());

    let extract_success_no_bundle = execute_extraction(
        PreparedExtraction::from_select(SelectArgs {
            definition: DefinitionArgs { request_file: None },
            source: SourceArgs {
                input: Some(input.clone()),
                base_url: None,
                max_bytes: DEFAULT_MAX_BYTES.to_string(),
                fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
                fetch_preflight: CliFetchPreflightMode::HeadFirst,
            },
            css: Some("article".to_owned()),
            selection: SelectionArgs {
                r#match: CliMatchMode::First,
                index: None,
            },
            output: ExtractOutputArgs {
                value: CliValueMode::Text,
                attribute: None,
                whitespace: CliWhitespaceMode::Preserve,
                rewrite_urls: false,
                output: Some(CliOutputMode::Text),
                bundle: None,
                preview_chars: DEFAULT_PREVIEW_CHARS,
                include_source_text: false,
                output_file: None,
            },
        })
        .expect("extract builder"),
    );
    assert_eq!(extract_success_no_bundle.exit_code, 0);
    assert!(extract_success_no_bundle.stderr.is_empty());

    let extract_success_verbose_no_bundle = execute_extraction(
        PreparedExtraction::from_select_with_logging(
            SelectArgs {
                definition: DefinitionArgs { request_file: None },
                source: SourceArgs {
                    input: Some(input.clone()),
                    base_url: None,
                    max_bytes: DEFAULT_MAX_BYTES.to_string(),
                    fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
                    fetch_preflight: CliFetchPreflightMode::HeadFirst,
                },
                css: Some("article".to_owned()),
                selection: SelectionArgs {
                    r#match: CliMatchMode::First,
                    index: None,
                },
                output: ExtractOutputArgs {
                    value: CliValueMode::Text,
                    attribute: None,
                    whitespace: CliWhitespaceMode::Preserve,
                    rewrite_urls: false,
                    output: Some(CliOutputMode::Text),
                    bundle: None,
                    preview_chars: DEFAULT_PREVIEW_CHARS,
                    include_source_text: false,
                    output_file: None,
                },
            },
            1,
            false,
        )
        .expect("extract builder"),
    );
    assert_eq!(extract_success_verbose_no_bundle.exit_code, 0);
    assert_eq!(extract_success_verbose_no_bundle.stderr.len(), 1);

    assert!(
        PreparedExtraction::from_slice(SliceArgs {
            definition: DefinitionArgs { request_file: None },
            source: SourceArgs {
                input: Some(input.clone()),
                base_url: None,
                max_bytes: DEFAULT_MAX_BYTES.to_string(),
                fetch_timeout_ms: DEFAULT_FETCH_TIMEOUT_MS,
                fetch_preflight: CliFetchPreflightMode::HeadFirst,
            },
            from: Some("<article>".to_owned()),
            to: Some("</article>".to_owned()),
            pattern: CliPatternMode::Literal,
            regex_flags: None,
            include_start: false,
            include_end: false,
            selection: SelectionArgs {
                r#match: CliMatchMode::First,
                index: None,
            },
            output: ExtractOutputArgs {
                value: CliValueMode::Text,
                attribute: None,
                whitespace: CliWhitespaceMode::Preserve,
                rewrite_urls: false,
                output: Some(CliOutputMode::None),
                bundle: None,
                preview_chars: DEFAULT_PREVIEW_CHARS,
                include_source_text: false,
                output_file: None,
            },
        })
        .is_err()
    );

    let one_structured_match_report = ExtractionCommandReport {
        tool: TOOL_NAME.to_owned(),
        engine: ENGINE_NAME.to_owned(),
        version: HTMLCUT_VERSION.to_owned(),
        schema_name: EXTRACTION_COMMAND_REPORT_SCHEMA_NAME.to_owned(),
        schema_version: EXTRACTION_COMMAND_REPORT_SCHEMA_VERSION,
        command: "select".to_owned(),
        operation_id: htmlcut_core::OperationId::SelectExtract,
        ok: true,
        source: SourceMetadata {
            kind: SourceKind::File,
            value: "/tmp/input.html".to_owned(),
            input_base_url: None,
            effective_base_url: None,
            bytes_read: 10,
            text: None,
        },
        extraction: ExtractionSpec::selector(SelectorQuery::new("article").expect("selector"))
            .with_selection(SelectionSpec::default())
            .with_value(ValueSpec::Structured),
        stats: ExtractionStats {
            duration_ms: 1,
            candidate_count: 1,
            match_count: 1,
        },
        document_title: None,
        matches: vec![ExtractionMatch {
            index: 1,
            path: None,
            value_type: ValueType::Structured,
            value: serde_json::json!({"hello":"world"}),
            html: None,
            text: None,
            preview: "preview".to_owned(),
            metadata: selector_metadata(1, 1, "article:nth-of-type(1)", "article", &[]),
        }],
        diagnostics: Vec::new(),
        bundle: None,
    };
    assert!(wrap_html_document(&one_structured_match_report).contains("<pre>"));

    let mut multi_match_report = build_extraction_report(
        "select",
        fixture_result(
            Value::String("<p>Hello</p>".to_owned()),
            ValueType::InnerHtml,
        ),
        None,
    );
    multi_match_report.matches.push(ExtractionMatch {
        index: 2,
        path: Some("article:nth-of-type(2)".to_owned()),
        value_type: ValueType::OuterHtml,
        value: Value::String("<article>World</article>".to_owned()),
        html: Some("<article>World</article>".to_owned()),
        text: Some("World".to_owned()),
        preview: "World".to_owned(),
        metadata: selector_metadata(2, 2, "article:nth-of-type(2)", "article", &[]),
    });
    assert!(wrap_html_document(&multi_match_report).contains("data-match-index=\"2\""));

    let outer_html_match = ExtractionMatch {
        index: 1,
        path: None,
        value_type: ValueType::OuterHtml,
        value: Value::String("<article>Hello</article>".to_owned()),
        html: Some("<article>Hello</article>".to_owned()),
        text: Some("Hello".to_owned()),
        preview: "Hello".to_owned(),
        metadata: selector_metadata(1, 1, "article:nth-of-type(1)", "article", &[]),
    };
    assert_eq!(
        render_match_as_html(&outer_html_match),
        "<article>Hello</article>"
    );
}
