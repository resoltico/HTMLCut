pub(super) use super::*;

#[test]
fn rendering_helpers_cover_minimal_verbose_and_inspection_states() {
    let report = build_extraction_report(
        "select",
        fixture_result(Value::String("Hello".to_owned()), ValueType::Text),
        None,
    );
    let verbose = build_verbose_lines(&report, 1);
    assert!(verbose.len() >= 3);
    assert!(
        verbose
            .iter()
            .any(|line| line.contains("selected text => Hello"))
    );
    assert!(verbose.iter().any(|line| line.contains("effective base")));

    let mut minimal_inspection = fixture_inspection();
    let document = minimal_inspection.document.as_mut().expect("document");
    document.top_tags.clear();
    document.top_classes.clear();
    document.extraction_candidates.clear();
    document.reading_candidates.clear();
    document.headings.clear();
    document.links.clear();
    let rendered = render_source_inspection_text(&minimal_inspection, DEFAULT_PREVIEW_CHARS);
    assert!(!rendered.contains("Top tags:"));
    assert!(!rendered.contains("Top classes:"));
    assert!(!rendered.contains("Suggested selectors for extraction:"));
    assert!(!rendered.contains("Suggested selectors for rendered text review:"));
    assert!(!rendered.contains("Headings:"));
    assert!(!rendered.contains("Link previews:"));
}
