mod support;
use support::invoke;
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
