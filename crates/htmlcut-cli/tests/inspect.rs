mod support;
use support::*;

#[test]
fn slice_json_report_has_core_parity() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture(tempdir.path(), "input.html", "<p>One</p><p>Two</p>");

    let mut request = ExtractionRequest::new(
        source_request(&input_path, None),
        slice_extraction("<p>", "</p>", PatternMode::Literal, false, false)
            .with_selection(SelectionSpec::All)
            .with_value(ValueSpec::Text),
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
            .args(["slice"])
            .arg(&input_path)
            .args([
                "--from", "<p>", "--to", "</p>", "--match", "all", "--output", "json",
            ])
            .assert()
            .success(),
    );

    assert_eq!(report.command, "slice");
    assert_eq!(report.source, expected.source);
    assert_eq!(report.extraction, expected.extraction);
    assert_eq!(report.stats.candidate_count, expected.stats.candidate_count);
    assert_eq!(report.stats.match_count, expected.stats.match_count);
    assert_eq!(report.matches, expected.matches);
    assert_eq!(report.diagnostics, expected.diagnostics);
}

#[test]
fn slice_html_value_modes_default_to_html_stdout() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture(
        tempdir.path(),
        "slice-html.html",
        "<article><p>Hello <strong>world</strong></p></article>",
    );

    let mut inner = Command::cargo_bin("htmlcut").expect("binary");
    inner
        .args(["slice"])
        .arg(&input_path)
        .args([
            "--from",
            "<article>",
            "--to",
            "</article>",
            "--value",
            "inner-html",
        ])
        .assert()
        .success()
        .stdout("<p>Hello <strong>world</strong></p>\n");

    let mut outer = Command::cargo_bin("htmlcut").expect("binary");
    outer
        .args(["slice"])
        .arg(&input_path)
        .args([
            "--from",
            "<article>",
            "--to",
            "</article>",
            "--value",
            "outer-html",
        ])
        .assert()
        .success()
        .stdout("<article><p>Hello <strong>world</strong></p></article>\n");
}

#[test]
fn inspect_source_json_has_core_parity() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture(
        tempdir.path(),
        "inspect.html",
        "<html><head><title>Inspect Me</title></head><body><main><h1>Heading</h1><a href=\"/guide\">Guide</a></main></body></html>",
    );
    let request = source_request(&input_path, Some("https://example.com/start"));
    let expected = inspect_source(
        &request,
        &runtime_options(),
        &htmlcut_core::InspectionOptions {
            include_source_text: false,
            sample_limit: 8,
        },
    );
    assert!(expected.ok);

    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    let report = parse_source_inspection_report(
        command
            .args(["inspect", "source"])
            .arg(&input_path)
            .args(["--base-url", "https://example.com/start"])
            .assert()
            .success(),
    );

    assert_eq!(report.tool, "htmlcut");
    assert_eq!(report.engine, "htmlcut-core");
    assert_eq!(report.version, expected_version());
    assert_eq!(
        report.schema_name,
        SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_NAME
    );
    assert_eq!(report.command, "inspect-source");
    assert_eq!(
        report.operation_id,
        htmlcut_core::OperationId::SourceInspect
    );
    assert_eq!(report.ok, expected.ok);
    assert_eq!(
        report.schema_version,
        SOURCE_INSPECTION_COMMAND_REPORT_SCHEMA_VERSION
    );
    assert_eq!(report.source, expected.source);
    assert_eq!(report.document, expected.document);
    assert_eq!(report.diagnostics, expected.diagnostics);
}

#[test]
fn inspect_source_text_surfaces_base_behavior_and_source_preview() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture(
        tempdir.path(),
        "inspect-text.html",
        "<html><head><base href=\"../content/\"><title>Inspect Me</title></head><body><main><h1>Heading</h1><a href=\"guide.html\">Guide</a></main></body></html>",
    );

    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    command
        .args(["inspect", "source"])
        .arg(&input_path)
        .args([
            "--base-url",
            "https://example.com/docs/start.html",
            "--output",
            "text",
            "--include-source-text",
            "--preview-chars",
            "32",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Input base URL: https://example.com/docs/start.html",
        ))
        .stdout(predicate::str::contains(
            "Effective base URL: https://example.com/content/",
        ))
        .stdout(predicate::str::contains(
            "Document <base href>: ../content/",
        ))
        .stdout(predicate::str::contains("Source text preview:"));
}

#[test]
fn inspect_source_text_reports_unresolved_effective_base() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture(
        tempdir.path(),
        "inspect-unresolved.html",
        "<html><head><base href=\"../content/\"><title>Inspect Me</title></head><body><main><a href=\"guide.html\">Guide</a></main></body></html>",
    );

    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    command
        .args(["inspect", "source"])
        .arg(&input_path)
        .args(["--output", "text"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Document <base href>: ../content/",
        ))
        .stdout(predicate::str::contains("Effective base URL: unresolved"))
        .stdout(predicate::str::contains(
            "warning EFFECTIVE_BASE_URL_UNRESOLVED",
        ));
}

#[test]
fn inspect_select_json_has_core_preview_parity() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture(
        tempdir.path(),
        "preview.html",
        "<section class=\"card\"><h2>One</h2></section><section class=\"card\"><h2>Two</h2></section>",
    );

    let mut request = ExtractionRequest::new(
        source_request(&input_path, None),
        selector_extraction("section.card")
            .with_selection(SelectionSpec::All)
            .with_value(ValueSpec::Structured),
    );
    request.normalization = NormalizationOptions {
        whitespace: WhitespaceMode::Preserve,
        rewrite_urls: false,
    };
    request.output = extraction_output();
    let expected = preview_extraction(&request, &runtime_options());
    assert!(expected.ok);

    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    let report = parse_extraction_report(
        command
            .args(["inspect", "select"])
            .arg(&input_path)
            .args(["--css", "section.card", "--match", "all"])
            .assert()
            .success(),
    );

    assert_eq!(report.command, "inspect-select");
    assert_eq!(
        report.operation_id,
        htmlcut_core::OperationId::SelectPreview
    );
    assert_eq!(report.source, expected.source);
    assert_eq!(report.extraction, expected.extraction);
    assert_eq!(report.stats.candidate_count, expected.stats.candidate_count);
    assert_eq!(report.stats.match_count, expected.stats.match_count);
    assert_eq!(report.matches, expected.matches);
    assert_eq!(report.diagnostics, expected.diagnostics);
}

#[test]
fn inspect_select_nth_does_not_warn_about_multiple_candidates() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture(
        tempdir.path(),
        "inspect-select-nth.html",
        "<section class=\"card\">One</section><section class=\"card\">Two</section>",
    );

    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    let report = parse_extraction_report(
        command
            .args(["inspect", "select"])
            .arg(&input_path)
            .args([
                "--css",
                "section.card",
                "--match",
                "nth",
                "--index",
                "2",
                "--output",
                "json",
            ])
            .assert()
            .success(),
    );

    assert!(report.ok);
    assert_eq!(report.stats.candidate_count, 2);
    assert_eq!(report.stats.match_count, 1);
    assert!(report.diagnostics.is_empty());
}

#[test]
fn inspect_slice_json_has_core_preview_parity() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture(
        tempdir.path(),
        "slice-preview.html",
        "<div>START::Alpha::END</div><div>START::Beta::END</div>",
    );

    let mut request = ExtractionRequest::new(
        source_request(&input_path, None),
        slice_extraction("START::[A-Za-z]+", "::END", PatternMode::Regex, true, true)
            .with_selection(SelectionSpec::All)
            .with_value(ValueSpec::Structured),
    );
    request.normalization = NormalizationOptions {
        whitespace: WhitespaceMode::Preserve,
        rewrite_urls: false,
    };
    request.output = extraction_output();
    let expected = preview_extraction(&request, &runtime_options());
    assert!(expected.ok);

    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    let report = parse_extraction_report(
        command
            .args(["inspect", "slice"])
            .arg(&input_path)
            .args([
                "--from",
                "START::[A-Za-z]+",
                "--to",
                "::END",
                "--pattern",
                "regex",
                "--include-start",
                "--include-end",
                "--match",
                "all",
            ])
            .assert()
            .success(),
    );

    assert_eq!(report.command, "inspect-slice");
    assert_eq!(report.operation_id, htmlcut_core::OperationId::SlicePreview);
    assert_eq!(report.source, expected.source);
    assert_eq!(report.extraction, expected.extraction);
    assert_eq!(report.stats.candidate_count, expected.stats.candidate_count);
    assert_eq!(report.stats.match_count, expected.stats.match_count);
    assert_eq!(report.matches, expected.matches);
    assert_eq!(report.diagnostics, expected.diagnostics);
}

#[test]
fn inspect_slice_text_surfaces_ranges_and_boundary_context() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture(
        tempdir.path(),
        "slice-preview-text.html",
        "<div>START::Alpha::END</div><div>START::Beta::END</div>",
    );

    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    command
        .args(["inspect", "slice"])
        .arg(&input_path)
        .args([
            "--from",
            "START::[A-Za-z]+",
            "--to",
            "::END",
            "--pattern",
            "regex",
            "--include-start",
            "--include-end",
            "--match",
            "all",
            "--output",
            "text",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Selected: 2"))
        .stdout(predicate::str::contains("candidate index:"))
        .stdout(predicate::str::contains("include start: true"))
        .stdout(predicate::str::contains("include end: true"))
        .stdout(predicate::str::contains("selected range:"))
        .stdout(predicate::str::contains("inner range:"))
        .stdout(predicate::str::contains("outer range:"))
        .stdout(predicate::str::contains("text: START::Alpha::END"));
}

#[test]
fn inspect_slice_text_shows_fragment_preview_when_boundary_consumption_hides_text() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture(
        tempdir.path(),
        "slice-preview-empty-text.html",
        "<div>START::Alpha::END</div><div>START::Beta::END</div>",
    );

    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    command
        .args(["inspect", "slice"])
        .arg(&input_path)
        .args([
            "--from",
            "START::[A-Za-z]+",
            "--to",
            "::END",
            "--pattern",
            "regex",
            "--match",
            "all",
            "--output",
            "text",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("fragment: START::Alpha::END"))
        .stdout(predicate::str::contains("fragment: START::Beta::END"));
}

#[test]
fn inspect_slice_text_shows_fragment_preview_for_html_like_matches() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture(
        tempdir.path(),
        "inspect-slice-html.html",
        "<article><a href=\"guide.html\">Guide</a></article><section><a href=\"more.html\">More</a></section>",
    );

    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    command
        .args(["inspect", "slice"])
        .arg(&input_path)
        .args([
            "--from",
            "<a",
            "--to",
            "</a>",
            "--include-start",
            "--include-end",
            "--match",
            "all",
            "--output",
            "text",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "fragment: <article><a href=\"guide.html\">Guide</a>",
        ))
        .stdout(predicate::str::contains(
            "fragment: <a href=\"more.html\">More</a>",
        ));
}

#[test]
fn stdin_bundle_flow_and_verbose_levels_work() {
    let tempdir = tempdir().expect("tempdir");
    let bundle_dir = tempdir.path().join("bundle space");

    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    command
        .args(["select", "-"])
        .args([
            "--css",
            "article",
            "--output",
            "json",
            "--bundle",
            bundle_dir.to_str().expect("bundle dir"),
            "-vv",
        ])
        .write_stdin("<article><p>Hello</p></article>")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"command\": \"select\""))
        .stderr(predicate::str::contains("selected 1 match"))
        .stderr(predicate::str::contains("scanned 1 candidates"))
        .stderr(predicate::str::contains("wrote bundle"));

    assert!(bundle_dir.join("selection.html").exists());
    assert!(bundle_dir.join("selection.txt").exists());
    assert!(bundle_dir.join("report.json").exists());
}

#[test]
fn global_verbose_before_subcommand_also_works() {
    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    command
        .args(["-vv", "select", "-"])
        .args(["--css", "article"])
        .write_stdin("<article><p>Hello</p></article>")
        .assert()
        .success()
        .stdout("Hello\n")
        .stderr(predicate::str::contains("selected 1 match"))
        .stderr(predicate::str::contains("scanned 1 candidates"));
}

#[test]
fn human_select_warns_when_rewrite_is_requested_without_an_effective_base() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture(
        tempdir.path(),
        "unresolved-base.html",
        "<html><head><base href=\"../content/\"></head><body><a href=\"guide.html\">Guide</a></body></html>",
    );

    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    command
        .args(["select"])
        .arg(&input_path)
        .args([
            "--css",
            "a",
            "--value",
            "attribute",
            "--attribute",
            "href",
            "--rewrite-urls",
        ])
        .assert()
        .success()
        .stdout("guide.html\n")
        .stderr(predicate::str::contains(
            "warning EFFECTIVE_BASE_URL_UNRESOLVED",
        ));
}

#[test]
fn invalid_selector_exits_with_usage_code() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture(tempdir.path(), "input.html", "<div>Hello</div>");

    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    command
        .args(["select"])
        .arg(input_path)
        .args(["--css", "["])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("Invalid selector"));
}

#[test]
fn slice_attribute_error_hints_when_excluded_start_boundary_drops_the_opening_tag() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture(
        tempdir.path(),
        "slice-attribute.html",
        "<article><a href=\"guide.html\">Guide</a></article>",
    );

    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    command
        .args(["slice"])
        .arg(&input_path)
        .args([
            "--from",
            "<a ",
            "--to",
            "</a>",
            "--value",
            "attribute",
            "--attribute",
            "href",
        ])
        .assert()
        .failure()
        .code(4)
        .stderr(predicate::str::contains("use --include-start"));
}

#[test]
fn output_none_requires_bundle() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture(tempdir.path(), "input.html", "<div>Hello</div>");

    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    command
        .args(["select"])
        .arg(input_path)
        .args(["--css", "div", "--output", "none"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("--output none requires --bundle"));
}
