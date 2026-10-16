// SPDX-License-Identifier: MPL-2.0
//! Every executable specification case through the public library.
use htmlcut_core::{Contract, Document, Limits, OutlineOptions, Source};
use serde_json::Value;
use std::path::Path;
#[test]
fn every_case_runs_through_the_core_engine() {
    let checked = run_cases(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance"));
    println!("HTMLCUT_CONFORMANCE_CASES={checked}");
}
fn run_cases(root: &Path) -> usize {
    let mut checked = 0;
    let mut discovered = 0;
    let validator = jsonschema::validator_for(&Contract::schema()).unwrap();
    for kind in ["contract", "outline"] {
        let mut paths = std::fs::read_dir(root.join(kind))
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect::<Vec<_>>();
        paths.retain(|path| path.is_dir());
        discovered += paths.len();
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
                let text = std::fs::read_to_string(path.join("contract.json")).unwrap();
                let contract =
                    Contract::from_json(&text, &htmlcut_core::ExecutionContext::new(None).unwrap());
                if contract.is_ok() {
                    assert!(
                        validator.is_valid(&serde_json::from_str::<Value>(&text).unwrap()),
                        "schema refused {path:?}"
                    );
                }
                if ["80-", "83-", "84-", "90-"].iter().any(|prefix| {
                    path.file_name()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .starts_with(prefix)
                }) {
                    assert!(
                        !validator.is_valid(&serde_json::from_str::<Value>(&text).unwrap()),
                        "schema admitted {path:?}"
                    );
                }
                contract
                    .and_then(|contract| {
                        let context =
                            htmlcut_core::ExecutionContext::new(limits.time_limit).unwrap();
                        let value = Document::parse(source, &limits.preparation(), &context)?
                            .extract(&contract, &limits.execution(), &context)?;
                        if options["raw"] == true {
                            value.export(
                                htmlcut_core::ExportFormat::Raw,
                                limits.max_output_bytes,
                                &context,
                            )?;
                        }
                        Ok(value)
                    })
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
                Document::parse(
                    source,
                    &limits.preparation(),
                    &htmlcut_core::ExecutionContext::new(limits.time_limit).unwrap(),
                )
                .and_then(|d| {
                    d.outline(
                        &outline,
                        &limits.execution(),
                        &htmlcut_core::ExecutionContext::new((limits).time_limit).unwrap(),
                    )
                    .and_then(|report| {
                        report.render(
                            limits.max_output_bytes,
                            &htmlcut_core::ExecutionContext::new((limits).time_limit).unwrap(),
                        )
                    })
                })
                .map(Value::String)
            };
            if path.join("error.json").exists() {
                let error = actual.unwrap_err();
                plain_message(&error);
                let mut actual = serde_json::to_value(error).unwrap();
                let mut expected: Value =
                    serde_json::from_slice(&std::fs::read(path.join("error.json")).unwrap())
                        .unwrap();
                actual.as_object_mut().unwrap().remove("message");
                expected.as_object_mut().unwrap().remove("message");
                if options["partial_error_facts"] == true {
                    for (key, value) in expected.as_object().unwrap() {
                        assert_eq!(actual.get(key), Some(value), "{path:?}: {key}");
                    }
                } else {
                    assert_eq!(actual, expected, "{path:?}");
                }
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
    assert!(discovered > 0, "the conformance corpus is empty");
    assert_eq!(checked, discovered);
    checked
}

#[test]
fn refusal_messages_explain_facts_without_debug_renderings() {
    for selector in ["[id=", "p:contains(x)", "div:has(p:has(a))", "p >"] {
        let error = Contract::from_json(
            &serde_json::json!({"select":selector}).to_string(),
            &htmlcut_core::ExecutionContext::new(None).unwrap(),
        )
        .err()
        .unwrap();
        plain_message(&error);
        assert!(error.message.contains("column"), "{}", error.message);
    }
    let error = Contract::from_json(
        r#"{"read":"html"}"#,
        &htmlcut_core::ExecutionContext::new(None).unwrap(),
    )
    .err()
    .unwrap();
    for reading in [
        "text",
        "text-content",
        "outer-html",
        "attr:NAME",
        "url:NAME",
    ] {
        assert!(error.message.contains(reading));
    }
    let document = Document::parse(
        Source::new(b"<p>x</p>"),
        &Limits::default().preparation(),
        &htmlcut_core::ExecutionContext::new(Limits::default().time_limit).unwrap(),
    )
    .unwrap();
    let mut options = OutlineOptions::default();
    options.select = Some("[id=".into());
    let error = document
        .outline(
            &options,
            &htmlcut_core::Limits::default().execution(),
            &htmlcut_core::ExecutionContext::new((htmlcut_core::Limits::default()).time_limit)
                .unwrap(),
        )
        .and_then(|report| {
            report.render(
                htmlcut_core::Limits::default().max_output_bytes,
                &htmlcut_core::ExecutionContext::new((htmlcut_core::Limits::default()).time_limit)
                    .unwrap(),
            )
        })
        .unwrap_err();
    plain_message(&error);
    assert_eq!(error.select.as_deref(), Some("[id="));
    assert!(error.path.is_none());
}

#[test]
fn corpus_runner_refuses_empty_or_incomplete_corpora() {
    let directory = tempfile::tempdir().unwrap();
    for kind in ["contract", "outline"] {
        std::fs::create_dir(directory.path().join(kind)).unwrap();
    }
    assert!(std::panic::catch_unwind(|| run_cases(directory.path())).is_err());
    let case = directory.path().join("contract/incomplete");
    std::fs::create_dir(&case).unwrap();
    std::fs::write(case.join("page.html"), b"<h1>x</h1>").unwrap();
    std::fs::write(case.join("contract.json"), b"\"h1\"").unwrap();
    assert!(std::panic::catch_unwind(|| run_cases(directory.path())).is_err());
}

fn plain_message(error: &htmlcut_core::Error) {
    for rendering in ["Some(", "None", "ParseError", "SourceLocation", " { "] {
        assert!(!error.message.contains(rendering), "{}", error.message);
    }
}

#[test]
fn documented_selector_examples_have_literal_dialect_evidence() {
    use std::collections::BTreeSet;
    fn selectors(value: &Value, output: &mut BTreeSet<String>) {
        match value {
            Value::String(text) => {
                output.insert(text.clone());
            }
            Value::Object(map) => {
                for key in ["select", "until"] {
                    if let Some(Value::String(text)) = map.get(key) {
                        output.insert(text.clone());
                    }
                }
                if let Some(Value::Array(excluded)) = map.get("exclude") {
                    for value in excluded {
                        selectors(value, output);
                    }
                }
                for key in ["where", "expect"] {
                    if let Some(Value::Array(predicates)) = map.get(key) {
                        for predicate in predicates {
                            selectors(predicate, output);
                        }
                    }
                }
                if let Some(Value::Object(fields)) = map.get("fields") {
                    for value in fields.values() {
                        selectors(value, output);
                    }
                }
            }
            Value::Null | Value::Bool(_) | Value::Number(_) | Value::Array(_) => (),
        }
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let text = std::fs::read_to_string(root.join("docs/contract.md")).unwrap();
    let accepted = text
        .split("**Accepted selectors.**")
        .nth(1)
        .unwrap()
        .split("**Case.**")
        .next()
        .unwrap();
    let examples = accepted
        .split('`')
        .enumerate()
        .filter(|(i, _)| i % 2 == 1)
        .map(|(_, text)| text.to_owned())
        .collect::<BTreeSet<_>>();
    assert!(!examples.is_empty());
    let mut tested = BTreeSet::new();
    for entry in std::fs::read_dir(root.join("conformance/contract")).unwrap() {
        let path = entry.unwrap().path();
        if path.file_name().unwrap().to_string_lossy().starts_with('1') {
            let value: Value =
                serde_json::from_slice(&std::fs::read(path.join("contract.json")).unwrap())
                    .unwrap();
            selectors(&value, &mut tested);
        }
    }
    assert!(
        examples.is_subset(&tested),
        "documented selectors lack conformance evidence: {:?}",
        examples.difference(&tested).collect::<Vec<_>>()
    );
}
