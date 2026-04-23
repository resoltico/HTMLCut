use super::*;

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
