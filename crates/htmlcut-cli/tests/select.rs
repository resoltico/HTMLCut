// SPDX-License-Identifier: MPL-2.0
mod support;
use support::invoke;

#[test]
fn literal_default_preserves_hidden_links_and_empty_values() {
    for (html, expected) in [
        (
            "<p hidden>Before <a class='reference internal'>IMPORTANT</a> after.</p>",
            "Before IMPORTANT after.",
        ),
        ("<p></p>", ""),
    ] {
        let output = invoke(&["extract", "--stdin", "--css", "p"], html.as_bytes());
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value, serde_json::json!([expected]));
    }
}

#[test]
fn markdown_and_html_are_explicit_and_raw_has_no_framing() {
    let html = b"<p>Damage: <img alt='Broken mirror'> end.</p>";
    let output = invoke(
        &[
            "extract", "--stdin", "--css", "p", "--read", "markdown", "--raw",
        ],
        html,
    );
    assert!(output.status.success());
    assert_eq!(output.stdout, b"Damage: Broken mirror end.");
    let output = invoke(
        &[
            "extract",
            "--stdin",
            "--css",
            "p",
            "--read",
            "outer_html",
            "--raw",
        ],
        b"<P a='x'>V</P>",
    );
    assert!(output.status.success());
    assert_eq!(output.stdout, br#"<p a="x">V</p>"#);
}

#[test]
fn strict_selection_and_missing_attributes_fail_without_values() {
    for (html, args, code) in [
        (
            &b"<p>A</p><p>B</p>"[..],
            vec!["extract", "--stdin", "--css", "p"],
            "ambiguous_selection",
        ),
        (
            &b"<p>A</p>"[..],
            vec!["extract", "--stdin", "--css", "aside"],
            "no_match",
        ),
        (
            &b"<p>A</p>"[..],
            vec![
                "extract",
                "--stdin",
                "--css",
                "p",
                "--read",
                "attribute:absent",
            ],
            "missing_attribute",
        ),
    ] {
        let output = invoke(&args, html);
        assert_eq!(output.status.code(), Some(3));
        assert!(output.stdout.is_empty());
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&output.stderr).unwrap()["code"],
            code
        );
    }
    let output = invoke(
        &[
            "extract", "--stdin", "--css", "p", "--match", "nth", "--index", "2", "--raw",
        ],
        b"<p>A</p><p>B</p>",
    );
    assert!(output.status.success());
    assert_eq!(output.stdout, b"B");
}
