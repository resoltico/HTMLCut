use htmlcut_core::{
    DEFAULT_PREVIEW_CHARS, ValueType,
    result::{ExtractionMatch, ExtractionMatchMetadata},
};

use crate::model::ExtractionCommandReport;
use crate::render::{render_match_as_html, render_match_as_text};

use super::shared::{
    block_text_preview, compact_inline_preview, format_range_summary, render_attribute_summary,
    render_diagnostic_line, render_fragment_preview, render_source_kind,
    render_source_load_trace_lines,
};

const MAX_PROJECTED_OUTPUT_MATCHES: usize = 3;
const MAX_RENDERED_MATCHES: usize = 8;
const MAX_LOCATION_SEGMENTS: usize = 4;
const MAX_LOCATION_CHARS: usize = 96;

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

    if let Some(projected_output) = projected_output_preview(report) {
        lines.push("Projected Output:".to_owned());
        lines.extend(
            block_text_preview(&projected_output, DEFAULT_PREVIEW_CHARS * 2, 10)
                .into_iter()
                .map(|line| format!("  {line}")),
        );
    }

    if report.matches.is_empty() {
        return lines.join("\n");
    }

    let shown_matches = report.matches.len().min(MAX_RENDERED_MATCHES);
    if shown_matches == report.matches.len() {
        lines.push("Matches:".to_owned());
    } else {
        lines.push(format!(
            "Matches: showing first {shown_matches} of {}.",
            report.matches.len()
        ));
    }
    for matched in report.matches.iter().take(MAX_RENDERED_MATCHES) {
        lines.extend(render_preview_match_lines(report.operation_id, matched));
    }
    if shown_matches < report.matches.len() {
        lines.push(
            "Use `--output json` or narrow the selector when you need every match in one report."
                .to_owned(),
        );
    }

    lines.join("\n")
}

fn projected_output_preview(report: &ExtractionCommandReport) -> Option<String> {
    let first = report.matches.first()?;
    let previewable_matches = report
        .matches
        .iter()
        .take(MAX_PROJECTED_OUTPUT_MATCHES)
        .collect::<Vec<_>>();
    let mut preview = match first.value_type {
        ValueType::Structured => None,
        ValueType::InnerHtml | ValueType::OuterHtml | ValueType::SelectedHtml => {
            previewable_matches
                .iter()
                .copied()
                .map(render_match_as_html)
                .collect::<Result<Vec<_>, _>>()
                .ok()
                .map(|parts| parts.join("\n\n"))
        }
        _ => previewable_matches
            .iter()
            .copied()
            .map(render_match_as_text)
            .collect::<Result<Vec<_>, _>>()
            .ok()
            .map(|parts| parts.join("\n\n")),
    }?;

    if report.matches.len() > MAX_PROJECTED_OUTPUT_MATCHES {
        preview.push_str(&format!(
            "\n\n... {} more match(es) omitted from the projected output preview.",
            report.matches.len() - MAX_PROJECTED_OUTPUT_MATCHES
        ));
    }

    Some(preview)
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
                push_text_preview_lines(&mut lines, text);
            } else {
                lines.push(format!("   preview: {}", matched.preview));
            }
        }
        htmlcut_core::OperationId::SlicePreview => {
            if let ExtractionMatchMetadata::DelimiterPair(metadata) = &matched.metadata {
                lines.push(format!(
                    "   candidate {} | selected {} | inner {} | outer {} | retention {}",
                    metadata.candidate_index,
                    format_range_summary(&metadata.selected_range),
                    format_range_summary(&metadata.inner_range),
                    format_range_summary(&metadata.outer_range),
                    render_boundary_retention(metadata.include_start, metadata.include_end),
                ));
                lines.push(format!(
                    "   boundaries: {} … {}",
                    compact_inline_preview(&metadata.matched_start, DEFAULT_PREVIEW_CHARS / 2),
                    compact_inline_preview(&metadata.matched_end, DEFAULT_PREVIEW_CHARS / 2)
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
            if let Some(text) = matched.text.as_deref() {
                push_text_preview_lines(&mut lines, text);
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

fn push_text_preview_lines(lines: &mut Vec<String>, text: &str) {
    let preview_lines = block_text_preview(text, DEFAULT_PREVIEW_CHARS, 6);
    if preview_lines.is_empty() {
        return;
    }

    if preview_lines.len() == 1 {
        lines.push(format!("   text: {}", preview_lines[0]));
        return;
    }

    lines.push("   text:".to_owned());
    lines.extend(preview_lines.into_iter().map(|line| format!("     {line}")));
}

pub(crate) fn render_preview_location(
    operation_id: htmlcut_core::OperationId,
    matched: &ExtractionMatch,
) -> String {
    if let Some(path) = matched.path.as_deref() {
        return compact_location(path);
    }

    if operation_id == htmlcut_core::OperationId::SlicePreview
        && let ExtractionMatchMetadata::DelimiterPair(metadata) = &matched.metadata
    {
        let range = format_range_summary(&metadata.selected_range);
        return format!("range {range}");
    }

    "(no path)".to_owned()
}

fn compact_location(path: &str) -> String {
    let segments = path.split(" > ").collect::<Vec<_>>();
    let compact = if segments.len() > MAX_LOCATION_SEGMENTS {
        format!(
            "... > {}",
            segments[segments.len() - MAX_LOCATION_SEGMENTS..].join(" > ")
        )
    } else {
        path.to_owned()
    };

    if compact.chars().count() <= MAX_LOCATION_CHARS {
        return compact;
    }

    let mut shortened = String::new();
    for (index, character) in compact.chars().enumerate() {
        if index >= MAX_LOCATION_CHARS.saturating_sub(3) {
            shortened.push_str("...");
            break;
        }
        shortened.push(character);
    }

    shortened
}

fn render_boundary_retention(include_start: bool, include_end: bool) -> &'static str {
    match (include_start, include_end) {
        (false, false) => "exclude-both",
        (true, false) => "include-start",
        (false, true) => "include-end",
        (true, true) => "include-both",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use htmlcut_core::{
        ExtractionSpec, OperationId, SourceKind, SourceMetadata, ValueType,
        result::{
            DelimiterPairMatchMetadata, ExtractionMatch, ExtractionMatchMetadata, ExtractionStats,
            Range, SelectorMatchMetadata,
        },
    };
    use serde_json::json;
    use std::collections::BTreeMap;

    fn source_metadata() -> SourceMetadata {
        SourceMetadata {
            kind: SourceKind::Memory,
            value: "<article>Hello</article>".to_owned(),
            input_base_url: None,
            effective_base_url: None,
            bytes_read: 24,
            load_steps: Vec::new(),
            text: None,
        }
    }

    fn selector_match(
        value_type: ValueType,
        html: Option<&str>,
        text: Option<&str>,
    ) -> ExtractionMatch {
        ExtractionMatch {
            index: 1,
            path: Some("article".to_owned()),
            value_type,
            value: json!(html.or(text).unwrap_or("Hello")),
            html: html.map(str::to_owned),
            text: text.map(str::to_owned),
            preview: "Hello".to_owned(),
            metadata: ExtractionMatchMetadata::Selector(SelectorMatchMetadata {
                candidate_count: 1,
                candidate_index: 1,
                path: "article".to_owned(),
                tag_name: "article".to_owned(),
                attributes: BTreeMap::new(),
            }),
        }
    }

    fn extraction_report(matches: Vec<ExtractionMatch>) -> ExtractionCommandReport {
        ExtractionCommandReport {
            tool: "htmlcut".to_owned(),
            engine: "htmlcut-core".to_owned(),
            version: "10.1.0".to_owned(),
            schema_name: "htmlcut.extraction_report".to_owned(),
            schema_version: 6,
            command: "inspect select".to_owned(),
            operation_id: OperationId::SelectPreview,
            ok: true,
            source: source_metadata(),
            extraction: ExtractionSpec::selector(
                htmlcut_core::SelectorQuery::new("article").expect("selector"),
            ),
            stats: ExtractionStats {
                duration_ms: 1,
                candidate_count: 1,
                match_count: matches.len(),
            },
            document_title: None,
            matches,
            diagnostics: Vec::new(),
            bundle: None,
        }
    }

    #[test]
    fn projected_output_preview_joins_html_and_text_modes() {
        let html_report = extraction_report(vec![
            selector_match(
                ValueType::SelectedHtml,
                Some("<article>Alpha</article>"),
                None,
            ),
            selector_match(
                ValueType::SelectedHtml,
                Some("<article>Beta</article>"),
                None,
            ),
        ]);
        assert_eq!(
            projected_output_preview(&html_report).as_deref(),
            Some("<article>Alpha</article>\n\n<article>Beta</article>")
        );

        let text_report = extraction_report(vec![
            selector_match(ValueType::Text, None, Some("Alpha")),
            selector_match(ValueType::Text, None, Some("Beta")),
        ]);
        assert_eq!(
            projected_output_preview(&text_report).as_deref(),
            Some("Alpha\n\nBeta")
        );

        let structured_report =
            extraction_report(vec![selector_match(ValueType::Structured, None, None)]);
        assert_eq!(projected_output_preview(&structured_report), None);
    }

    #[test]
    fn render_preview_location_falls_back_to_slice_ranges() {
        let matched = ExtractionMatch {
            index: 1,
            path: None,
            value_type: ValueType::Text,
            value: json!("Alpha"),
            html: None,
            text: Some("Alpha".to_owned()),
            preview: "Alpha".to_owned(),
            metadata: ExtractionMatchMetadata::DelimiterPair(DelimiterPairMatchMetadata {
                candidate_count: 1,
                candidate_index: 1,
                selected_range: Range { start: 5, end: 10 },
                inner_range: Range { start: 6, end: 9 },
                outer_range: Range { start: 4, end: 11 },
                include_start: false,
                include_end: false,
                matched_start: "START".to_owned(),
                matched_end: "END".to_owned(),
            }),
        };
        assert_eq!(
            render_preview_location(OperationId::SlicePreview, &matched),
            "range 5..10"
        );
    }

    #[test]
    fn preview_rendering_helpers_cover_truncation_and_retention_labels() {
        let mut long_path_match = selector_match(ValueType::Text, None, Some("Alpha"));
        long_path_match.path = Some(
            "html > body > main:nth-of-type(1) > article:nth-of-type(1) > section:nth-of-type(1) > div[data-section=\"primary-feature-surface\"][data-variant=\"comparison-rail\"] > ul[data-list=\"deep-navigation-links\"] > li[data-entry=\"international-purchasing-and-delivery-details\"] > a[data-tracking-id=\"primary-cta\"][aria-label=\"Open the full international purchasing and delivery details article\"]"
                .to_owned(),
        );
        let compact = render_preview_location(OperationId::SelectPreview, &long_path_match);
        assert!(compact.starts_with("... > "));
        assert!(compact.ends_with("..."));

        let overflow_report = extraction_report(
            (1..=MAX_RENDERED_MATCHES + 1)
                .map(|index| {
                    let mut matched = selector_match(ValueType::Text, None, Some("Alpha"));
                    matched.index = index;
                    matched
                })
                .collect(),
        );
        let overflow_text = render_preview_text(&overflow_report);
        assert!(overflow_text.contains(&format!(
            "Matches: showing first {} of {}.",
            MAX_RENDERED_MATCHES,
            MAX_RENDERED_MATCHES + 1
        )));
        assert!(overflow_text.contains("Use `--output json` or narrow the selector"));

        let projected_overflow = extraction_report(
            (1..=MAX_PROJECTED_OUTPUT_MATCHES + 1)
                .map(|index| {
                    let mut matched = selector_match(
                        ValueType::SelectedHtml,
                        Some(&format!("<article>{index}</article>")),
                        None,
                    );
                    matched.index = index;
                    matched
                })
                .collect(),
        );
        assert!(
            projected_output_preview(&projected_overflow)
                .expect("projected preview")
                .contains("more match(es) omitted")
        );

        assert_eq!(render_boundary_retention(true, false), "include-start");
        assert_eq!(render_boundary_retention(false, true), "include-end");
        assert_eq!(render_boundary_retention(true, true), "include-both");
    }
}
