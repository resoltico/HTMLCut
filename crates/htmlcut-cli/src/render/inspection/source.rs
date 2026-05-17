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
mod tests {
    use super::*;
    use htmlcut_core::{
        Diagnostic, DiagnosticLevel, ExtractionSpec, OperationId, SourceKind, SourceLoadAction,
        SourceLoadOutcome, SourceMetadata, ValueType,
        result::{
            ContentCandidateInspection, DelimiterPairMatchMetadata, DocumentInspection,
            ExtractionMatch, ExtractionMatchMetadata, ExtractionStats, Range,
            SelectorMatchMetadata,
        },
    };
    use serde_json::json;
    use std::collections::BTreeMap;

    fn source_metadata() -> SourceMetadata {
        SourceMetadata {
            kind: SourceKind::Url,
            value: "fixture.html".to_owned(),
            input_base_url: Some("https://example.test/input".to_owned()),
            effective_base_url: Some("https://example.test/effective".to_owned()),
            bytes_read: 128,
            load_steps: vec![SourceLoadStep {
                action: SourceLoadAction::HeadPreflight,
                outcome: SourceLoadOutcome::Succeeded,
                status: Some(200),
                message: "HEAD ok".to_owned(),
            }],
            text: None,
        }
    }

    fn selector_match_with(value_type: ValueType) -> ExtractionMatch {
        ExtractionMatch {
            index: 1,
            path: None,
            value_type,
            value: json!("Alpha"),
            html: Some("<article>Alpha</article>".to_owned()),
            text: Some("Alpha".to_owned()),
            preview: "Alpha".to_owned(),
            metadata: ExtractionMatchMetadata::Selector(SelectorMatchMetadata {
                candidate_count: 2,
                candidate_index: 1,
                path: "article.main".to_owned(),
                tag_name: "article".to_owned(),
                attributes: BTreeMap::new(),
            }),
        }
    }

    fn slice_match() -> ExtractionMatch {
        ExtractionMatch {
            index: 1,
            path: None,
            value_type: ValueType::Text,
            value: json!("Alpha"),
            html: Some("START Alpha END".to_owned()),
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
        }
    }

    fn extraction_report(matches: Vec<ExtractionMatch>) -> ExtractionCommandReport {
        ExtractionCommandReport {
            tool: "htmlcut".to_owned(),
            engine: "htmlcut-core".to_owned(),
            version: "10.1.0".to_owned(),
            schema_name: "htmlcut.extraction_report".to_owned(),
            schema_version: 6,
            command: "select".to_owned(),
            operation_id: OperationId::SelectExtract,
            ok: true,
            source: source_metadata(),
            extraction: ExtractionSpec::selector(
                htmlcut_core::SelectorQuery::new("article").expect("selector"),
            ),
            stats: ExtractionStats {
                duration_ms: 2,
                candidate_count: 2,
                match_count: matches.len(),
            },
            document_title: Some("Fixture".to_owned()),
            matches,
            diagnostics: vec![Diagnostic {
                level: DiagnosticLevel::Warning,
                code: DiagnosticCode::MultipleMatches,
                message: "multiple".to_owned(),
                details: None,
            }],
            bundle: None,
        }
    }

    #[test]
    fn verbose_and_followup_helpers_cover_optional_paths() {
        let empty = extraction_report(Vec::new());
        let empty_verbose = build_verbose_lines(&empty, 1);
        assert!(
            empty_verbose
                .iter()
                .any(|line| line.contains("effective base"))
        );
        assert!(
            !empty_verbose
                .iter()
                .any(|line| line.starts_with("htmlcut: selected "))
        );
        assert!(build_human_followup_lines(&empty, Some("Alpha")).is_empty());

        let report = extraction_report(vec![selector_match_with(ValueType::SelectedHtml)]);
        let followup = build_human_followup_lines(&report, Some("   "));
        assert!(
            followup
                .iter()
                .any(|line| line.contains("selected match rendered as empty output"))
        );
        assert!(
            followup
                .iter()
                .any(|line| line.contains("selected preview: Alpha"))
        );

        let verbose = build_verbose_lines(&report, 2);
        assert!(
            verbose
                .iter()
                .any(|line| line.contains("selected selected-html"))
        );
        assert!(verbose.iter().any(|line| line.contains("article.main")));
        assert!(verbose.iter().any(|line| line.contains("HEAD ok")));
    }

    #[test]
    fn source_verbose_and_context_helpers_cover_selector_slice_and_fallbacks() {
        let report = SourceInspectionCommandReport {
            tool: "htmlcut".to_owned(),
            engine: "htmlcut-core".to_owned(),
            version: "10.1.0".to_owned(),
            schema_name: "htmlcut.source_inspection_report".to_owned(),
            schema_version: 5,
            command: "inspect source".to_owned(),
            operation_id: OperationId::SourceInspect,
            ok: true,
            source: source_metadata(),
            document: Some(DocumentInspection {
                title: Some("Fixture".to_owned()),
                root_tag: "html".to_owned(),
                element_count: 10,
                text_char_count: 20,
                link_count: 3,
                image_count: 0,
                form_count: 0,
                table_count: 0,
                script_count: 0,
                style_count: 0,
                document_base_href: None,
                top_tags: Vec::new(),
                top_classes: Vec::new(),
                extraction_candidates: vec![ContentCandidateInspection {
                    selector: "#main".to_owned(),
                    path: "html > body > #main".to_owned(),
                    tag_name: "main".to_owned(),
                    text_char_count: 20,
                    heading_count: 1,
                    link_count: 3,
                }],
                reading_candidates: vec![ContentCandidateInspection {
                    selector: "article".to_owned(),
                    path: "html > body > article".to_owned(),
                    tag_name: "article".to_owned(),
                    text_char_count: 18,
                    heading_count: 1,
                    link_count: 1,
                }],
                headings: Vec::new(),
                links: Vec::new(),
            }),
            diagnostics: Vec::new(),
        };
        let verbose = build_source_inspection_verbose_lines(&report, 1);
        assert!(verbose.iter().any(|line| line.contains("title Fixture")));
        assert!(
            verbose
                .iter()
                .any(|line| line.contains("extraction top #main"))
        );
        assert!(
            verbose
                .iter()
                .any(|line| line.contains("reading top article"))
        );

        let sparse_document_verbose = build_source_inspection_verbose_lines(
            &SourceInspectionCommandReport {
                document: Some(DocumentInspection {
                    title: None,
                    extraction_candidates: Vec::new(),
                    reading_candidates: Vec::new(),
                    ..report.document.clone().expect("document")
                }),
                ..report.clone()
            },
            1,
        );
        assert!(
            !sparse_document_verbose
                .iter()
                .any(|line| line.contains("title "))
        );
        assert!(
            !sparse_document_verbose
                .iter()
                .any(|line| line.contains("extraction top"))
        );
        assert!(
            !sparse_document_verbose
                .iter()
                .any(|line| line.contains("reading top"))
        );

        let no_doc_verbose = build_source_inspection_verbose_lines(
            &SourceInspectionCommandReport {
                document: None,
                ..report.clone()
            },
            1,
        );
        assert!(!no_doc_verbose.iter().any(|line| line.contains("title")));

        assert_eq!(
            selected_match_context(&selector_match_with(ValueType::Attribute)),
            "article.main"
        );
        assert_eq!(selected_match_context(&slice_match()), "range 5..10");
        assert_eq!(render_verbose_value_type(ValueType::Attribute), "attribute");
        assert_eq!(
            render_verbose_value_type(ValueType::InnerHtml),
            "inner-html"
        );
        assert_eq!(
            render_verbose_value_type(ValueType::OuterHtml),
            "outer-html"
        );
        assert_eq!(
            render_verbose_value_type(ValueType::SelectedHtml),
            "selected-html"
        );
        assert_eq!(
            render_verbose_value_type(ValueType::Structured),
            "structured"
        );
        assert_eq!(render_verbose_value_type(ValueType::Text), "text");
        assert_eq!(fallback_document_title(&source_metadata()), "example.test");
        assert_eq!(
            fallback_document_title(&SourceMetadata {
                effective_base_url: None,
                input_base_url: None,
                value: "/tmp/report.html".to_owned(),
                ..source_metadata()
            }),
            "report"
        );
        assert_eq!(
            compact_path_context(
                "html > body > main:nth-of-type(1) > article:nth-of-type(2) > p:nth-of-type(3)"
            ),
            "... > main:nth-of-type(1) > article:nth-of-type(2) > p:nth-of-type(3)"
        );
    }
}
