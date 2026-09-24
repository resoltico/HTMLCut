use super::*;

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
        schema_name: CORE_RESULT_SCHEMA_NAME.to_owned(),
        schema_version: CORE_RESULT_SCHEMA_VERSION,
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
    let roundtrip: ExtractionResult = ExtractionResultDocument::try_from(result.clone())
        .expect("result document")
        .try_into()
        .expect("current extraction result document");
    assert_eq!(roundtrip, result);
    assert!(
        ExtractionResult::try_from(tamper_wire_profile(
            ExtractionResultDocument::try_from(roundtrip).expect("result document"),
        ))
        .is_err()
    );

    let inspection = SourceInspectionResult {
        operation_id: OperationId::SourceInspect,
        schema_name: CORE_SOURCE_INSPECTION_SCHEMA_NAME.to_owned(),
        schema_version: CORE_SOURCE_INSPECTION_SCHEMA_VERSION,
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
        SourceInspectionResultDocument::try_from(inspection.clone())
            .expect("source inspection document")
            .try_into()
            .expect("current source inspection result document");
    assert_eq!(inspection_roundtrip, inspection);
    assert!(
        SourceInspectionResult::try_from(tamper_wire_profile(
            SourceInspectionResultDocument::try_from(inspection_roundtrip)
                .expect("inspection document"),
        ))
        .is_err()
    );
}
