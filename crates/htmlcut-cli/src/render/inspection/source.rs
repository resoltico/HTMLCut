use std::path::Path;

use htmlcut_core::SourceLoadStep;
use url::Url;

use crate::model::{ExtractionCommandReport, SourceInspectionCommandReport};

use super::shared::{
    build_source_load_verbose_lines, render_count_list, render_diagnostic_line, render_source_kind,
    render_source_load_step, render_source_load_trace_lines,
};

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

pub(crate) fn build_source_load_error_lines(source_load_steps: &[SourceLoadStep]) -> Vec<String> {
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
