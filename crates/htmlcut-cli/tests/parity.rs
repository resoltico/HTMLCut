// SPDX-License-Identifier: MPL-2.0
mod support;
use support::invoke;

#[test]
fn file_stdin_and_inline_library_have_equal_deterministic_results() {
    let root = htmlcut_tempdir::tempdir().unwrap();
    let html = "<p id='amount'>EUR 180</p>";
    let source = {
        let path = root.path().join("page.html");
        std::fs::write(&path, html).unwrap();
        path
    };
    let file = invoke(
        &[
            "extract",
            "--file",
            source.to_str().unwrap(),
            "--css",
            "#amount",
        ],
        b"",
    );
    let stdin = invoke(&["extract", "--stdin", "--css", "#amount"], html.as_bytes());
    assert!(file.status.success());
    assert!(stdin.status.success());
    assert_eq!(file.stdout, stdin.stdout);
    let prepared = htmlcut_core::PreparedDocument::new(
        htmlcut_core::SourceSnapshot::new(html, Default::default()).unwrap(),
        Default::default(),
    )
    .unwrap();
    let compiled =
        htmlcut_core::CompiledPlan::compile(&htmlcut_core::ExtractionPlan::css("#amount").unwrap())
            .unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&file.stdout).unwrap(),
        serde_json::to_value(prepared.execute(&compiled).unwrap().data).unwrap()
    );
    let missing =
        htmlcut_core::CompiledPlan::compile(&htmlcut_core::ExtractionPlan::css("aside").unwrap())
            .unwrap();
    let expected = prepared.execute(&missing).unwrap_err();
    let actual = invoke(
        &[
            "extract",
            "--file",
            source.to_str().unwrap(),
            "--css",
            "aside",
        ],
        b"",
    );
    assert_eq!(actual.status.code(), Some(3));
    assert!(actual.stdout.is_empty());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&actual.stderr).unwrap(),
        serde_json::to_value(expected).unwrap()
    );
}
