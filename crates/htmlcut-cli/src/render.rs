use std::fs;
use std::path::Path;

use std::collections::BTreeMap;

use htmlcut_core::{
    DEFAULT_PREVIEW_CHARS, Diagnostic, DiagnosticLevel, SchemaStability, ValueType,
    result::{ExtractionMatch, ExtractionMatchMetadata, InspectionCount, Range},
};
use serde::Serialize;
use serde_json::Value;
use url::Url;

use crate::args::CliOutputMode;
use crate::error::{CliError, output_error};
use crate::model::{
    BundlePaths, CatalogAvailability, CatalogCommandContract, CatalogCommandReport,
    CatalogCondition, CatalogConstraint, CatalogContractSurface, CatalogParameterKind,
    CatalogParameterRequirement, ExtractionCommandReport, SchemaCommandReport,
    SourceInspectionCommandReport,
};

pub(crate) fn get_bundle_paths(dir: &Path) -> BundlePaths {
    let dir = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
    BundlePaths {
        dir: dir.to_string_lossy().into_owned(),
        html: dir.join("selection.html").to_string_lossy().into_owned(),
        text: dir.join("selection.txt").to_string_lossy().into_owned(),
        report: dir.join("report.json").to_string_lossy().into_owned(),
    }
}

pub(crate) fn write_bundle(
    report: &ExtractionCommandReport,
    bundle: &BundlePaths,
) -> Result<(), CliError> {
    fs::create_dir_all(&bundle.dir).map_err(|error| {
        output_error(
            "CLI_BUNDLE_DIRECTORY_CREATE_FAILED",
            format!("Could not create bundle directory {}: {error}", bundle.dir),
        )
    })?;
    fs::write(&bundle.html, wrap_html_document(report)).map_err(|error| {
        output_error(
            "CLI_BUNDLE_HTML_WRITE_FAILED",
            format!("Could not write {}: {error}", bundle.html),
        )
    })?;
    fs::write(&bundle.text, render_text_payload(report)).map_err(|error| {
        output_error(
            "CLI_BUNDLE_TEXT_WRITE_FAILED",
            format!("Could not write {}: {error}", bundle.text),
        )
    })?;
    fs::write(&bundle.report, format!("{}\n", to_pretty_json(report))).map_err(|error| {
        output_error(
            "CLI_BUNDLE_REPORT_WRITE_FAILED",
            format!("Could not write {}: {error}", bundle.report),
        )
    })?;
    Ok(())
}

pub(crate) fn render_extraction_output(
    report: &ExtractionCommandReport,
    output: CliOutputMode,
) -> Option<String> {
    match output {
        CliOutputMode::Text => Some(render_text_payload(report)),
        CliOutputMode::Html => Some(render_html_payload(report)),
        CliOutputMode::Json => Some(to_pretty_json(report)),
        CliOutputMode::None => None,
    }
}

pub(crate) fn render_text_payload(report: &ExtractionCommandReport) -> String {
    report
        .matches
        .iter()
        .map(render_match_as_text)
        .collect::<Vec<_>>()
        .join("\n\n")
}

pub(crate) fn render_html_payload(report: &ExtractionCommandReport) -> String {
    report
        .matches
        .iter()
        .map(render_match_as_html)
        .collect::<Vec<_>>()
        .join("\n\n")
}

pub(crate) fn render_match_as_text(matched: &ExtractionMatch) -> String {
    if let Value::String(text) = &matched.value {
        return text.clone();
    }

    serde_json::to_string_pretty(&matched.value)
        .expect("serde_json::Value should always serialize to pretty JSON")
}

pub(crate) fn render_match_as_html(matched: &ExtractionMatch) -> String {
    if let Value::String(html) = &matched.value
        && (matched.value_type == ValueType::InnerHtml
            || matched.value_type == ValueType::OuterHtml)
    {
        return html.clone();
    }

    matched
        .html
        .clone()
        .unwrap_or_else(|| format!("<pre>{}</pre>", escape_html(&render_match_as_text(matched))))
}

pub(crate) fn render_catalog_text(report: &CatalogCommandReport) -> String {
    let mut lines = vec![
        format!("{} {}", report.tool, report.version),
        report.description.clone(),
    ];

    let operation_count = report.operations.len();
    lines.push(format!(
        "Catalog: {operation_count} operation{}.",
        if operation_count == 1 { "" } else { "s" }
    ));
    lines.push(
        "Use `htmlcut catalog --operation <OPERATION_ID> --output json` for one exact contract."
            .to_owned(),
    );

    if report.operations.is_empty() {
        return lines.join("\n");
    }

    lines.push(if report.operations.len() == 1 {
        "Operation:".to_owned()
    } else {
        "Operations:".to_owned()
    });

    for (index, operation) in report.operations.iter().enumerate() {
        if index > 0 {
            lines.push(String::new());
        }
        lines.extend(render_catalog_operation_lines(operation));
    }

    lines.join("\n")
}

fn render_catalog_operation_lines(operation: &crate::model::CatalogOperationReport) -> Vec<String> {
    let mut lines = vec![
        format!(
            "- {} | {}",
            operation.operation_id,
            render_catalog_surface(operation.command.as_deref(), &operation.availability)
        ),
        format!("  {}", operation.summary),
        format!("  core: {}", operation.core_surface),
    ];
    lines.extend(render_catalog_contract_surface_lines(
        "request",
        &operation.request_contract,
    ));
    lines.extend(render_catalog_contract_surface_lines(
        "result",
        &operation.result_contract,
    ));
    if let Some(command_contract) = operation.command_contract.as_ref() {
        lines.extend(render_catalog_contract_lines(command_contract));
    }

    lines
}

fn render_catalog_contract_lines(contract: &CatalogCommandContract) -> Vec<String> {
    let mut lines = vec![format!("  usage: {}", contract.invocation)];

    push_joined_catalog_line(&mut lines, "inputs", &contract.inputs, " | ");
    push_optional_catalog_line(
        &mut lines,
        "default match",
        contract.default_match.as_deref(),
    );
    push_joined_catalog_line(&mut lines, "match modes", &contract.selection_modes, ", ");
    push_optional_catalog_line(
        &mut lines,
        "default value",
        contract.default_value.as_deref(),
    );
    push_joined_catalog_line(&mut lines, "value modes", &contract.value_modes, ", ");
    push_optional_catalog_line(
        &mut lines,
        "default output",
        contract.default_output.as_deref(),
    );
    if !contract.default_output_overrides.is_empty() {
        lines.push("  default output overrides:".to_owned());
        lines.extend(
            contract
                .default_output_overrides
                .iter()
                .map(|override_spec| {
                    format!(
                        "  - when {} => {}",
                        render_catalog_condition(&override_spec.when),
                        override_spec.value
                    )
                }),
        );
    }
    push_joined_catalog_line(&mut lines, "output modes", &contract.output_modes, ", ");
    if !contract.constraints.is_empty() {
        lines.push("  constraints:".to_owned());
        lines.extend(
            contract
                .constraints
                .iter()
                .map(render_catalog_constraint_line),
        );
    }
    if !contract.notes.is_empty() {
        lines.push("  notes:".to_owned());
        lines.extend(contract.notes.iter().map(|note| format!("  - {note}")));
    }
    if !contract.examples.is_empty() {
        lines.push("  examples:".to_owned());
        lines.extend(
            contract
                .examples
                .iter()
                .map(|example| format!("  - {example}")),
        );
    }
    if !contract.parameters.is_empty() {
        lines.push("  parameters:".to_owned());
        for parameter in &contract.parameters {
            lines.push(format!(
                "  - {} | {} {} | {}",
                parameter.section,
                render_parameter_kind(&parameter.kind),
                render_parameter_name(parameter),
                render_parameter_requirement(parameter)
            ));
            lines.push(format!("    {}", parameter.summary));
            if let Some(default) = parameter.default.as_deref() {
                lines.push(format!("    default: {default}"));
            }
            if !parameter.allowed_values.is_empty() {
                lines.push(format!(
                    "    values: {}",
                    parameter.allowed_values.join(", ")
                ));
            }
        }
    }

    lines
}

fn push_joined_catalog_line(
    lines: &mut Vec<String>,
    label: &str,
    values: &[String],
    separator: &str,
) {
    if !values.is_empty() {
        lines.push(format!("  {label}: {}", values.join(separator)));
    }
}

fn push_optional_catalog_line(lines: &mut Vec<String>, label: &str, value: Option<&str>) {
    if let Some(value) = value {
        lines.push(format!("  {label}: {value}"));
    }
}

fn render_catalog_constraint_line(constraint: &CatalogConstraint) -> String {
    match constraint {
        CatalogConstraint::RequiresParameter { parameter, when } => {
            format!(
                "  - requires {parameter} when {}",
                render_catalog_condition(when)
            )
        }
        CatalogConstraint::AllowedOnlyWhen { parameter, when } => format!(
            "  - allows {parameter} only when {}",
            render_catalog_condition(when)
        ),
        CatalogConstraint::RestrictsParameterValues {
            parameter,
            allowed_values,
            when,
        } => format!(
            "  - restricts {parameter} to {} when {}",
            allowed_values.join(", "),
            render_catalog_condition(when)
        ),
    }
}

fn render_catalog_condition(condition: &CatalogCondition) -> String {
    if condition.values.is_empty() {
        return condition.parameter.clone();
    }

    format!(
        "{} is {}",
        condition.parameter,
        condition.values.join(" or ")
    )
}

fn render_catalog_contract_surface_lines(
    label: &str,
    contract: &CatalogContractSurface,
) -> Vec<String> {
    let mut lines = vec![format!("  {label}: {}", contract.rust_shape)];
    if !contract.schema_refs.is_empty() {
        lines.push(format!(
            "  {label} schemas: {}",
            contract
                .schema_refs
                .iter()
                .map(render_schema_ref)
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    lines
}

pub(crate) fn render_schema_text(report: &SchemaCommandReport) -> String {
    let mut lines = vec![
        format!("{} {}", report.tool, report.version),
        report.description.clone(),
        format!("Schema profile: {}", report.schema_profile),
    ];

    let schema_count = report.schemas.len();
    lines.push(format!(
        "Registry: {schema_count} schema{}.",
        if schema_count == 1 { "" } else { "s" }
    ));
    lines.push(
        "Use `htmlcut schema --name <SCHEMA_NAME> --output json` for one schema family.".to_owned(),
    );

    if report.schemas.is_empty() {
        return lines.join("\n");
    }

    let single_schema = report.schemas.len() == 1;
    lines.push(if single_schema {
        "Schema:".to_owned()
    } else {
        "Schemas:".to_owned()
    });

    for schema in &report.schemas {
        lines.push(format!(
            "- {} | {} | {}",
            render_schema_ref(schema),
            schema.owner_surface,
            render_schema_stability(schema.stability)
        ));
        lines.push(format!("  rust: {}", schema.rust_shape));
        if single_schema {
            lines.push(format!(
                "  json schema keys: {}",
                render_json_schema_keys(&schema.json_schema)
            ));
        }
    }

    lines.join("\n")
}

fn render_parameter_kind(kind: &CatalogParameterKind) -> &'static str {
    match kind {
        CatalogParameterKind::Positional => "positional",
        CatalogParameterKind::Option => "option",
        CatalogParameterKind::Flag => "flag",
    }
}

fn render_parameter_name(parameter: &crate::model::CatalogParameterSpec) -> String {
    match parameter.value_hint.as_deref() {
        Some(value_hint) if parameter.kind == CatalogParameterKind::Option => {
            format!("{} <{value_hint}>", parameter.name)
        }
        _ => parameter.name.clone(),
    }
}

fn render_parameter_requirement(parameter: &crate::model::CatalogParameterSpec) -> String {
    match parameter.requirement {
        CatalogParameterRequirement::Required => "required".to_owned(),
        CatalogParameterRequirement::Optional => "optional".to_owned(),
        CatalogParameterRequirement::Conditional => format!(
            "conditional ({})",
            parameter
                .requirement_note
                .as_deref()
                .unwrap_or("see command notes")
        ),
    }
}

fn render_schema_ref(schema: &impl SchemaRefLike) -> String {
    format!("{}@{}", schema.schema_name(), schema.schema_version())
}

fn render_schema_stability(stability: SchemaStability) -> &'static str {
    match stability {
        SchemaStability::Versioned => "versioned",
        SchemaStability::Frozen => "frozen",
    }
}

fn render_json_schema_keys(value: &Value) -> String {
    value
        .as_object()
        .map(|object| object.keys().cloned().collect::<Vec<_>>().join(", "))
        .unwrap_or_else(|| "(not-an-object)".to_owned())
}

trait SchemaRefLike {
    fn schema_name(&self) -> &str;
    fn schema_version(&self) -> u32;
}

impl SchemaRefLike for crate::model::SchemaRefReport {
    fn schema_name(&self) -> &str {
        &self.schema_name
    }

    fn schema_version(&self) -> u32 {
        self.schema_version
    }
}

impl SchemaRefLike for crate::model::SchemaDocumentReport {
    fn schema_name(&self) -> &str {
        &self.schema_name
    }

    fn schema_version(&self) -> u32 {
        self.schema_version
    }
}

pub(crate) fn render_preview_text(report: &ExtractionCommandReport) -> String {
    let mut lines = vec![
        format!("Command: {}", report.command),
        format!(
            "Source: {} {}",
            render_source_kind(&report.source.kind),
            report.source.value
        ),
    ];
    if !report.source.load_steps.is_empty() {
        lines.push("Load trace:".to_owned());
        lines.extend(render_source_load_trace_lines(&report.source));
    }
    lines.push(format!(
        "Candidates: {} | Selected: {} | Duration: {}ms",
        report.stats.candidate_count, report.stats.match_count, report.stats.duration_ms
    ));

    if !report.diagnostics.is_empty() {
        lines.push("Diagnostics:".to_owned());
        lines.extend(report.diagnostics.iter().map(render_diagnostic_line));
    }

    if report.matches.is_empty() {
        return lines.join("\n");
    }

    lines.push("Matches:".to_owned());
    for matched in &report.matches {
        lines.extend(render_preview_match_lines(report.operation_id, matched));
    }

    lines.join("\n")
}

pub(crate) fn render_catalog_surface(
    command: Option<&str>,
    availability: &CatalogAvailability,
) -> String {
    match (command, availability) {
        (Some(command), _) => command.to_owned(),
        (None, CatalogAvailability::CoreOnly) => "core only".to_owned(),
        (None, CatalogAvailability::Cli) => "cli".to_owned(),
    }
}

pub(crate) fn render_preview_match_lines(
    operation_id: htmlcut_core::OperationId,
    matched: &ExtractionMatch,
) -> Vec<String> {
    let mut lines = vec![format!(
        "{}. {}",
        matched.index,
        render_preview_location(operation_id, matched)
    )];

    match operation_id {
        htmlcut_core::OperationId::SelectPreview => {
            if let ExtractionMatchMetadata::Selector(metadata) = &matched.metadata {
                lines.push(format!("   tag: {}", metadata.tag_name));
                if let Some(attributes) = render_attribute_summary(&metadata.attributes) {
                    lines.push(format!("   attributes: {attributes}"));
                }
            }
            if let Some(text) = matched.text.as_deref() {
                lines.push(format!(
                    "   text: {}",
                    compact_inline_preview(text, DEFAULT_PREVIEW_CHARS)
                ));
            } else {
                lines.push(format!("   preview: {}", matched.preview));
            }
        }
        htmlcut_core::OperationId::SlicePreview => {
            if let ExtractionMatchMetadata::DelimiterPair(metadata) = &matched.metadata {
                lines.push(format!("   candidate index: {}", metadata.candidate_index));
                lines.push(format!(
                    "   selected range: {}",
                    render_range_summary(Some(&metadata.selected_range))
                        .expect("selected range should always be present")
                ));
                lines.push(format!(
                    "   inner range: {}",
                    render_range_summary(Some(&metadata.inner_range))
                        .expect("inner range should always be present")
                ));
                lines.push(format!(
                    "   outer range: {}",
                    render_range_summary(Some(&metadata.outer_range))
                        .expect("outer range should always be present")
                ));
                lines.push(format!("   include start: {}", metadata.include_start));
                lines.push(format!("   include end: {}", metadata.include_end));
                lines.push(format!(
                    "   matched start: {}",
                    compact_inline_preview(&metadata.matched_start, DEFAULT_PREVIEW_CHARS)
                ));
                lines.push(format!(
                    "   matched end: {}",
                    compact_inline_preview(&metadata.matched_end, DEFAULT_PREVIEW_CHARS)
                ));
            }
            let text_preview = matched
                .text
                .as_deref()
                .map(|text| compact_inline_preview(text, DEFAULT_PREVIEW_CHARS));
            let fragment_preview = render_fragment_preview(matched.html.as_deref());
            if let Some(fragment) = fragment_preview.as_deref()
                && text_preview.as_deref() != Some(fragment)
            {
                lines.push(format!("   fragment: {fragment}"));
            }
            if let Some(text) = text_preview {
                lines.push(format!("   text: {text}"));
            } else {
                lines.push(format!("   preview: {}", matched.preview));
            }
        }
        _ => {
            lines.push(format!("   preview: {}", matched.preview));
        }
    }

    lines
}

pub(crate) fn render_preview_location(
    operation_id: htmlcut_core::OperationId,
    matched: &ExtractionMatch,
) -> String {
    if let Some(path) = matched.path.as_deref() {
        return path.to_owned();
    }

    if operation_id == htmlcut_core::OperationId::SlicePreview
        && let ExtractionMatchMetadata::DelimiterPair(metadata) = &matched.metadata
    {
        let range = render_range_summary(Some(&metadata.selected_range))
            .expect("selected range should always be present");
        return format!("range {range}");
    }

    "(no path)".to_owned()
}

pub(crate) fn render_attribute_summary(attributes: &BTreeMap<String, String>) -> Option<String> {
    if attributes.is_empty() {
        return None;
    }

    Some(
        attributes
            .iter()
            .map(|(name, value)| format!("{name}={value:?}"))
            .collect::<Vec<_>>()
            .join(", "),
    )
}

pub(crate) fn render_range_summary(range: Option<&Range>) -> Option<String> {
    let range = range?;
    let start = u64::try_from(range.start).ok()?;
    let end = u64::try_from(range.end).ok()?;
    Some(format!("{start}..{end}"))
}

pub(crate) fn compact_inline_preview(input: &str, preview_chars: usize) -> String {
    let normalized = input.split_whitespace().collect::<Vec<_>>().join(" ");
    let char_count = normalized.chars().count();
    if char_count <= preview_chars {
        return normalized;
    }

    let preview = normalized.chars().take(preview_chars).collect::<String>();
    format!("{preview}...")
}

pub(crate) fn render_fragment_preview(fragment: Option<&str>) -> Option<String> {
    let fragment = fragment?;
    (!fragment.is_empty()).then(|| compact_inline_preview(fragment, DEFAULT_PREVIEW_CHARS))
}

pub(crate) fn render_source_inspection_text(
    report: &SourceInspectionCommandReport,
    preview_chars: usize,
) -> String {
    let mut lines = vec![
        format!(
            "Source: {} {}",
            render_source_kind(&report.source.kind),
            report.source.value
        ),
        format!("Bytes: {}", report.source.bytes_read),
    ];

    match (
        report.source.input_base_url.as_deref(),
        report.source.effective_base_url.as_deref(),
    ) {
        (Some(input_base_url), Some(effective_base_url))
            if input_base_url != effective_base_url =>
        {
            lines.push(format!("Input base URL: {input_base_url}"));
            lines.push(format!("Effective base URL: {effective_base_url}"));
        }
        (_, Some(effective_base_url)) => {
            lines.push(format!("Effective base URL: {effective_base_url}"));
        }
        (Some(input_base_url), None) => {
            lines.push(format!("Input base URL: {input_base_url}"));
        }
        (None, None) => {}
    }
    if !report.source.load_steps.is_empty() {
        lines.push("Load trace:".to_owned());
        lines.extend(render_source_load_trace_lines(&report.source));
    }

    if !report.diagnostics.is_empty() {
        lines.push("Diagnostics:".to_owned());
        lines.extend(report.diagnostics.iter().map(render_diagnostic_line));
    }

    let Some(document) = report.document.as_ref() else {
        return lines.join("\n");
    };

    if let Some(title) = document.title.as_deref() {
        lines.push(format!("Title: {title}"));
    }
    lines.push(format!("Root tag: {}", document.root_tag));
    if let Some(document_base_href) = document.document_base_href.as_deref() {
        lines.push(format!("Document <base href>: {document_base_href}"));
        if report.source.effective_base_url.is_none() {
            lines.push("Effective base URL: unresolved".to_owned());
        }
    }
    lines.push(format!(
        "Elements: {} | Text chars: {} | Links: {} | Images: {} | Forms: {} | Tables: {}",
        document.element_count,
        document.text_char_count,
        document.link_count,
        document.image_count,
        document.form_count,
        document.table_count
    ));
    if !document.top_tags.is_empty() {
        lines.push(format!(
            "Top tags: {}",
            render_count_list(&document.top_tags)
        ));
    }
    if !document.top_classes.is_empty() {
        lines.push(format!(
            "Top classes: {}",
            render_count_list(&document.top_classes)
        ));
    }
    if !document.headings.is_empty() {
        lines.push("Headings:".to_owned());
        lines.extend(
            document
                .headings
                .iter()
                .map(|heading| format!("- h{} {} [{}]", heading.level, heading.text, heading.path)),
        );
    }
    if !document.links.is_empty() {
        lines.push("Link previews:".to_owned());
        lines.extend(document.links.iter().map(|link| {
            match (link.href.as_deref(), link.resolved_href.as_deref()) {
                (Some(href), Some(resolved)) if href != resolved => {
                    format!("- {} [{} -> {}] [{}]", link.text, href, resolved, link.path)
                }
                (Some(href), _) => format!("- {} [{}] [{}]", link.text, href, link.path),
                (None, _) => format!("- {} [{}]", link.text, link.path),
            }
        }));
    }
    if let Some(source_text) = report.source.text.as_deref() {
        lines.push("Source text preview:".to_owned());
        lines.push(render_text_preview(source_text, preview_chars));
    }

    lines.join("\n")
}

pub(crate) fn build_verbose_lines(report: &ExtractionCommandReport, verbose: u8) -> Vec<String> {
    if verbose == 0 {
        return Vec::new();
    }

    let noun = if report.stats.match_count == 1 {
        "match"
    } else {
        "matches"
    };
    let mut lines = vec![format!(
        "htmlcut: selected {} {} in {}ms",
        report.stats.match_count, noun, report.stats.duration_ms
    )];
    if verbose > 1 {
        lines.push(format!(
            "htmlcut: scanned {} candidates from {} bytes",
            report.stats.candidate_count, report.source.bytes_read
        ));
        lines.extend(build_source_load_verbose_lines(&report.source));
    }
    lines
}

pub(crate) fn build_source_inspection_verbose_lines(
    report: &SourceInspectionCommandReport,
    verbose: u8,
) -> Vec<String> {
    if verbose == 0 {
        return Vec::new();
    }

    let mut lines = vec![format!(
        "htmlcut: inspected {} bytes from {} source",
        report.source.bytes_read,
        render_source_kind(&report.source.kind)
    )];
    if verbose > 1 {
        lines.extend(build_source_load_verbose_lines(&report.source));
    }
    lines
}

/// Builds stderr lines for non-fatal diagnostics when stdout is reserved for the payload.
pub(crate) fn build_human_diagnostic_stderr_lines(diagnostics: &[Diagnostic]) -> Vec<String> {
    diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.level != DiagnosticLevel::Error)
        .map(|diagnostic| {
            format!(
                "htmlcut: {} {}: {}",
                render_diagnostic_level(diagnostic.level),
                diagnostic.code,
                diagnostic.message
            )
        })
        .collect()
}

pub(crate) fn build_source_load_error_lines(
    source_load_steps: &[htmlcut_core::SourceLoadStep],
) -> Vec<String> {
    if source_load_steps.is_empty() {
        return Vec::new();
    }

    let mut lines = vec!["htmlcut: source load trace:".to_owned()];
    lines.extend(
        source_load_steps
            .iter()
            .map(|step| format!("htmlcut:   {}", render_source_load_step(step))),
    );
    lines
}

pub(crate) fn render_count_list(entries: &[InspectionCount]) -> String {
    entries
        .iter()
        .map(|entry| format!("{} ({})", entry.name, entry.count))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn render_diagnostic_line(diagnostic: &Diagnostic) -> String {
    format!(
        "- {} {}: {}",
        render_diagnostic_level(diagnostic.level),
        diagnostic.code,
        diagnostic.message
    )
}

pub(crate) fn render_diagnostic_level(level: DiagnosticLevel) -> &'static str {
    match level {
        DiagnosticLevel::Error => "error",
        DiagnosticLevel::Warning => "warning",
        DiagnosticLevel::Info => "info",
    }
}

pub(crate) fn render_source_kind(kind: &htmlcut_core::SourceKind) -> &'static str {
    match kind {
        htmlcut_core::SourceKind::Url => "url",
        htmlcut_core::SourceKind::File => "file",
        htmlcut_core::SourceKind::Stdin => "stdin",
        htmlcut_core::SourceKind::Memory => "memory",
    }
}

fn render_source_load_trace_lines(source: &htmlcut_core::SourceMetadata) -> Vec<String> {
    source
        .load_steps
        .iter()
        .map(|step| format!("- {}", render_source_load_step(step)))
        .collect()
}

fn build_source_load_verbose_lines(source: &htmlcut_core::SourceMetadata) -> Vec<String> {
    source
        .load_steps
        .iter()
        .map(|step| format!("htmlcut: source load {}", render_source_load_step(step)))
        .collect()
}

fn render_source_load_step(step: &htmlcut_core::SourceLoadStep) -> String {
    let action = match step.action {
        htmlcut_core::SourceLoadAction::HeadPreflight => "head preflight",
        htmlcut_core::SourceLoadAction::Get => "get",
    };
    let outcome = match step.outcome {
        htmlcut_core::SourceLoadOutcome::Succeeded => "succeeded",
        htmlcut_core::SourceLoadOutcome::Skipped => "skipped",
        htmlcut_core::SourceLoadOutcome::Fallback => "fallback",
        htmlcut_core::SourceLoadOutcome::Failed => "failed",
    };
    let status = step
        .status
        .map(|status| format!(" ({status})"))
        .unwrap_or_default();

    format!("{action} {outcome}{status}: {}", step.message)
}

pub(crate) fn wrap_html_document(report: &ExtractionCommandReport) -> String {
    if report.matches.len() == 1
        && let Some(Value::String(html)) = report.matches.first().map(|matched| &matched.value)
        && looks_like_document(html)
    {
        return html.clone();
    }

    let body = report
        .matches
        .iter()
        .map(|matched| {
            format!(
                "<section data-match-index=\"{}\">{}</section>",
                matched.index,
                render_match_as_html(matched)
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    format!(
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n  <meta charset=\"utf-8\">\n  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n  <title>{}</title>\n  <style>\n    body {{ font-family: ui-serif, Georgia, serif; margin: 2rem auto; max-width: 72rem; padding: 0 1.25rem 3rem; line-height: 1.6; }}\n    section + section {{ border-top: 1px solid #d6d6d6; margin-top: 2rem; padding-top: 2rem; }}\n  </style>\n</head>\n<body>\n{}\n</body>\n</html>\n",
        escape_html(&bundle_document_title(report)),
        body
    )
}

pub(crate) fn looks_like_document(fragment: &str) -> bool {
    let trimmed = fragment.trim_start().to_ascii_lowercase();
    trimmed.starts_with("<!doctype") || trimmed.starts_with("<html")
}

pub(crate) fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub(crate) fn bundle_document_title(report: &ExtractionCommandReport) -> String {
    report
        .document_title
        .clone()
        .filter(|title| !title.trim().is_empty())
        .unwrap_or_else(|| fallback_document_title(&report.source))
}

pub(crate) fn fallback_document_title(source: &htmlcut_core::SourceMetadata) -> String {
    let base_url = source
        .effective_base_url
        .as_deref()
        .or(source.input_base_url.as_deref());

    if let Some(base_url) = base_url {
        let parsed_url = Url::parse(base_url);
        if let Ok(url) = parsed_url {
            return url.host_str().unwrap_or("HTMLCut Selection").to_owned();
        }
    }

    Path::new(&source.value)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("HTMLCut Selection")
        .to_owned()
}

pub(crate) fn render_text_preview(input: &str, preview_chars: usize) -> String {
    let mut preview = String::new();
    for (index, character) in input.chars().enumerate() {
        if index >= preview_chars {
            preview.push_str("...");
            break;
        }
        preview.push(character);
    }

    preview
}

pub(crate) fn to_pretty_json<T: Serialize>(value: &T) -> String {
    serde_json::to_string_pretty(value)
        .expect("HTMLCut CLI reports should always serialize to pretty JSON")
}
