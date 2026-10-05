// SPDX-License-Identifier: MPL-2.0
mod support;
use support::invoke;

#[test]
fn identifier_inspection_reports_exact_selector_parts_only_when_requested() {
    let source = b"<main id='content' class='docs main docs' data-secret='private'><table><tr><td>China</td><td>17.3%</td></tr></table></main>";
    let ordinary = invoke(&["inspect", "--stdin", "--css", "main"], source);
    assert!(ordinary.status.success());
    let ordinary: serde_json::Value = serde_json::from_slice(&ordinary.stdout).unwrap();
    assert_eq!(
        ordinary["samples"][0]["attributes"],
        serde_json::json!(["class", "data-secret", "id"])
    );
    assert!(ordinary["samples"][0].get("classes").is_none());

    let identified_output = invoke(
        &["inspect", "--stdin", "--css", "main", "--identifiers"],
        source,
    );
    assert!(identified_output.status.success());
    let identified: serde_json::Value = serde_json::from_slice(&identified_output.stdout).unwrap();
    assert_eq!(identified["count"], 1);
    assert_eq!(identified["samples"][0]["id"], "content");
    assert_eq!(
        identified["samples"][0]["classes"],
        serde_json::json!(["docs", "main"])
    );
    assert_eq!(identified["samples"][0]["text"], "China 17.3%");
    assert_eq!(identified["samples"][0]["identifiers_complete"], true);
    assert!(!String::from_utf8_lossy(&identified_output.stdout).contains("private"));
}
#[test]
fn targeted_inspection_counts_completely_and_labels_preview_abbreviation() {
    let source = format!("<p>{}</p><p>second</p>", "é".repeat(200));
    let output = invoke(
        &["inspect", "--stdin", "--css", "p", "--samples", "1"],
        source.as_bytes(),
    );
    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["count"], 2);
    assert_eq!(value["samples_complete"], false);
    assert_eq!(value["samples"][0]["text_complete"], false);
    assert_eq!(
        value["samples"][0]["text"]
            .as_str()
            .unwrap()
            .chars()
            .count(),
        160
    );
    assert!(value.get("next_cursor").is_none());
}

#[test]
fn file_inspection_accepts_explicit_base_metadata_and_returns_complete_samples() {
    let root = htmlcut_tempdir::tempdir().unwrap();
    let path = root.path().join("source.html");
    std::fs::write(&path, "<p id='row'>é</p>").unwrap();
    let output = invoke(
        &[
            "inspect",
            "--file",
            path.to_str().unwrap(),
            "--base-url",
            "https://example.test/root",
            "--css",
            "p",
        ],
        b"",
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let answer: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(answer["count"], 1);
    assert_eq!(answer["samples_complete"], true);
    assert_eq!(answer["samples"][0]["text"], "é");
    assert_eq!(answer["samples"][0]["text_complete"], true);
}

#[test]
fn inspection_acquisition_failure_emits_no_partial_answer() {
    let root = htmlcut_tempdir::tempdir().unwrap();
    let absent = root.path().join("absent.html");
    let result = invoke(
        &["inspect", "--file", absent.to_str().unwrap(), "--css", "p"],
        b"",
    );
    assert_eq!(result.status.code(), Some(5));
    assert!(result.stdout.is_empty());
}
