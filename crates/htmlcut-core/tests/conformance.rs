// SPDX-License-Identifier: MPL-2.0
//! Every executable specification case through the public library.
use htmlcut_core::{Contract, Document, Limits, OutlineOptions, Source};
use serde_json::Value;
use std::path::Path;
#[test]
fn every_case_runs_through_the_finished_library() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance");
    let mut checked = 0;
    for kind in ["contract", "outline"] {
        let mut paths = std::fs::read_dir(root.join(kind))
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect::<Vec<_>>();
        paths.sort();
        for path in paths {
            let bytes = std::fs::read(path.join("page.html")).unwrap();
            let options: Value = std::fs::read(path.join("options.json"))
                .ok()
                .map(|v| serde_json::from_slice(&v).unwrap())
                .unwrap_or_default();
            let mut source = Source::new(&bytes);
            source.encoding = options["encoding"].as_str();
            source.base_url = options["base_url"].as_str();
            let mut limits = Limits::default();
            if let Some(n) = options["max_nodes"].as_u64() {
                limits.max_nodes = n as usize;
            }
            let actual = if kind == "contract" {
                Contract::from_json(&std::fs::read_to_string(path.join("contract.json")).unwrap())
                    .and_then(|contract| Document::parse(source, &limits)?.extract(&contract))
                    .map(|v| serde_json::to_value(v).unwrap())
            } else {
                let config: Value =
                    serde_json::from_slice(&std::fs::read(path.join("outline.json")).unwrap())
                        .unwrap();
                let mut outline = OutlineOptions::default();
                outline.select = config["select"].as_str().map(String::from);
                if let Some(n) = config["depth"].as_u64() {
                    outline.depth = n as usize;
                }
                if let Some(n) = config["max_lines"].as_u64() {
                    outline.max_lines = n as usize;
                }
                outline.attrs = config["attrs"].as_bool().unwrap_or(false);
                Document::parse(source, &limits)
                    .and_then(|d| d.outline(&outline))
                    .map(Value::String)
            };
            if path.join("error.json").exists() {
                let error = actual.unwrap_err();
                let mut actual = serde_json::to_value(error).unwrap();
                let mut expected: Value =
                    serde_json::from_slice(&std::fs::read(path.join("error.json")).unwrap())
                        .unwrap();
                actual.as_object_mut().unwrap().remove("message");
                expected.as_object_mut().unwrap().remove("message");
                assert_eq!(actual, expected, "{path:?}");
            } else if kind == "contract" {
                let expected: Value =
                    serde_json::from_slice(&std::fs::read(path.join("expected.json")).unwrap())
                        .unwrap();
                assert_eq!(actual.unwrap(), expected, "{path:?}");
            } else {
                assert_eq!(
                    actual.unwrap().as_str().unwrap().as_bytes(),
                    std::fs::read(path.join("expected.txt")).unwrap(),
                    "{path:?}"
                );
            }
            checked += 1;
        }
    }
    assert_eq!(checked, 62);
}

#[test]
fn refusal_messages_explain_facts_without_debug_renderings() {
    fn plain(error: &htmlcut_core::Error) {
        for rendering in ["Some(", "None", "ParseError", "SourceLocation", " { "] {
            assert!(!error.message.contains(rendering), "{}", error.message);
        }
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/contract");
    let mut checked = 0;
    for entry in std::fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if !path.join("error.json").exists() {
            continue;
        }
        let bytes = std::fs::read(path.join("page.html")).unwrap();
        let options: Value = std::fs::read(path.join("options.json"))
            .ok()
            .map(|v| serde_json::from_slice(&v).unwrap())
            .unwrap_or_default();
        let mut source = Source::new(&bytes);
        source.encoding = options["encoding"].as_str();
        source.base_url = options["base_url"].as_str();
        let mut limits = Limits::default();
        if let Some(n) = options["max_nodes"].as_u64() {
            limits.max_nodes = n as usize;
        }
        let result =
            Contract::from_json(&std::fs::read_to_string(path.join("contract.json")).unwrap())
                .and_then(|c| Document::parse(source, &limits)?.extract(&c));
        plain(&result.unwrap_err());
        checked += 1;
    }
    assert!(checked > 0);
    for selector in ["[id=", "p:contains(x)", "div:has(p:has(a))", "p >"] {
        let error = Contract::from_json(&serde_json::json!({"select":selector}).to_string())
            .err()
            .unwrap();
        plain(&error);
        assert!(error.message.contains("column"), "{}", error.message);
    }
    let error = Contract::from_json(r#"{"read":"html"}"#).err().unwrap();
    for reading in [
        "text",
        "text-content",
        "outer-html",
        "attr:NAME",
        "url:NAME",
    ] {
        assert!(error.message.contains(reading));
    }
    let document = Document::parse(Source::new(b"<p>x</p>"), &Limits::default()).unwrap();
    let mut options = OutlineOptions::default();
    options.select = Some("[id=".into());
    let error = document.outline(&options).unwrap_err();
    plain(&error);
    assert_eq!(error.select.as_deref(), Some("[id="));
    assert!(error.path.is_none());
}
