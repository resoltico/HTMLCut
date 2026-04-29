use serde_json::Value;

#[path = "../examples/support/request_and_result_namespaces.rs"]
mod request_and_result_namespaces;
#[path = "../examples/support/reusable_extraction_definition.rs"]
mod reusable_extraction_definition;

#[test]
fn request_and_result_namespaces_example_emits_a_namespace_summary() {
    let mut output = Vec::new();
    request_and_result_namespaces::write_request_and_result_namespace_summary(&mut output)
        .expect("write summary");
    let value: Value = serde_json::from_slice(&output).expect("parse example json");

    assert_eq!(value["request_namespace"], "htmlcut_core::request");
    assert_eq!(value["result_namespace"], "htmlcut_core::result");
    assert_eq!(value["value"], "https://example.com/guide.html");
    assert_eq!(value["tag_name"], "a");
    assert!(
        value["path"]
            .as_str()
            .is_some_and(|path| path.contains("article:nth-of-type(1) > a:nth-of-type(1)"))
    );
}

#[test]
fn reusable_extraction_definition_example_emits_a_reusable_definition() {
    let mut output = Vec::new();
    reusable_extraction_definition::write_reusable_extraction_definition(&mut output)
        .expect("write definition");
    let value: Value = serde_json::from_slice(&output).expect("parse example json");

    assert_eq!(value["schema_name"], "htmlcut.extraction_definition");
    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["request"]["extraction"]["kind"], "selector");
    assert_eq!(value["request"]["extraction"]["selector"], "article a");
    assert_eq!(value["runtime"]["fetch_preflight"], "head-first");
}
