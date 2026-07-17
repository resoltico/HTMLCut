use super::*;

#[test]
fn bundle_document_title_prefers_core_and_then_falls_back() {
    let titled_report = build_extraction_report(
        "select",
        fixture_result(Value::String("Hello".to_owned()), ValueType::Text),
        None,
    );
    assert_eq!(bundle_document_title(&titled_report), "Fixture");

    let mut fallback_host = build_extraction_report(
        "select",
        fixture_result(Value::String("Hello".to_owned()), ValueType::Text),
        None,
    );
    fallback_host.document_title = None;
    fallback_host.source.effective_base_url =
        Some("https://example.net/docs/start.html".to_owned());
    assert_eq!(bundle_document_title(&fallback_host), "example.net");

    let mut fallback_path = build_extraction_report(
        "select",
        fixture_result(Value::String("Hello".to_owned()), ValueType::Text),
        None,
    );
    fallback_path.document_title = None;
    fallback_path.source.input_base_url = None;
    fallback_path.source.effective_base_url = None;
    fallback_path.source.value = "/tmp/sample name.html".to_owned();
    assert_eq!(bundle_document_title(&fallback_path), "sample name");

    let mut invalid_url = build_extraction_report(
        "select",
        fixture_result(Value::String("Hello".to_owned()), ValueType::Text),
        None,
    );
    invalid_url.document_title = None;
    invalid_url.source.effective_base_url = Some("not a url".to_owned());
    invalid_url.source.value = "/tmp/sample name.html".to_owned();
    assert_eq!(bundle_document_title(&invalid_url), "sample name");
}

#[test]
fn render_output_helpers_cover_text_html_json_and_none() {
    let text_report = build_extraction_report(
        "select",
        fixture_result(Value::String("Hello".to_owned()), ValueType::Text),
        None,
    );
    assert_eq!(
        render_extraction_output(&text_report, CliOutputMode::Text)
            .expect("text output")
            .expect("stdout payload"),
        "Hello"
    );

    let html_report = build_extraction_report(
        "select",
        fixture_result(
            Value::String("<p>Hello</p>".to_owned()),
            ValueType::InnerHtml,
        ),
        None,
    );
    assert!(
        render_extraction_output(&html_report, CliOutputMode::Html)
            .expect("html output")
            .expect("stdout payload")
            .contains("<p>Hello</p>")
    );
    assert_eq!(
        render_extraction_output(&html_report, CliOutputMode::Text)
            .expect("html rendered as text")
            .expect("stdout payload"),
        "Hello"
    );
    let mut broken_html_report = build_extraction_report(
        "select",
        fixture_result(
            Value::String("<p>Hello</p>".to_owned()),
            ValueType::OuterHtml,
        ),
        None,
    );
    broken_html_report.matches[0].text = None;
    assert_eq!(
        render_extraction_output(&broken_html_report, CliOutputMode::Text)
            .expect_err("missing text projection should fail")
            .code,
        "CLI_TEXT_PROJECTION_MISSING"
    );
    assert!(
        render_extraction_output(&text_report, CliOutputMode::Json)
            .expect("json output")
            .expect("stdout payload")
            .contains("\"command\": \"select\"")
    );
    assert!(
        render_extraction_output(&text_report, CliOutputMode::None)
            .expect("none output")
            .is_none()
    );

    let tempdir = tempdir().expect("tempdir");
    let existing_parent = tempdir.path().join("existing-parent");
    fs::create_dir(&existing_parent).expect("existing parent");
    let bundle = get_bundle_paths(&existing_parent.join("..").join("fresh bundle"));
    let expected_bundle_dir = tempdir
        .path()
        .canonicalize()
        .expect("canonical tempdir")
        .join("fresh bundle");
    assert_eq!(bundle.dir, expected_bundle_dir.display().to_string());
    assert_eq!(
        bundle.html,
        expected_bundle_dir
            .join("selection.html")
            .display()
            .to_string()
    );
    assert_eq!(
        bundle.json,
        expected_bundle_dir
            .join("selection.json")
            .display()
            .to_string()
    );
}

#[test]
fn bundle_report_omits_sidecar_payload_duplicates() {
    let report = build_extraction_report(
        "select",
        fixture_result(
            Value::String("<article><a href=\"guide.html\">Guide</a></article>".to_owned()),
            ValueType::OuterHtml,
        ),
        None,
    );
    let tempdir = tempdir().expect("tempdir");
    let bundle = get_bundle_paths(&tempdir.path().join("bundle"));

    write_bundle(&report, &bundle).expect("bundle write");

    let bundled_report: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&bundle.report).expect("bundle report"))
            .expect("json");
    let bundled_selection: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&bundle.json).expect("bundle selection"))
            .expect("json");

    assert_eq!(bundled_report["schema_name"], "htmlcut.bundle_report");
    assert!(bundled_report["matches"][0].get("value").is_none());
    assert!(bundled_report["matches"][0].get("html").is_none());
    assert!(bundled_report["matches"][0].get("text").is_none());
    assert_eq!(bundled_selection["schema_name"], "htmlcut.bundle_selection");
    assert_eq!(
        bundled_selection["matches"][0]["value"],
        Value::String("<article><a href=\"guide.html\">Guide</a></article>".to_owned())
    );
}

#[test]
fn canonical_bundle_dir_covers_fallback_edges() {
    let tempdir = tempdir().expect("tempdir");
    let _guard = CurrentDirGuard::enter(tempdir.path());
    let entered_dir = std::env::current_dir().expect("entered current dir");
    assert_same_path_identity(
        &canonical_bundle_dir_for_tests(Path::new("missing/child")),
        &entered_dir.join("missing/child"),
    );
    assert_same_path_identity(
        &canonical_bundle_dir_for_tests(Path::new("missing/..")),
        &entered_dir,
    );

    #[cfg(unix)]
    {
        std::env::set_current_dir(Path::new("/")).expect("enter root");
        assert_eq!(
            canonical_bundle_dir_for_tests(Path::new("")),
            std::path::PathBuf::from("/")
        );
    }
}

#[test]
fn lexical_path_normalization_covers_relative_and_empty_edges() {
    assert_eq!(
        lexical_normalize_path_for_tests(Path::new("./nested/../report")),
        Path::new("report")
    );
    assert_eq!(
        lexical_normalize_path_for_tests(Path::new("../nested/../../report")),
        Path::new("../../report")
    );
    assert_eq!(
        lexical_normalize_path_for_tests(Path::new(".")),
        Path::new(".")
    );

    #[cfg(unix)]
    {
        assert_eq!(
            lexical_normalize_path_for_tests(Path::new("/")),
            Path::new("/")
        );
        assert_eq!(
            lexical_normalize_path_for_tests(Path::new("/../")),
            Path::new("/")
        );
    }
}
