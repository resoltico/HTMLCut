mod support;
use support::*;

#[test]
fn select_text_output_extracts_text_for_humans() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture(
        tempdir.path(),
        "input.html",
        "<article><p>Hello <strong>world</strong></p></article>",
    );

    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    command
        .args(["select"])
        .arg(&input_path)
        .args(["--css", "article"])
        .assert()
        .success()
        .stdout("Hello world\n");
}

#[test]
fn select_text_output_preserves_ordered_lists_and_image_alt_text() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture(
        tempdir.path(),
        "semantic-text.html",
        "<article><ol start=\"5\"><li>Five</li><li><img src=\"hero.png\" alt=\"Hero\"></li></ol></article>",
    );

    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    command
        .args(["select"])
        .arg(&input_path)
        .args(["--css", "article"])
        .assert()
        .success()
        .stdout("5. Five\n6. Hero\n");
}

#[test]
fn select_text_output_honors_selected_element_semantics() {
    let tempdir = tempdir().expect("tempdir");
    let image_path = write_fixture(
        tempdir.path(),
        "selected-image.html",
        "<img src=\"hero.png\" alt=\"Hero\">",
    );
    let pre_path = write_fixture(
        tempdir.path(),
        "selected-pre.html",
        "<pre>line 1\n  line 2</pre>",
    );

    let mut image = Command::cargo_bin("htmlcut").expect("binary");
    image
        .args(["select"])
        .arg(&image_path)
        .args(["--css", "img"])
        .assert()
        .success()
        .stdout("Hero\n");

    let mut pre = Command::cargo_bin("htmlcut").expect("binary");
    pre.args(["select"])
        .arg(&pre_path)
        .args(["--css", "pre"])
        .assert()
        .success()
        .stdout("line 1\n  line 2\n");
}

#[test]
fn select_html_value_modes_default_to_html_stdout() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture(
        tempdir.path(),
        "select-html.html",
        "<article><p>Hello <strong>world</strong></p></article>",
    );

    let mut inner = Command::cargo_bin("htmlcut").expect("binary");
    inner
        .args(["select"])
        .arg(&input_path)
        .args(["--css", "article", "--value", "inner-html"])
        .assert()
        .success()
        .stdout("<p>Hello <strong>world</strong></p>\n");

    let mut outer = Command::cargo_bin("htmlcut").expect("binary");
    outer
        .args(["select"])
        .arg(&input_path)
        .args(["--css", "article", "--value", "outer-html"])
        .assert()
        .success()
        .stdout("<article><p>Hello <strong>world</strong></p></article>\n");
}

#[test]
fn select_nth_human_output_does_not_warn_about_multiple_candidates() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture(
        tempdir.path(),
        "select-nth.html",
        "<article class=\"card\">One</article><article class=\"card\">Two</article>",
    );

    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    command
        .args(["select"])
        .arg(&input_path)
        .args(["--css", "article.card", "--match", "nth", "--index", "2"])
        .assert()
        .success()
        .stdout("Two\n")
        .stderr("");
}

#[test]
fn select_single_fails_when_multiple_candidates_exist() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture(
        tempdir.path(),
        "select-single.html",
        "<article class=\"card\">One</article><article class=\"card\">Two</article>",
    );

    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    command
        .args(["select"])
        .arg(&input_path)
        .args(["--css", "article.card", "--match", "single"])
        .assert()
        .failure()
        .code(4)
        .stderr(predicate::str::contains(
            "Exact-one selection requires exactly one candidate",
        ));
}

#[test]
fn select_json_parse_failures_emit_a_versioned_error_report() {
    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    let report = parse_error_report(
        command
            .args(["select", "--output", "json"])
            .assert()
            .failure()
            .code(2),
    );

    assert_eq!(report.tool, "htmlcut");
    assert_eq!(report.engine, "htmlcut-core");
    assert_eq!(report.version, expected_version());
    assert_eq!(report.schema_name, ERROR_COMMAND_REPORT_SCHEMA_NAME);
    assert_eq!(report.schema_version, ERROR_COMMAND_REPORT_SCHEMA_VERSION);
    assert_eq!(report.command, "select");
    assert!(!report.ok);
    assert_eq!(report.exit_code, 2);
    assert_eq!(
        report.error.category,
        htmlcut_cli::ErrorReportCategory::Usage
    );
    assert_eq!(
        report.error.code,
        ErrorReportCode::Cli(CliErrorCode::ParseError)
    );
    assert!(!report.diagnostics.is_empty());
    assert!(report.source_load_steps.is_empty());
}

#[test]
fn select_json_report_has_core_parity_for_structured_extraction() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture(
        tempdir.path(),
        "page.html",
        "<html><head><title>Parity</title></head><body><article class=\"card\"><p>Hello</p></article></body></html>",
    );

    let mut request = ExtractionRequest::new(
        source_request(&input_path, None),
        selector_extraction("article.card")
            .with_selection(SelectionSpec::First)
            .with_value(ValueSpec::Structured),
    );
    request.normalization = NormalizationOptions {
        whitespace: WhitespaceMode::Preserve,
        rewrite_urls: false,
    };
    request.output = extraction_output();
    let expected = extract(&request, &runtime_options());
    assert!(expected.ok);

    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    let report = parse_extraction_report(
        command
            .args(["select"])
            .arg(&input_path)
            .args(["--css", "article.card", "--value", "structured"])
            .assert()
            .success(),
    );

    assert_eq!(report.tool, "htmlcut");
    assert_eq!(report.engine, "htmlcut-core");
    assert_eq!(report.version, expected_version());
    assert_eq!(report.schema_name, EXTRACTION_COMMAND_REPORT_SCHEMA_NAME);
    assert_eq!(report.command, "select");
    assert_eq!(
        report.operation_id,
        htmlcut_core::OperationId::SelectExtract
    );
    assert_eq!(report.ok, expected.ok);
    assert_eq!(
        report.schema_version,
        EXTRACTION_COMMAND_REPORT_SCHEMA_VERSION
    );
    assert_eq!(report.source, expected.source);
    assert_eq!(report.extraction, expected.extraction);
    assert_eq!(report.stats.candidate_count, expected.stats.candidate_count);
    assert_eq!(report.stats.match_count, expected.stats.match_count);
    assert_eq!(report.matches, expected.matches);
    assert_eq!(report.diagnostics, expected.diagnostics);
    assert_eq!(report.document_title.as_deref(), Some("Parity"));
    assert!(report.bundle.is_none());
}

#[test]
fn structured_selector_metadata_only_rewrites_url_bearing_attributes() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture(
        tempdir.path(),
        "input.html",
        "<article><a class=\"card featured\" href=\"guide.html\" data-track=\"hero\">Guide</a></article>",
    );

    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    let report = parse_extraction_report(
        command
            .args(["select"])
            .arg(&input_path)
            .args([
                "--css",
                "a",
                "--value",
                "structured",
                "--rewrite-urls",
                "--base-url",
                "https://example.com/docs/start.html",
                "--output",
                "json",
            ])
            .assert()
            .success(),
    );

    let attributes = report.matches[0].metadata.clone();
    let ExtractionMatchMetadata::Selector(attributes) = attributes else {
        panic!("expected selector metadata");
    };
    assert_eq!(
        attributes.attributes.get("href").map(String::as_str),
        Some("https://example.com/docs/guide.html")
    );
    assert_eq!(
        attributes.attributes.get("class").map(String::as_str),
        Some("card featured")
    );
    assert_eq!(
        attributes.attributes.get("data-track").map(String::as_str),
        Some("hero")
    );
}

#[test]
fn select_attribute_rewrite_has_core_parity() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture(
        tempdir.path(),
        "input.html",
        "<article><a href=\"../guide.html\">Guide</a></article>",
    );
    let base_url = "https://example.com/docs/start.html";

    let mut request = ExtractionRequest::new(
        source_request(&input_path, Some(base_url)),
        selector_extraction("article a")
            .with_selection(SelectionSpec::First)
            .with_value(ValueSpec::Attribute {
                name: AttributeName::new("href").expect("attribute name"),
            }),
    );
    request.normalization = NormalizationOptions {
        whitespace: WhitespaceMode::Preserve,
        rewrite_urls: true,
    };
    request.output = extraction_output();
    let expected = extract(&request, &runtime_options());
    assert!(expected.ok);

    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    let report = parse_extraction_report(
        command
            .args(["select"])
            .arg(&input_path)
            .args([
                "--css",
                "article a",
                "--value",
                "attribute",
                "--attribute",
                "href",
                "--rewrite-urls",
                "--base-url",
                base_url,
                "--output",
                "json",
            ])
            .assert()
            .success(),
    );

    assert_eq!(report.source, expected.source);
    assert_eq!(report.extraction, expected.extraction);
    assert_eq!(report.matches, expected.matches);
    assert_eq!(
        report.matches[0].value.as_str(),
        Some("https://example.com/guide.html")
    );
}

#[test]
fn select_attribute_rewrite_honors_document_base_for_file_inputs() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture(
        tempdir.path(),
        "document-base.html",
        "<html><head><base href=\"https://fixture.example/base/\"></head><body><article><a href=\"guide/start.html\">Guide</a></article></body></html>",
    );

    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    command
        .args(["select"])
        .arg(&input_path)
        .args([
            "--css",
            "article a",
            "--value",
            "attribute",
            "--attribute",
            "href",
            "--rewrite-urls",
        ])
        .assert()
        .success()
        .stdout("https://fixture.example/base/guide/start.html\n");
}
