use std::path::Path;

use htmlcut_core::{DiagnosticCode, SourceLoadStep, result::ExtractionMatchMetadata};
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
        "Elements: {} | Body text chars: {} | Links: {} | Images: {} | Forms: {} | Tables: {}",
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
    if document.extraction_candidates == document.reading_candidates {
        if !document.extraction_candidates.is_empty() {
            lines.push("Suggested selectors for extraction and reading:".to_owned());
            lines.extend(document.extraction_candidates.iter().map(|candidate| {
                format!(
                    "- {} | {} chars | {} headings | {} links",
                    candidate.selector,
                    candidate.text_char_count,
                    candidate.heading_count,
                    candidate.link_count
                )
            }));
        }
    } else {
        if !document.extraction_candidates.is_empty() {
            lines.push("Suggested selectors for extraction:".to_owned());
            lines.extend(document.extraction_candidates.iter().map(|candidate| {
                format!(
                    "- {} | {} chars | {} headings | {} links",
                    candidate.selector,
                    candidate.text_char_count,
                    candidate.heading_count,
                    candidate.link_count
                )
            }));
        }
        if !document.reading_candidates.is_empty() {
            lines.push("Suggested selectors for rendered text review:".to_owned());
            lines.extend(document.reading_candidates.iter().map(|candidate| {
                format!(
                    "- {} | {} chars | {} headings | {} links",
                    candidate.selector,
                    candidate.text_char_count,
                    candidate.heading_count,
                    candidate.link_count
                )
            }));
        }
    }
    if !document.headings.is_empty() {
        lines.push("Headings:".to_owned());
        lines.extend(
            document
                .headings
                .iter()
                .map(|heading| format!("- h{} {}", heading.level, heading.text)),
        );
    }
    if !document.links.is_empty() {
        lines.push("Link previews:".to_owned());
        lines.extend(document.links.iter().map(|link| {
            match (link.href.as_deref(), link.resolved_href.as_deref()) {
                (Some(href), Some(resolved)) if href != resolved => {
                    format!("- {} [{} -> {}]", link.text, href, resolved)
                }
                (Some(href), _) => format!("- {} [{}]", link.text, href),
                (None, _) => format!("- {}", link.text),
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

    let mut lines = vec![format!(
        "htmlcut: {} source, {} bytes, {} candidates, {} selected, {}ms",
        render_source_kind(&report.source.kind),
        report.source.bytes_read,
        report.stats.candidate_count,
        report.stats.match_count,
        report.stats.duration_ms
    )];
    if let Some(first_match) = report.matches.first() {
        lines.push(format!(
            "htmlcut: selected {} => {}",
            render_verbose_value_type(first_match.value_type),
            first_match.preview
        ));
        lines.push(format!(
            "htmlcut: selected location {}",
            selected_match_context(first_match)
        ));
    }
    lines.push(format!(
        "htmlcut: effective base {}",
        report
            .source
            .effective_base_url
            .as_deref()
            .or(report.source.input_base_url.as_deref())
            .unwrap_or("(none)")
    ));
    lines.extend(build_source_load_verbose_lines(&report.source));
    if verbose > 1 {
        for matched in report.matches.iter().take(3) {
            lines.push(format!(
                "htmlcut: match {} => {} | {}",
                matched.index,
                selected_match_context(matched),
                matched.preview
            ));
        }
    }
    lines
}

pub(crate) fn build_human_followup_lines(
    report: &ExtractionCommandReport,
    rendered_stdout: Option<&str>,
) -> Vec<String> {
    if report.matches.is_empty() {
        return Vec::new();
    }

    let first = &report.matches[0];
    let mut lines = Vec::new();
    if report
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == DiagnosticCode::MultipleMatches)
    {
        lines.push(format!(
            "htmlcut: selected {} from multiple candidates.",
            selected_match_context(first)
        ));
        lines.push(format!("htmlcut: selected preview: {}", first.preview));
        lines.push(
            "htmlcut: use `inspect select` or `inspect slice` with `--match all` when you want to compare the candidate set before extraction."
                .to_owned(),
        );
    }

    if rendered_stdout.is_some_and(|stdout| stdout.trim().is_empty()) {
        lines.push(format!(
            "htmlcut: the selected match rendered as empty output: {}.",
            selected_match_context(first)
        ));
        lines.push(format!("htmlcut: selected preview: {}", first.preview));
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
    if let Some(document) = report.document.as_ref() {
        if let Some(title) = document.title.as_deref() {
            lines.push(format!("htmlcut: title {title}"));
        }
        if let Some(candidate) = document.extraction_candidates.first() {
            lines.push(format!(
                "htmlcut: extraction top {} | {} chars | {} headings | {} links",
                candidate.selector,
                candidate.text_char_count,
                candidate.heading_count,
                candidate.link_count
            ));
        }
        if let Some(candidate) = document.reading_candidates.first() {
            lines.push(format!(
                "htmlcut: reading top {} | {} chars | {} headings | {} links",
                candidate.selector,
                candidate.text_char_count,
                candidate.heading_count,
                candidate.link_count
            ));
        }
    }
    lines.push(format!(
        "htmlcut: effective base {}",
        report
            .source
            .effective_base_url
            .as_deref()
            .or(report.source.input_base_url.as_deref())
            .unwrap_or("(none)")
    ));
    lines.extend(build_source_load_verbose_lines(&report.source));
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

fn selected_match_context(matched: &htmlcut_core::result::ExtractionMatch) -> String {
    if let Some(path) = matched.path.as_deref() {
        return compact_path_context(path);
    }

    match &matched.metadata {
        ExtractionMatchMetadata::Selector(metadata) => compact_path_context(&metadata.path),
        ExtractionMatchMetadata::DelimiterPair(metadata) => format!(
            "range {}..{}",
            metadata.selected_range.start, metadata.selected_range.end
        ),
    }
}

fn compact_path_context(path: &str) -> String {
    let segments = path.split(" > ").collect::<Vec<_>>();
    if segments.len() <= 4 {
        return path.to_owned();
    }

    format!(
        "... > {} > {} > {}",
        segments[segments.len() - 3],
        segments[segments.len() - 2],
        segments[segments.len() - 1]
    )
}

fn render_verbose_value_type(value_type: htmlcut_core::ValueType) -> &'static str {
    match value_type {
        htmlcut_core::ValueType::Attribute => "attribute",
        htmlcut_core::ValueType::InnerHtml => "inner-html",
        htmlcut_core::ValueType::OuterHtml => "outer-html",
        htmlcut_core::ValueType::SelectedHtml => "selected-html",
        htmlcut_core::ValueType::Structured => "structured",
        htmlcut_core::ValueType::Text => "text",
    }
}

#[cfg(test)]
mod tests;
