use super::*;

#[test]
fn catalog_and_preview_renderers_cover_remaining_branches() {
    let empty_catalog = CatalogCommandReport {
        tool: TOOL_NAME.to_owned(),
        version: HTMLCUT_VERSION.to_owned(),
        schema_name: CATALOG_REPORT_SCHEMA_NAME.to_owned(),
        schema_version: crate::model::CATALOG_SCHEMA_VERSION,
        schema_profile: htmlcut_core::HTMLCUT_JSON_SCHEMA_PROFILE.to_owned(),
        description: HTMLCUT_DESCRIPTION.to_owned(),
        command: "catalog".to_owned(),
        operations: Vec::new(),
    };
    assert_eq!(
        render_catalog_text(&empty_catalog),
        format!(
            "{DISPLAY_NAME} {HTMLCUT_VERSION}\n{HTMLCUT_DESCRIPTION}\nCatalog: 0 operations.\nUse `htmlcut catalog --operation <OPERATION_ID>` for one compact contract or `--output json` for the full machine-readable surface."
        )
    );
    assert_eq!(
        render_catalog_surface(None, &CatalogAvailability::Cli),
        "cli".to_owned()
    );

    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "catalog".to_owned(),
        "--output".to_owned(),
        "text".to_owned(),
    ]);
    assert_eq!(exit_code, 0);
    assert!(stdout.contains("Operations:"));
    assert!(stderr.is_empty());

    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "catalog".to_owned(),
        "--operation".to_owned(),
        "slice.extract".to_owned(),
    ]);
    assert_eq!(exit_code, 0);
    assert!(stdout.contains("Operation:"));
    assert!(stdout.contains("engine capability: extract slice values"));
    assert!(stdout.contains("request: extraction request + runtime options"));
    assert!(
        stdout.contains("request schemas: htmlcut.extraction_request@7, htmlcut.runtime_options@7")
    );
    assert!(stdout.contains("result: extraction result"));
    assert!(stdout.contains("result schemas: htmlcut.extraction_result@6"));
    assert!(
        stdout.contains("Use `--output json` for parameters, defaults, constraints, and examples.")
    );
    assert!(stderr.is_empty());

    let (exit_code, stdout, stderr) = run_vec(vec![
        "htmlcut".to_owned(),
        "catalog".to_owned(),
        "--output".to_owned(),
        "json".to_owned(),
        "--operation".to_owned(),
        "unknown.operation".to_owned(),
    ]);
    assert_eq!(exit_code, EXIT_CODE_USAGE);
    assert!(stdout.contains("\"code\": \"CLI_OPERATION_ID_UNKNOWN\""));
    assert!(stderr.is_empty());

    let select_preview_lines = render_preview_match_lines(
        htmlcut_core::OperationId::SelectPreview,
        &ExtractionMatch {
            index: 2,
            path: None,
            value_type: ValueType::Structured,
            value: serde_json::json!({}),
            html: None,
            text: None,
            preview: "structured preview".to_owned(),
            metadata: selector_metadata(1, 1, "article:nth-of-type(1)", "article", &[]),
        },
    );
    assert!(
        select_preview_lines
            .iter()
            .any(|line| line == "2. (no path)")
    );
    assert!(
        select_preview_lines
            .iter()
            .any(|line| line == "   preview: structured preview")
    );

    let rich_select_preview_lines = render_preview_match_lines(
        htmlcut_core::OperationId::SelectPreview,
        &ExtractionMatch {
            index: 4,
            path: Some("article:nth-of-type(1)".to_owned()),
            value_type: ValueType::Structured,
            value: serde_json::json!({}),
            html: None,
            text: Some("Alpha beta".to_owned()),
            preview: "unused".to_owned(),
            metadata: selector_metadata(
                3,
                2,
                "article:nth-of-type(1)",
                "article",
                &[("class", "card featured")],
            ),
        },
    );
    assert!(
        rich_select_preview_lines
            .iter()
            .any(|line| line == "   attributes: class=\"card featured\"")
    );
    assert!(
        rich_select_preview_lines
            .iter()
            .any(|line| line == "   text: Alpha beta")
    );

    let slice_preview_lines = render_preview_match_lines(
        htmlcut_core::OperationId::SlicePreview,
        &ExtractionMatch {
            index: 3,
            path: None,
            value_type: ValueType::Structured,
            value: serde_json::json!({}),
            html: None,
            text: None,
            preview: "slice preview".to_owned(),
            metadata: delimiter_metadata(9, 7, (1, 12), (4, 9), (1, 12), true, true),
        },
    );
    assert!(
        slice_preview_lines
            .iter()
            .any(|line| line == "3. range 1..12")
    );
    assert!(
        slice_preview_lines
            .iter()
            .any(|line| {
                line
                    == "   candidate 7 | selected 1..12 | inner 4..9 | outer 1..12 | retention include-both"
            })
    );
    assert!(
        slice_preview_lines
            .iter()
            .any(|line| line == "   boundaries: <article> … </article>")
    );
    assert!(
        slice_preview_lines
            .iter()
            .any(|line| line == "   preview: slice preview")
    );

    let rich_slice_preview_lines = render_preview_match_lines(
        htmlcut_core::OperationId::SlicePreview,
        &ExtractionMatch {
            index: 5,
            path: None,
            value_type: ValueType::Structured,
            value: serde_json::json!({}),
            html: None,
            text: Some("alpha beta".to_owned()),
            preview: "unused".to_owned(),
            metadata: delimiter_metadata(10, 8, (2, 7), (2, 7), (1, 8), false, false),
        },
    );
    assert!(rich_slice_preview_lines.iter().any(|line| {
        line == "   candidate 8 | selected 2..7 | inner 2..7 | outer 1..8 | retention exclude-both"
    }));
    assert!(
        rich_slice_preview_lines
            .iter()
            .any(|line| line == "   text: alpha beta")
    );

    let fragment_signal_slice_preview_lines = render_preview_match_lines(
        htmlcut_core::OperationId::SlicePreview,
        &ExtractionMatch {
            index: 8,
            path: None,
            value_type: ValueType::Structured,
            value: serde_json::json!({}),
            html: Some("START::Alpha::END".to_owned()),
            text: Some(String::new()),
            preview: "unused".to_owned(),
            metadata: delimiter_metadata(2, 1, (12, 12), (12, 12), (5, 17), false, false),
        },
    );
    assert!(
        fragment_signal_slice_preview_lines
            .iter()
            .any(|line| line == "   fragment: START::Alpha::END")
    );
    assert!(
        fragment_signal_slice_preview_lines
            .iter()
            .all(|line| !line.starts_with("   text:"))
    );

    let sparse_slice_preview_lines = render_preview_match_lines(
        htmlcut_core::OperationId::SlicePreview,
        &ExtractionMatch {
            index: 6,
            path: None,
            value_type: ValueType::Structured,
            value: serde_json::json!({}),
            html: None,
            text: Some("fallback branch coverage".to_owned()),
            preview: "unused".to_owned(),
            metadata: delimiter_metadata(1, 1, (10, 20), (10, 20), (9, 21), false, false),
        },
    );
    assert!(
        sparse_slice_preview_lines
            .iter()
            .any(|line| line == "6. range 10..20")
    );
    assert!(
        sparse_slice_preview_lines
            .iter()
            .all(|line| !line.contains("source index:"))
    );
    assert!(
        sparse_slice_preview_lines
            .iter()
            .any(|line| {
                line
                    == "   candidate 1 | selected 10..20 | inner 10..20 | outer 9..21 | retention exclude-both"
            })
    );
    assert!(
        sparse_slice_preview_lines
            .iter()
            .any(|line| line == "   text: fallback branch coverage")
    );
    assert_eq!(
        render_preview_location(
            htmlcut_core::OperationId::SlicePreview,
            &ExtractionMatch {
                index: 7,
                path: None,
                value_type: ValueType::Structured,
                value: serde_json::json!({}),
                html: None,
                text: None,
                preview: "unused".to_owned(),
                metadata: selector_metadata(1, 1, "article:nth-of-type(1)", "article", &[]),
            }
        ),
        "(no path)".to_owned()
    );

    let fallback_preview_lines = render_preview_match_lines(
        htmlcut_core::OperationId::SelectExtract,
        &ExtractionMatch {
            index: 1,
            path: None,
            value_type: ValueType::Structured,
            value: serde_json::json!({}),
            html: None,
            text: None,
            preview: "fallback".to_owned(),
            metadata: selector_metadata(1, 1, "article:nth-of-type(1)", "article", &[]),
        },
    );
    assert!(
        fallback_preview_lines
            .iter()
            .any(|line| line == "   preview: fallback")
    );

    assert_eq!(render_attribute_summary(&attribute_map(&[])), None);
    assert_eq!(
        render_attribute_summary(&attribute_map(&[("count", "1")])),
        Some("count=\"1\"".to_owned())
    );
    assert_eq!(
        render_attribute_summary(&attribute_map(&[("class", "card")])),
        Some("class=\"card\"".to_owned())
    );
    assert_eq!(
        render_range_summary(Some(&Range { start: 9, end: 12 })),
        Some("9..12".to_owned())
    );
    assert_eq!(render_range_summary(None), None);
    assert_eq!(
        compact_inline_preview("alpha beta gamma", 5),
        "alpha...".to_owned()
    );
}
