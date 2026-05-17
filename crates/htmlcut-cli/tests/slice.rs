mod support;
use support::*;

#[test]
fn slice_text_output_preserves_selected_fragment_roots_that_only_look_like_utility_chrome() {
    let tempdir = tempdir().expect("tempdir");
    let status_path = write_fixture(
        tempdir.path(),
        "slice-status.html",
        "START::<p class=\"status pricing report\">All Systems Operational</p>::END",
    );
    let nav_path = write_fixture(
        tempdir.path(),
        "slice-nav.html",
        "START::<nav><a href=\"/docs\">Docs</a></nav>::END",
    );

    let mut status = Command::cargo_bin("htmlcut").expect("binary");
    status
        .args(["slice"])
        .arg(&status_path)
        .args(["--from", "START::", "--to", "::END"])
        .assert()
        .success()
        .stdout("All Systems Operational\n");

    let mut nav = Command::cargo_bin("htmlcut").expect("binary");
    nav.args(["slice"])
        .arg(&nav_path)
        .args(["--from", "START::", "--to", "::END"])
        .assert()
        .success()
        .stdout("Docs [/docs]\n");
}

#[test]
fn slice_accepts_inline_html_as_a_first_class_source() {
    let mut command = Command::cargo_bin("htmlcut").expect("binary");
    command
        .args([
            "slice",
            "--input-html",
            "START::<article>Hello</article>::END",
            "--from",
            "START::",
            "--to",
            "::END",
        ])
        .assert()
        .success()
        .stdout("Hello\n");
}
