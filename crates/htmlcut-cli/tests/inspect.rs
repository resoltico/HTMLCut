mod support;
use support::invoke;

#[test]
fn preview_is_explicitly_separate_and_incomplete() {
    let root = htmlcut_tempdir::tempdir().unwrap();
    let source = root.path().join("source.html");
    let plan = root.path().join("plan.json");
    std::fs::write(&source, format!("<p>{}</p>", "value".repeat(500))).unwrap();
    std::fs::write(
        &plan,
        htmlcut_core::canonical_json(&htmlcut_core::ExtractionPlan::css("p").unwrap()).unwrap(),
    )
    .unwrap();
    let output = invoke(
        &[
            "inspect",
            "--file",
            source.to_str().unwrap(),
            "--preview-plan",
            plan.to_str().unwrap(),
        ],
        b"",
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["schema"], "htmlcut.preview");
    assert_eq!(value["complete"], false);
    assert_eq!(value["values"][0].as_str().unwrap().chars().count(), 1024);
}
