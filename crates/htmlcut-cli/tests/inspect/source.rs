use super::*;

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
        .stdout(predicate::str::contains(
            "Suggested selectors for extraction and reading:",
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
fn inspect_source_text_suggests_content_selector_and_skips_placeholder_links() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture(
        tempdir.path(),
        "inspect-scope.html",
        "<html><body><nav><h2>Navigation</h2><a href=\"#\">Comments</a></nav><main id=\"content\"><article><h1>Article Title</h1><p>Alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu.</p><a href=\"/guide\">Guide</a><a href=\"javascript:void(0)\">Share</a></article></main></body></html>",
    );

    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    command
        .args(["inspect", "source"])
        .arg(&input_path)
        .args([
            "--base-url",
            "https://example.com/start",
            "--output",
            "text",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Suggested selectors for extraction:",
        ))
        .stdout(predicate::str::contains(
            "Suggested selectors for rendered text review:",
        ))
        .stdout(predicate::str::contains("- article ["))
        .stdout(predicate::str::contains("#content"))
        .stdout(predicate::str::contains("- h1 Article Title"))
        .stdout(predicate::str::contains(
            "Guide [/guide -> https://example.com/guide]",
        ))
        .stdout(predicate::str::contains("Comments [#]").not())
        .stdout(predicate::str::contains("javascript:void(0)").not());
}

#[test]
fn inspect_source_text_prefers_inner_content_candidate_over_outer_wrapper() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture(
        tempdir.path(),
        "inspect-inner-content.html",
        "<html><body><main id=\"content\"><nav class=\"page-tools\"><a href=\"/edit\">Edit</a></nav><div id=\"mw-content-text\"><article class=\"article-body\"><h1>Article Title</h1><p>Alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu.</p><a href=\"/guide\">Guide</a></article></div><aside class=\"related-topics\"><h2>Related Topics</h2><a href=\"/other\">Other</a></aside></main></body></html>",
    );

    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    command
        .args(["inspect", "source"])
        .arg(&input_path)
        .args([
            "--base-url",
            "https://example.com/start",
            "--output",
            "text",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Suggested selectors for extraction:",
        ))
        .stdout(predicate::str::contains(
            "Suggested selectors for rendered text review:",
        ))
        .stdout(predicate::str::contains("article.article-body"))
        .stdout(predicate::str::contains("- h1 Article Title"))
        .stdout(predicate::str::contains(
            "Guide [/guide -> https://example.com/guide]",
        ))
        .stdout(predicate::str::contains("Edit [/edit").not())
        .stdout(predicate::str::contains("Related Topics").not())
        .stdout(predicate::str::contains("Other [/other").not());
}
