use super::*;

#[test]
fn render_preview_and_source_inspection_text_are_human_readable() {
    let mut preview = build_extraction_report(
        "inspect-select",
        fixture_result(
            serde_json::json!({"tagName":"article"}),
            ValueType::Structured,
        ),
        None,
    );
    preview.operation_id = htmlcut_core::OperationId::SelectPreview;
    let preview_text = render_preview_text(&preview);
    assert!(preview_text.contains("Command: inspect-select"));
    assert!(preview_text.contains("Matches:"));
    assert!(preview_text.contains("tag: article"));
    assert!(preview_text.contains("text: Hello"));

    let mut slice_preview = build_extraction_report(
        "inspect-slice",
        fixture_result(
            serde_json::json!({"range":{"start":1,"end":18}}),
            ValueType::Structured,
        ),
        None,
    );
    slice_preview.operation_id = htmlcut_core::OperationId::SlicePreview;
    slice_preview.matches[0].path = None;
    slice_preview.matches[0].html = Some("<article>Hello</article>".to_owned());
    slice_preview.matches[0].text = Some("Hello".to_owned());
    slice_preview.matches[0].metadata =
        delimiter_metadata(1, 1, (1, 24), (10, 15), (1, 24), true, true);
    let slice_preview_text = render_preview_text(&slice_preview);
    assert!(slice_preview_text.contains("fragment: <article>Hello</article>"));
    assert!(slice_preview_text.contains("text: Hello"));
    assert!(slice_preview_text.contains("retention include-both"));
    assert!(slice_preview_text.contains("boundaries: <article> … </article>"));

    let mut inspection = fixture_inspection();
    inspection.source.load_steps = vec![
        SourceLoadStep {
            action: SourceLoadAction::HeadPreflight,
            outcome: SourceLoadOutcome::Fallback,
            status: Some(405),
            message: "HEAD returned 405, so HTMLCut fell back to GET.".to_owned(),
        },
        SourceLoadStep {
            action: SourceLoadAction::Get,
            outcome: SourceLoadOutcome::Succeeded,
            status: Some(200),
            message: "Fetched the remote source with GET.".to_owned(),
        },
    ];
    let inspection_text = render_source_inspection_text(&inspection, DEFAULT_PREVIEW_CHARS);
    assert!(inspection_text.contains("Top tags: a (2)"));
    assert!(inspection_text.contains("Suggested selectors for extraction:"));
    assert!(inspection_text.contains("Suggested selectors for rendered text review:"));
    assert!(inspection_text.contains("Link previews:"));
    assert!(inspection_text.contains("Document <base href>: ../content/"));
    assert!(inspection_text.contains("Load trace:"));
    assert!(inspection_text.contains("head preflight fallback (405)"));
    assert!(inspection_text.contains("get succeeded (200)"));

    let mut extraction_only = fixture_inspection();
    extraction_only
        .document
        .as_mut()
        .expect("document")
        .reading_candidates
        .clear();
    let extraction_only_text =
        render_source_inspection_text(&extraction_only, DEFAULT_PREVIEW_CHARS);
    assert!(extraction_only_text.contains("Suggested selectors for extraction:"));
    assert!(!extraction_only_text.contains("Suggested selectors for extraction and reading:"));
    assert!(!extraction_only_text.contains("Suggested selectors for rendered text review:"));

    let mut reading_only = fixture_inspection();
    reading_only
        .document
        .as_mut()
        .expect("document")
        .extraction_candidates
        .clear();
    let reading_only_text = render_source_inspection_text(&reading_only, DEFAULT_PREVIEW_CHARS);
    assert!(!reading_only_text.contains("Suggested selectors for extraction and reading:"));
    assert!(!reading_only_text.contains("Suggested selectors for extraction:"));
    assert!(reading_only_text.contains("Suggested selectors for rendered text review:"));

    let mut untitled = fixture_inspection();
    untitled.source.input_base_url = None;
    untitled.source.effective_base_url = None;
    let document = untitled.document.as_mut().expect("document");
    document.title = None;
    document.document_base_href = None;
    document.top_tags.clear();
    document.top_classes.clear();
    document.extraction_candidates.clear();
    document.reading_candidates.clear();
    document.headings.clear();
    document.links.clear();
    let untitled_text = render_source_inspection_text(&untitled, DEFAULT_PREVIEW_CHARS);
    assert!(!untitled_text.contains("Input base URL:"));
    assert!(!untitled_text.contains("Effective base URL:"));
    assert!(!untitled_text.contains("Title:"));
    assert!(!untitled_text.contains("Document <base href>:"));
    assert!(!untitled_text.contains("Top tags:"));
    assert!(!untitled_text.contains("Top classes:"));
    assert!(!untitled_text.contains("Suggested selectors for extraction:"));
    assert!(!untitled_text.contains("Suggested selectors for rendered text review:"));
    assert!(!untitled_text.contains("Headings:"));
    assert!(!untitled_text.contains("Link previews:"));
}

#[test]
fn wrap_html_document_and_match_renderers_cover_remaining_paths() {
    let report = build_extraction_report(
        "select",
        fixture_result(
            Value::String("<!DOCTYPE html><html><body>Hello</body></html>".to_owned()),
            ValueType::OuterHtml,
        ),
        None,
    );
    assert!(
        wrap_html_document(&report)
            .expect("wrapped html document")
            .starts_with("<!DOCTYPE html>")
    );

    let json_match = ExtractionMatch {
        index: 1,
        path: None,
        value_type: ValueType::Structured,
        value: serde_json::json!({"hello":"world"}),
        html: None,
        text: None,
        preview: "preview".to_owned(),
        metadata: selector_metadata(1, 1, "article:nth-of-type(1)", "article", &[]),
    };
    assert!(
        render_match_as_text(&json_match)
            .expect("json text render")
            .contains("\"hello\"")
    );
    assert!(
        render_match_as_html(&json_match)
            .expect("json html render")
            .contains("<pre>")
    );

    let text_match = ExtractionMatch {
        index: 1,
        path: None,
        value_type: ValueType::Text,
        value: Value::String("Hello".to_owned()),
        html: Some("<article>Hello</article>".to_owned()),
        text: Some("Hello".to_owned()),
        preview: "Hello".to_owned(),
        metadata: selector_metadata(1, 1, "article:nth-of-type(1)", "article", &[]),
    };
    assert_eq!(
        render_match_as_html(&text_match).expect("text html render"),
        "<article>Hello</article>"
    );

    let wrapped = build_extraction_report(
        "select",
        fixture_result(
            Value::String("<p>Hello</p>".to_owned()),
            ValueType::InnerHtml,
        ),
        None,
    );
    assert!(
        wrap_html_document(&wrapped)
            .expect("wrapped html fragment")
            .contains("<section data-match-index=\"1\">")
    );
    assert!(
        !wrap_html_document(&wrapped)
            .expect("wrapped html fragment")
            .contains("<html lang=")
    );
    let mut language_tagged = build_extraction_report(
        "select",
        fixture_result(
            Value::String("<p>Sveiki</p>".to_owned()),
            ValueType::InnerHtml,
        ),
        None,
    );
    language_tagged.matches[0].html =
        Some("<article lang=\"lv\"><p>Sveiki</p></article>".to_owned());
    assert!(
        wrap_html_document(&language_tagged)
            .expect("language-aware wrapped html fragment")
            .contains("<html lang=\"lv\">")
    );
    let structured_outer_html = build_extraction_report(
        "select",
        fixture_result(serde_json::json!({"kind":"html"}), ValueType::OuterHtml),
        None,
    );
    let structured_outer_html_wrapped =
        wrap_html_document(&structured_outer_html).expect("wrapped html document");
    assert!(!structured_outer_html_wrapped.contains(" lang=\""));
    assert!(!looks_like_document("<section>Hello</section>"));
}

#[test]
fn verbose_and_diagnostic_renderers_cover_branching_paths() {
    let mut result = fixture_result(Value::String("Hello".to_owned()), ValueType::Text);
    result.source.load_steps = vec![
        SourceLoadStep {
            action: SourceLoadAction::HeadPreflight,
            outcome: SourceLoadOutcome::Fallback,
            status: Some(405),
            message: "HEAD returned 405, so HTMLCut fell back to GET.".to_owned(),
        },
        SourceLoadStep {
            action: SourceLoadAction::Get,
            outcome: SourceLoadOutcome::Succeeded,
            status: Some(200),
            message: "Fetched the remote source with GET.".to_owned(),
        },
    ];
    let report = build_extraction_report(
        "select",
        result,
        Some(BundlePaths {
            dir: "/tmp/bundle".to_owned(),
            html: "/tmp/bundle/selection.html".to_owned(),
            text: "/tmp/bundle/selection.txt".to_owned(),
            json: "/tmp/bundle/selection.json".to_owned(),
            report: "/tmp/bundle/report.json".to_owned(),
        }),
    );
    let verbose = build_verbose_lines(&report, 2);
    assert!(verbose[0].contains("2 candidates"));
    assert!(verbose[0].contains("1 selected"));
    assert!(verbose.iter().any(|line| line.contains("match 1 =>")));
    assert!(
        verbose
            .iter()
            .any(|line| { line.contains("source load head preflight fallback (405)") })
    );
    assert!(
        verbose
            .iter()
            .any(|line| line.contains("source load get succeeded (200)"))
    );
    assert!(build_verbose_lines(&report, 0).is_empty());
    let concise_verbose = build_verbose_lines(&report, 1);
    assert!(concise_verbose.len() >= 4);
    assert!(
        concise_verbose
            .iter()
            .any(|line| line.contains("selected text => Hello"))
    );
    assert!(
        concise_verbose
            .iter()
            .any(|line| line.contains("effective base"))
    );
    let mut inspection = fixture_inspection();
    inspection.source.load_steps = report.source.load_steps.clone();
    let inspection_verbose = build_source_inspection_verbose_lines(&inspection, 2);
    assert!(inspection_verbose[0].contains("inspected 123 bytes"));
    assert!(
        inspection_verbose
            .iter()
            .any(|line| line.contains("extraction top"))
    );
    assert!(
        inspection_verbose
            .iter()
            .any(|line| line.contains("head preflight fallback (405)"))
    );
    assert!(
        inspection_verbose
            .iter()
            .any(|line| line.contains("get succeeded (200)"))
    );
    assert!(build_source_inspection_verbose_lines(&inspection, 1).len() >= 4);
    let warning_stderr = build_human_diagnostic_stderr_lines(&[Diagnostic {
        level: DiagnosticLevel::Warning,
        code: DiagnosticCode::EffectiveBaseUrlUnresolved,
        message: "warning".to_owned(),
        details: None,
    }]);
    assert_eq!(warning_stderr.len(), 1);
    assert!(warning_stderr[0].contains("htmlcut: warning EFFECTIVE_BASE_URL_UNRESOLVED"));
    assert_eq!(render_diagnostic_level(DiagnosticLevel::Warning), "warning");
    assert_eq!(render_source_kind(&SourceKind::Url), "url");
}
