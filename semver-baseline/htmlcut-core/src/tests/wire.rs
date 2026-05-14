use super::*;
use crate::result::{
    ContentCandidateInspection, DelimiterPairMatchMetadata, DocumentInspection, ExtractionMatch,
    ExtractionStats, HeadingInspection, InspectionCount, LinkInspection, Range,
    SelectorMatchMetadata,
};
use crate::wire::v1::{
    ExtractionDefinitionDocument, ExtractionRequestDocument, ExtractionResultDocument,
    InspectionOptionsDocument, RuntimeOptionsDocument, SourceInspectionResultDocument,
    SourceRequestDocument,
};
use std::collections::BTreeMap;

#[test]
fn wire_default_documents_match_domain_defaults() {
    let runtime: RuntimeOptions = serde_json::from_value::<RuntimeOptionsDocument>(json!({}))
        .expect("runtime doc")
        .into();
    assert_eq!(runtime, RuntimeOptions::default());

    let runtime_default: RuntimeOptions = RuntimeOptionsDocument::default().into();
    assert_eq!(runtime_default, RuntimeOptions::default());

    let inspection: InspectionOptions =
        serde_json::from_value::<InspectionOptionsDocument>(json!({}))
            .expect("inspection doc")
            .into();
    assert_eq!(inspection, InspectionOptions::default());

    let inspection_default: InspectionOptions = InspectionOptionsDocument::default().into();
    assert_eq!(inspection_default, InspectionOptions::default());

    let output_request: ExtractionRequest =
        serde_json::from_value::<ExtractionRequestDocument>(json!({
            "spec_version": CORE_SPEC_VERSION,
            "source": { "input": { "type": "stdin" } },
            "extraction": { "kind": "selector", "selector": "article" }
        }))
        .expect("request doc with default output")
        .into();
    assert_eq!(output_request.output, OutputOptions::default());
}

#[test]
fn wire_request_documents_round_trip_all_source_and_extraction_variants() {
    let sources = vec![
        SourceRequest::stdin(),
        SourceRequest::file("page.html"),
        SourceRequest::memory("inline", "<article>Hello</article>"),
        SourceRequest::url(http_url("https://example.com/input.html")),
    ];

    for source in sources {
        let roundtrip: SourceRequest = SourceRequestDocument::try_from(source.clone())
            .expect("source document")
            .into();
        assert_eq!(roundtrip, source);
    }

    let requests = vec![
        ExtractionRequest::new(
            SourceRequest::stdin(),
            ExtractionSpec::selector(selector_query("article"))
                .with_selection(SelectionSpec::single())
                .with_value(ValueSpec::Text),
        ),
        ExtractionRequest::new(
            SourceRequest::file("page.html"),
            ExtractionSpec::selector(selector_query("article"))
                .with_selection(SelectionSpec::default())
                .with_value(ValueSpec::OuterHtml),
        ),
        ExtractionRequest::new(
            SourceRequest::memory("inline", "<article>Hello</article>"),
            ExtractionSpec::selector(selector_query("article"))
                .with_selection(nth_selection(2))
                .with_value(attribute_value("HREF")),
        ),
        ExtractionRequest::new(
            memory_source("inline", "<article>Hello</article>"),
            ExtractionSpec::selector(selector_query("article"))
                .with_selection(SelectionSpec::All)
                .with_value(ValueSpec::Structured),
        ),
        ExtractionRequest::new(
            memory_source("inline", "<article>Hello</article>"),
            ExtractionSpec::slice(
                slice_spec("<article>", "</article>")
                    .with_boundary_retention(BoundaryRetention::IncludeBoth),
            )
            .with_selection(SelectionSpec::single())
            .with_value(ValueSpec::SelectedHtml),
        ),
        ExtractionRequest::new(
            memory_source("inline", "<article>Hello</article>"),
            ExtractionSpec::slice(
                SliceSpec::regex(
                    slice_boundary("<article>"),
                    slice_boundary("</article>"),
                    "is",
                )
                .with_boundary_retention(BoundaryRetention::IncludeStart),
            )
            .with_selection(SelectionSpec::First)
            .with_value(ValueSpec::InnerHtml),
        ),
        ExtractionRequest::new(
            memory_source("inline", "<article>Hello</article>"),
            ExtractionSpec::slice(
                slice_spec("<article>", "</article>")
                    .with_boundary_retention(BoundaryRetention::IncludeEnd),
            )
            .with_selection(SelectionSpec::All)
            .with_value(ValueSpec::OuterHtml),
        ),
        ExtractionRequest::new(
            memory_source("inline", "<article data-id=\"7\">Hello</article>"),
            ExtractionSpec::slice(
                slice_spec("<article", "</article>")
                    .with_boundary_retention(BoundaryRetention::ExcludeBoth),
            )
            .with_selection(nth_selection(1))
            .with_value(attribute_value("data-id")),
        ),
    ];

    for request in requests {
        let roundtrip: ExtractionRequest = ExtractionRequestDocument::try_from(request.clone())
            .expect("request document")
            .into();
        assert_eq!(roundtrip, request);
    }

    let definition = ExtractionDefinition {
        schema_name: EXTRACTION_DEFINITION_SCHEMA_NAME.to_owned(),
        schema_version: EXTRACTION_DEFINITION_SCHEMA_VERSION,
        request: ExtractionRequest::new(
            SourceRequest::memory("inline", "<article>Hello</article>")
                .with_base_url(http_url("https://example.com/base/")),
            ExtractionSpec::selector(selector_query("article"))
                .with_selection(SelectionSpec::single())
                .with_value(ValueSpec::Structured),
        ),
        runtime: RuntimeOptions {
            max_bytes: max_bytes_limit(2048),
            fetch_timeout_ms: fetch_timeout_limit(1500),
            fetch_connect_timeout_ms: FetchConnectTimeoutMs::new(250).expect("connect timeout"),
            fetch_preflight: FetchPreflightMode::GetOnly,
            tls_trust: TlsTrustPolicy::Platform,
        },
    };
    let roundtrip: ExtractionDefinition =
        ExtractionDefinitionDocument::try_from(definition.clone())
            .expect("definition document")
            .into();
    assert_eq!(roundtrip, definition);

    let runtime = RuntimeOptions {
        max_bytes: max_bytes_limit(4096),
        fetch_timeout_ms: fetch_timeout_limit(2750),
        fetch_connect_timeout_ms: FetchConnectTimeoutMs::new(750).expect("connect timeout"),
        fetch_preflight: FetchPreflightMode::HeadFirst,
        tls_trust: TlsTrustPolicy::CustomCaBundle {
            path: "certs/custom.pem".into(),
        },
    };
    let runtime_roundtrip: RuntimeOptions = RuntimeOptionsDocument::from(runtime.clone()).into();
    assert_eq!(runtime_roundtrip, runtime);

    let inspection_options = InspectionOptions {
        include_source_text: true,
        sample_limit: 7,
    };
    let inspection_roundtrip: InspectionOptions =
        InspectionOptionsDocument::from(inspection_options.clone()).into();
    assert_eq!(inspection_roundtrip, inspection_options);
}

#[test]
fn wire_result_documents_round_trip_nested_payloads() {
    let source = SourceMetadata {
        kind: SourceKind::Url,
        value: "https://example.com/docs/page.html?[redacted]".to_owned(),
        input_base_url: Some("https://example.com/docs/page.html?[redacted]".to_owned()),
        effective_base_url: Some("https://example.com/base/".to_owned()),
        bytes_read: 128,
        load_steps: vec![
            SourceLoadStep {
                action: SourceLoadAction::HeadPreflight,
                outcome: SourceLoadOutcome::Fallback,
                status: Some(405),
                message: "HEAD fell back to GET.".to_owned(),
            },
            SourceLoadStep {
                action: SourceLoadAction::Get,
                outcome: SourceLoadOutcome::Succeeded,
                status: Some(200),
                message: "Fetched with GET.".to_owned(),
            },
        ],
        text: Some("<article>Hello</article>".to_owned()),
    };

    let selector_match = ExtractionMatch {
        index: 1,
        path: Some("html > body > article".to_owned()),
        value_type: ValueType::Structured,
        value: json!({
            "tagName": "article",
            "matchIndex": 1,
        }),
        html: Some("<article>Hello</article>".to_owned()),
        text: Some("Hello".to_owned()),
        preview: "Hello".to_owned(),
        metadata: ExtractionMatchMetadata::Selector(SelectorMatchMetadata {
            candidate_count: 2,
            candidate_index: 1,
            path: "html > body > article".to_owned(),
            tag_name: "article".to_owned(),
            attributes: BTreeMap::from([
                (
                    "href".to_owned(),
                    "https://example.com/base/guide".to_owned(),
                ),
                ("class".to_owned(), "hero".to_owned()),
            ]),
        }),
    };
    let slice_match = ExtractionMatch {
        index: 2,
        path: None,
        value_type: ValueType::SelectedHtml,
        value: Value::String("<article>Hello</article>".to_owned()),
        html: Some("<article>Hello</article>".to_owned()),
        text: Some("Hello".to_owned()),
        preview: "<article>Hello</article>".to_owned(),
        metadata: ExtractionMatchMetadata::DelimiterPair(DelimiterPairMatchMetadata {
            candidate_count: 3,
            candidate_index: 2,
            selected_range: Range { start: 10, end: 34 },
            inner_range: Range { start: 19, end: 24 },
            outer_range: Range { start: 10, end: 34 },
            include_start: true,
            include_end: true,
            matched_start: "<article>".to_owned(),
            matched_end: "</article>".to_owned(),
        }),
    };
    let result = ExtractionResult {
        operation_id: OperationId::SliceExtract,
        schema_name: "htmlcut.extraction_result".to_owned(),
        schema_version: 2,
        ok: false,
        source: source.clone(),
        document_title: Some("Guide".to_owned()),
        extraction: ExtractionSpec::slice(
            slice_spec("<article>", "</article>")
                .with_boundary_retention(BoundaryRetention::IncludeBoth),
        )
        .with_selection(SelectionSpec::All)
        .with_value(ValueSpec::SelectedHtml),
        stats: ExtractionStats {
            duration_ms: 42,
            candidate_count: 3,
            match_count: 2,
        },
        matches: vec![selector_match, slice_match],
        diagnostics: vec![
            Diagnostic {
                level: DiagnosticLevel::Warning,
                code: DiagnosticCode::EffectiveBaseUrlUnresolved,
                message: "Base URL could not be resolved.".to_owned(),
                details: Some(json!({ "source": "inline" })),
            },
            Diagnostic {
                level: DiagnosticLevel::Error,
                code: DiagnosticCode::UnsupportedValueType,
                message: "selected-html is only valid for slice extraction.".to_owned(),
                details: Some(json!({ "strategy": "selector" })),
            },
        ],
    };
    let roundtrip: ExtractionResult = ExtractionResultDocument::from(result.clone()).into();
    assert_eq!(roundtrip, result);

    let inspection = SourceInspectionResult {
        operation_id: OperationId::SourceInspect,
        schema_name: "htmlcut.source_inspection_result".to_owned(),
        schema_version: 2,
        ok: true,
        source,
        document: Some(DocumentInspection {
            title: Some("Guide".to_owned()),
            root_tag: "body".to_owned(),
            element_count: 10,
            text_char_count: 55,
            link_count: 3,
            image_count: 1,
            form_count: 1,
            table_count: 0,
            script_count: 0,
            style_count: 1,
            document_base_href: Some("https://example.com/base/".to_owned()),
            top_tags: vec![InspectionCount {
                name: "article".to_owned(),
                count: 1,
            }],
            top_classes: vec![InspectionCount {
                name: "hero".to_owned(),
                count: 1,
            }],
            extraction_candidates: vec![ContentCandidateInspection {
                selector: "article.hero".to_owned(),
                path: "html > body > article.hero".to_owned(),
                tag_name: "article".to_owned(),
                text_char_count: 55,
                heading_count: 1,
                link_count: 3,
            }],
            reading_candidates: vec![ContentCandidateInspection {
                selector: "main".to_owned(),
                path: "html > body > main".to_owned(),
                tag_name: "main".to_owned(),
                text_char_count: 55,
                heading_count: 2,
                link_count: 3,
            }],
            headings: vec![HeadingInspection {
                level: 2,
                text: "Guide".to_owned(),
                path: "html > body > article > h2".to_owned(),
            }],
            links: vec![LinkInspection {
                text: "Guide".to_owned(),
                href: Some("/guide".to_owned()),
                resolved_href: Some("https://example.com/guide".to_owned()),
                path: "html > body > article > a".to_owned(),
            }],
        }),
        diagnostics: vec![Diagnostic {
            level: DiagnosticLevel::Info,
            code: DiagnosticCode::MultipleMatches,
            message: "Multiple candidates were available.".to_owned(),
            details: None,
        }],
    };
    let inspection_roundtrip: SourceInspectionResult =
        SourceInspectionResultDocument::from(inspection.clone()).into();
    assert_eq!(inspection_roundtrip, inspection);
}
