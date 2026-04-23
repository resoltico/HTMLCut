use super::*;

#[test]
fn preview_and_manifest_helpers_cover_remaining_branches() {
    assert_eq!(
        validate_preview_chars(32).expect("preview chars"),
        NonZeroUsize::new(32).expect("preview chars")
    );
    assert!(validate_preview_chars(0).is_err());
    assert_eq!(render_text_preview("short", 32), "short");
    assert_eq!(render_text_preview("preview", 3), "pre...");
    assert_eq!(
        workspace_package_field("[workspace.package]\nversion = \"3.0.0\"\n", "description"),
        None
    );
    assert_eq!(
        workspace_package_field(
            "[package]\ndescription = \"wrong\"\n[workspace.package]\ndescription = \"right\"\n",
            "description"
        ),
        Some("right".to_owned())
    );
    assert_eq!(
        workspace_package_field(
            "[workspace.package]\ndescription = \"broken\n",
            "description"
        ),
        None
    );

    let mut input_only = fixture_inspection();
    input_only.source.effective_base_url = None;
    let rendered = render_source_inspection_text(&input_only, DEFAULT_PREVIEW_CHARS);
    assert!(rendered.contains("Input base URL: https://example.com/docs/start.html"));
    assert!(!rendered.contains("Effective base URL: https://example.com/docs/start.html"));
}
#[test]
fn preview_helpers_cover_metadata_mismatches_and_empty_reports() {
    let empty_preview = build_extraction_report(
        "inspect-select",
        fixture_result(
            serde_json::json!({"tagName":"article"}),
            ValueType::Structured,
        ),
        None,
    );
    let mut empty_preview = empty_preview;
    empty_preview.matches.clear();
    empty_preview.diagnostics.clear();
    let empty_preview_text = render_preview_text(&empty_preview);
    assert!(!empty_preview_text.contains("Diagnostics:"));
    assert!(!empty_preview_text.contains("Matches:"));

    let select_preview_with_slice_metadata = render_preview_match_lines(
        htmlcut_core::OperationId::SelectPreview,
        &ExtractionMatch {
            index: 1,
            path: Some("explicit-path".to_owned()),
            value_type: ValueType::Structured,
            value: serde_json::json!({}),
            html: None,
            text: None,
            preview: "fallback select preview".to_owned(),
            metadata: delimiter_metadata(1, 1, (1, 3), (1, 3), (1, 3), false, false),
        },
    );
    assert_eq!(select_preview_with_slice_metadata[0], "1. explicit-path");
    assert!(
        select_preview_with_slice_metadata
            .iter()
            .any(|line| line == "   preview: fallback select preview")
    );
    assert!(
        select_preview_with_slice_metadata
            .iter()
            .all(|line| !line.contains("tag:"))
    );

    let slice_preview_with_selector_metadata = render_preview_match_lines(
        htmlcut_core::OperationId::SlicePreview,
        &ExtractionMatch {
            index: 2,
            path: Some("slice-path".to_owned()),
            value_type: ValueType::Structured,
            value: serde_json::json!({}),
            html: Some("same".to_owned()),
            text: Some("same".to_owned()),
            preview: "unused".to_owned(),
            metadata: selector_metadata(1, 1, "article:nth-of-type(1)", "article", &[]),
        },
    );
    assert_eq!(slice_preview_with_selector_metadata[0], "2. slice-path");
    assert!(
        slice_preview_with_selector_metadata
            .iter()
            .any(|line| line == "   text: same")
    );
    assert!(
        slice_preview_with_selector_metadata
            .iter()
            .all(|line| !line.contains("candidate index:"))
    );
    assert!(
        slice_preview_with_selector_metadata
            .iter()
            .all(|line| !line.contains("fragment:"))
    );
}
