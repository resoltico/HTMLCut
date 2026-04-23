use super::*;

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
