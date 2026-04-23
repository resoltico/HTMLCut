use super::*;

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
