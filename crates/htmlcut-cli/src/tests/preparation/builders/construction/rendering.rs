pub(super) use super::*;

#[test]
fn rendering_helpers_cover_minimal_verbose_and_inspection_states() {
    let report = build_extraction_report(
        "select",
        fixture_result(Value::String("Hello".to_owned()), ValueType::Text),
        None,
    );
    assert_eq!(build_verbose_lines(&report, 1).len(), 1);

    let mut minimal_inspection = fixture_inspection();
    let document = minimal_inspection.document.as_mut().expect("document");
    document.top_tags.clear();
    document.top_classes.clear();
    document.headings.clear();
    document.links.clear();
    let rendered = render_source_inspection_text(&minimal_inspection, DEFAULT_PREVIEW_CHARS);
    assert!(!rendered.contains("Top tags:"));
    assert!(!rendered.contains("Top classes:"));
    assert!(!rendered.contains("Headings:"));
    assert!(!rendered.contains("Link previews:"));
}
