use std::collections::BTreeMap;
use std::path::Path;

use htmlcut_core::{
    DEFAULT_PREVIEW_CHARS, Diagnostic, DiagnosticLevel,
    result::{ExtractionMatch, ExtractionMatchMetadata, InspectionCount, Range},
};
use url::Url;

use crate::model::{ExtractionCommandReport, SourceInspectionCommandReport};

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
