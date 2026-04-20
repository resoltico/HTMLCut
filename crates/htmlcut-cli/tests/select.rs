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
