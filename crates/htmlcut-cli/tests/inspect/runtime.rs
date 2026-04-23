use super::*;

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
