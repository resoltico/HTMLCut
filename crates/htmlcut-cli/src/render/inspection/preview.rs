use htmlcut_core::{
    DEFAULT_PREVIEW_CHARS,
    result::{ExtractionMatch, ExtractionMatchMetadata},
};

use crate::model::ExtractionCommandReport;

use super::shared::{
    compact_inline_preview, format_range_summary, render_attribute_summary, render_diagnostic_line,
    render_fragment_preview, render_source_kind, render_source_load_trace_lines,
};

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
                    format_range_summary(&metadata.selected_range)
                ));
                lines.push(format!(
                    "   inner range: {}",
                    format_range_summary(&metadata.inner_range)
                ));
                lines.push(format!(
                    "   outer range: {}",
                    format_range_summary(&metadata.outer_range)
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
        let range = format_range_summary(&metadata.selected_range);
        return format!("range {range}");
    }

    "(no path)".to_owned()
}
