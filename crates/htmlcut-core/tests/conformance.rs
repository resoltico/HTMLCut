use support::{DocumentFixture, TestOptions, parse_fixture};
mod support;
// SPDX-License-Identifier: MPL-2.0
// Every executable specification case through the public library.
use htmlcut_core::{Contract, OutlineOptions};
use serde_json::Value;
use std::path::Path;
#[test]
fn every_case_runs_through_the_core_engine() {
    let checked = run_cases(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance"));
    println!("HTMLCUT_CONFORMANCE_CASES={checked}");
}
use htmlcut_conformance as fixture;
fn run_cases(root: &Path) -> usize {
    let cases = fixture::discover(root);
    for case in &cases {
        run_case(case);
        println!("HTMLCUT_CONFORMANCE_ID={}", case.identity());
    }
    cases.len()
}
fn run_case(case: &fixture::Case) {
    let options = &case.options;
    let mut limits = TestOptions::default();
    if let Some(n) = options.max_input_bytes {
        limits.max_input_bytes = n;
    }
    if let Some(n) = options.max_nodes {
        limits.max_nodes = n;
    }
    if let Some(n) = options.max_reading_bytes {
        limits.max_reading_bytes = n;
    }
    if let Some(n) = options.max_output_bytes {
        limits.max_output_bytes = n;
    }
    let context = support::test_stop(limits.time_limit).unwrap();
    let mut source = DocumentFixture::new(&case.page);
    source.encoding = options.encoding.as_deref();
    source.base_url = options.base_url.as_deref();
    let actual = if let Some(text) = &case.contract {
        Contract::from_json(text, &context).and_then(|contract| {
            let format = if options.raw {
                htmlcut_core::ExportFormat::Raw
            } else {
                htmlcut_core::ExportFormat::Json
            };
            contract.supports(format)?;
            let value = parse_fixture(source, &limits.preparation(), &context)?.extract(
                &contract,
                format,
                &limits.execution(),
                &context,
            )?;
            let bytes = value.export(format, limits.max_output_bytes, &context)?;
            Ok((Some(serde_json::to_value(value).unwrap()), bytes))
        })
    } else {
        let config = case.outline.as_ref().unwrap();
        let mut outline = OutlineOptions::default();
        outline.select = config.select.clone();
        if let Some(n) = config.depth {
            outline.depth = n;
        }
        if let Some(n) = config.max_lines {
            outline.max_lines = n;
        }
        outline.attrs = config.attrs;
        outline.validate(&context).and_then(|()| {
            parse_fixture(source, &limits.preparation(), &context)?
                .outline(&outline, &limits.execution(), &context)?
                .render(limits.max_output_bytes, &context)
                .map(|text| (None, text.into_bytes()))
        })
    };
    if case.error.is_some() {
        let error = actual.unwrap_err();
        plain_message(&error);
        case.assert_error(serde_json::to_value(error).unwrap());
    } else {
        let (value, bytes) = actual.unwrap_or_else(|e| panic!("{:?}: {e}", case.path));
        assert_eq!(value, case.expected, "{:?}", case.path);
        case.assert_bytes(&bytes);
    }
}

#[test]
fn compiling_literal_contracts_have_schema_witnesses() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance");
    let validator = jsonschema::validator_for(&Contract::schema()).unwrap();
    let context = support::test_stop(None).unwrap();
    for case in fixture::discover(&root) {
        if let Some(text) = &case.contract
            && Contract::from_json(text, &context).is_ok()
        {
            assert!(
                validator.is_valid(&serde_json::from_str::<Value>(text).unwrap()),
                "{:?}",
                case.path
            );
        }
    }
}

#[test]
fn refusal_messages_explain_facts_without_debug_renderings() {
    for selector in ["[id=", "p:contains(x)", "div:has(p:has(a))", "p >"] {
        let error = Contract::from_json(
            &serde_json::json!({"select":selector}).to_string(),
            &support::test_stop(None).unwrap(),
        )
        .err()
        .unwrap();
        plain_message(&error);
        assert!(
            error.to_string().contains("column"),
            "{}",
            error.to_string()
        );
    }
    let error = Contract::from_json(r#"{"read":"html"}"#, &support::test_stop(None).unwrap())
        .err()
        .unwrap();
    for reading in [
        "text",
        "text-content",
        "outer-html",
        "attr:NAME",
        "url:NAME",
    ] {
        assert!(error.to_string().contains(reading));
    }
    let document = parse_fixture(
        DocumentFixture::new(b"<p>x</p>"),
        &TestOptions::default().preparation(),
        &support::test_stop(TestOptions::default().time_limit).unwrap(),
    )
    .unwrap();
    let mut options = OutlineOptions::default();
    options.select = Some("[id=".into());
    let error = document
        .outline(
            &options,
            &support::TestOptions::default().execution(),
            &support::test_stop((support::TestOptions::default()).time_limit).unwrap(),
        )
        .and_then(|report| {
            report.render(
                support::TestOptions::default().max_output_bytes,
                &support::test_stop((support::TestOptions::default()).time_limit).unwrap(),
            )
        })
        .unwrap_err();
    plain_message(&error);
    assert_eq!(
        serde_json::to_value(&error).unwrap()["select"].as_str(),
        Some("[id=")
    );
    assert!(serde_json::to_value(&error).unwrap().get("path").is_none());
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
    for rendering in [
        "Some(",
        "None",
        "ParseError",
        "DocumentFixtureLocation",
        " { ",
    ] {
        assert!(
            !error.to_string().contains(rendering),
            "{}",
            error.to_string()
        );
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
    for case in fixture::discover(&root.join("conformance")) {
        if case.error.is_none()
            && let Some(contract) = case.contract
        {
            selectors(&serde_json::from_str(&contract).unwrap(), &mut tested);
        }
    }
    assert!(
        examples.is_subset(&tested),
        "documented selectors lack conformance evidence: {:?}",
        examples.difference(&tested).collect::<Vec<_>>()
    );
}

#[test]
fn core_adapter_rejects_wrong_json_and_raw_oracles() {
    for (id, oracle) in [
        ("cardinality/one", "expected.json"),
        ("raw/optional-null", "expected.raw"),
    ] {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/contract");
        let dir = tempfile::tempdir().unwrap();
        for entry in std::fs::read_dir(root.join(id)).unwrap() {
            let path = entry.unwrap().path();
            std::fs::copy(&path, dir.path().join(path.file_name().unwrap())).unwrap();
        }
        std::fs::write(
            dir.path().join(oracle),
            if oracle.ends_with("json") {
                b"\"wrong\"\n".as_slice()
            } else {
                b"wrong\n"
            },
        )
        .unwrap();
        let case = fixture::load(dir.path(), "contract");
        assert!(std::panic::catch_unwind(|| run_case(&case)).is_err());
    }
}
