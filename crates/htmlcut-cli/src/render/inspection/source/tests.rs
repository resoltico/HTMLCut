use super::*;
use htmlcut_core::{
    Diagnostic, DiagnosticLevel, ExtractionSpec, OperationId, SourceKind, SourceLoadAction,
    SourceLoadOutcome, SourceMetadata, ValueType,
    result::{
        ContentCandidateInspection, DelimiterPairMatchMetadata, DocumentInspection,
        ExtractionMatch, ExtractionMatchMetadata, ExtractionStats, Range, SelectorMatchMetadata,
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
        version: "10.2.0".to_owned(),
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
        version: "10.2.0".to_owned(),
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
