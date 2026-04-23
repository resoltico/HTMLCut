mod support;
use support::*;

const README_FIXTURE_HTML: &str = "<html><head><title>HTMLCut README Fixture</title></head><body><main><article><h1>Guide</h1><div class=\"card\">Card alpha</div><div class=\"card\">Card beta</div><p><a class=\"more\" href=\"../guide.html\">Read more</a></p><pre>START::Regex slice payload::END</pre></article></main></body></html>";
const README_TEXT_OUTPUT: &str =
    "Guide\n\nCard alpha\n\nCard beta\n\nRead more\n\nSTART::Regex slice payload::END\n";

#[test]
fn readme_quick_start_commands_run_on_the_demo_page() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture(tempdir.path(), "page.html", README_FIXTURE_HTML);

    let mut select_text = Command::cargo_bin("htmlcut").expect("binary");
    select_text
        .args(["select"])
        .arg(&input_path)
        .args(["--css", "article"])
        .assert()
        .success()
        .stdout(README_TEXT_OUTPUT);

    let mut select_single = Command::cargo_bin("htmlcut").expect("binary");
    select_single
        .args(["select"])
        .arg(&input_path)
        .args(["--css", "article", "--match", "single"])
        .assert()
        .success()
        .stdout(README_TEXT_OUTPUT);

    let mut select_inner_html = Command::cargo_bin("htmlcut").expect("binary");
    select_inner_html
        .args(["select"])
        .arg(&input_path)
        .args(["--css", "article", "--value", "inner-html"])
        .assert()
        .success()
        .stdout("<h1>Guide</h1><div class=\"card\">Card alpha</div><div class=\"card\">Card beta</div><p><a class=\"more\" href=\"../guide.html\">Read more</a></p><pre>START::Regex slice payload::END</pre>\n");

    let mut select_outer_html = Command::cargo_bin("htmlcut").expect("binary");
    select_outer_html
        .args(["select"])
        .arg(&input_path)
        .args(["--css", ".card", "--match", "all", "--value", "outer-html"])
        .assert()
        .success()
        .stdout("<div class=\"card\">Card alpha</div>\n\n<div class=\"card\">Card beta</div>\n");

    let mut select_rewrite = Command::cargo_bin("htmlcut").expect("binary");
    select_rewrite
        .args(["select"])
        .arg(&input_path)
        .args([
            "--css",
            "article a.more",
            "--value",
            "attribute",
            "--attribute",
            "href",
            "--rewrite-urls",
            "--base-url",
            "https://example.com/docs/start.html",
        ])
        .assert()
        .success()
        .stdout("https://example.com/guide.html\n");

    let mut slice_text = Command::cargo_bin("htmlcut").expect("binary");
    slice_text
        .args(["slice"])
        .arg(&input_path)
        .args(["--from", "<article>", "--to", "</article>"])
        .assert()
        .success()
        .stdout(README_TEXT_OUTPUT);

    let mut slice_regex = Command::cargo_bin("htmlcut").expect("binary");
    let report = parse_extraction_report(
        slice_regex
            .args(["slice"])
            .arg(&input_path)
            .args([
                "--from",
                "START::",
                "--to",
                "::END",
                "--pattern",
                "regex",
                "--match",
                "all",
                "--output",
                "json",
            ])
            .assert()
            .success(),
    );
    assert_eq!(report.command, "slice");
    assert_eq!(report.matches.len(), 1);
    assert_eq!(
        report.matches[0].value.as_str(),
        Some("Regex slice payload")
    );

    let mut inspect_source_text = Command::cargo_bin("htmlcut").expect("binary");
    inspect_source_text
        .args(["inspect", "source"])
        .arg(&input_path)
        .args(["--output", "text"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Title: HTMLCut README Fixture"))
        .stdout(predicate::str::contains("Root tag: html"));

    let mut inspect_select = Command::cargo_bin("htmlcut").expect("binary");
    let inspect_select_report = parse_extraction_report(
        inspect_select
            .args(["inspect", "select"])
            .arg(&input_path)
            .args(["--css", ".card", "--match", "all"])
            .assert()
            .success(),
    );
    assert_eq!(inspect_select_report.command, "inspect-select");
    assert_eq!(inspect_select_report.matches.len(), 2);

    let mut inspect_slice = Command::cargo_bin("htmlcut").expect("binary");
    let inspect_slice_report = parse_extraction_report(
        inspect_slice
            .args(["inspect", "slice"])
            .arg(&input_path)
            .args(["--from", "<article>", "--to", "</article>"])
            .assert()
            .success(),
    );
    assert_eq!(inspect_slice_report.command, "inspect-slice");
    assert_eq!(inspect_slice_report.matches.len(), 1);
}

#[test]
fn readme_request_file_and_output_file_flows_run_end_to_end() {
    let tempdir = tempdir().expect("tempdir");
    let input_path = write_fixture(tempdir.path(), "page.html", README_FIXTURE_HTML);
    let request_path = tempdir.path().join("article-links.json");
    let output_path = tempdir.path().join("article.txt");

    let mut emit_request = Command::cargo_bin("htmlcut").expect("binary");
    emit_request
        .args(["select"])
        .arg(&input_path)
        .args([
            "--css",
            "article a.more",
            "--value",
            "attribute",
            "--attribute",
            "href",
            "--emit-request-file",
        ])
        .arg(&request_path)
        .assert()
        .success()
        .stdout("../guide.html\n");
    assert!(request_path.exists());

    let mut request_file = Command::cargo_bin("htmlcut").expect("binary");
    request_file
        .args(["select", "--request-file"])
        .arg(&request_path)
        .assert()
        .success()
        .stdout("../guide.html\n");

    let mut output_file = Command::cargo_bin("htmlcut").expect("binary");
    output_file
        .args(["select"])
        .arg(&input_path)
        .args(["--css", "article", "--output-file"])
        .arg(&output_path)
        .assert()
        .success()
        .stdout("");
    assert_eq!(
        fs::read_to_string(&output_path).expect("output file"),
        README_TEXT_OUTPUT
    );
}
