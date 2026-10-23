use support::{DocumentFixture, TestOptions, parse_fixture};
use support::{QueryFixture, compile_query};
mod support;
// SPDX-License-Identifier: MPL-2.0
use htmlcut_core::*;

#[test]
fn public_documentation_example_has_a_regular_mutation_detector() {
    let limits = TestOptions::default();
    let context = support::test_stop(limits.time_limit).unwrap();
    let document = parse_fixture(
        DocumentFixture::new(b"<h1>Snapshot</h1>"),
        &limits.preparation(),
        &context,
    )
    .unwrap();
    let contract = Contract::from_json(r#""h1""#, &context).unwrap();
    assert_eq!(
        document
            .extract(
                &contract,
                htmlcut_core::ExportFormat::Json,
                &limits.execution(),
                &context
            )
            .unwrap(),
        Value::String("Snapshot".into())
    );
}
#[test]
fn immutable_engine_types_share_one_stop_across_phases() {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    fn share<T: Send + Sync>() {}
    share::<Document>();
    share::<Contract>();
    share::<Value>();
    share::<OutlineReport>();
    let limits = TestOptions::default();
    let stopped = Arc::new(AtomicBool::new(false));
    let observed = stopped.clone();
    let context = Stop::none().with_cancel(observed);
    let contract = Contract::from_json("\"p\"", &context).unwrap();
    let document = parse_fixture(
        DocumentFixture::new(b"<p>x</p>"),
        &limits.preparation(),
        &context,
    )
    .unwrap();
    stopped.store(true, Ordering::SeqCst);
    let error = document
        .extract(
            &contract,
            htmlcut_core::ExportFormat::Json,
            &limits.execution(),
            &context,
        )
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::Cancelled);
    let fresh = support::test_stop(None).unwrap();
    assert_eq!(
        document
            .extract(
                &contract,
                htmlcut_core::ExportFormat::Json,
                &limits.execution(),
                &fresh
            )
            .unwrap(),
        Value::String("x".into())
    );
}
#[test]
fn typed_specs_and_json_share_independent_reading_cardinality_oracles() {
    let context = support::test_stop(None).unwrap();
    let limits = TestOptions::default();
    let mut checked = 0;
    for (reading, literal) in [
        ("text", "X"),
        ("text-content", "X"),
        ("outer-html", "<p title=\"T\" href=\"/x\">X</p>"),
        ("attr:title", "T"),
        ("url:href", "https://origin.test/x"),
    ] {
        for (cardinality, name, expected_range) in [
            (Match::One, "one", serde_json::json!({"min":1,"max":1})),
            (
                Match::Optional,
                "optional",
                serde_json::json!({"min":0,"max":1}),
            ),
            (Match::All, "all", serde_json::json!({"min":1})),
        ] {
            for count in 0..=2 {
                let page = "<p title=\"T\" href=\"/x\">X</p>".repeat(count);
                let mut source = DocumentFixture::new(page.as_bytes());
                source.base_url = Some("https://origin.test/");
                let document = parse_fixture(source, &limits.preparation(), &context).unwrap();
                let query = QueryFixture {
                    select: "p".into(),
                    read: reading.into(),
                    cardinality,
                };
                let text =
                    serde_json::json!({"select":"p","read":reading,"match":name}).to_string();
                for contract in [
                    compile_query(&query, &context),
                    Contract::from_json(&text, &context),
                ] {
                    let actual = document.extract(
                        &contract.unwrap(),
                        htmlcut_core::ExportFormat::Json,
                        &limits.execution(),
                        &context,
                    );
                    if (count == 0 && name != "optional") || (count == 2 && name != "all") {
                        let error = actual.unwrap_err();
                        let mut facts = serde_json::to_value(error).unwrap();
                        facts.as_object_mut().unwrap().remove("message");
                        assert_eq!(
                            facts,
                            serde_json::json!({"error":"count_mismatch","path":"$","select":"p","found":count,"expected":expected_range})
                        );
                    } else {
                        let expected = if name == "all" {
                            serde_json::Value::Array(vec![literal.into(); count])
                        } else if count == 0 {
                            serde_json::Value::Null
                        } else {
                            literal.into()
                        };
                        let mut bytes = serde_json::to_vec(&expected).unwrap();
                        bytes.push(b'\n');
                        let actual = actual.unwrap();
                        assert_eq!(serde_json::to_value(&actual).unwrap(), expected);
                        assert_eq!(
                            actual
                                .export(ExportFormat::Json, limits.max_output_bytes, &context)
                                .unwrap(),
                            bytes
                        );
                    }
                }
                checked += 1;
            }
        }
    }
    assert_eq!(checked, 45);
    for reading in ["html", "attr:"] {
        let query = QueryFixture {
            select: "p".into(),
            read: reading.into(),
            cardinality: Match::One,
        };
        let text = serde_json::json!({"select":"p","read":reading,"match":"one"}).to_string();
        for result in [
            compile_query(&query, &context),
            Contract::from_json(&text, &context),
        ] {
            let error = result.err().expect("invalid reading admitted");
            let mut facts = serde_json::to_value(error).unwrap();
            facts.as_object_mut().unwrap().remove("message");
            assert_eq!(
                facts,
                serde_json::json!({"error":"invalid_contract","path":"$.read"}),
                "{text}"
            );
        }
    }
    let contract =
        Contract::from_json(r#"{"select":"p","match":"all","min":1,"max":3}"#, &context).unwrap();
    for count in 0..=4 {
        let page = "<p>X</p>".repeat(count);
        let document = parse_fixture(
            DocumentFixture::new(page.as_bytes()),
            &limits.preparation(),
            &context,
        )
        .unwrap();
        assert_eq!(
            document
                .extract(
                    &contract,
                    htmlcut_core::ExportFormat::Json,
                    &limits.execution(),
                    &context
                )
                .is_ok(),
            (1..=3).contains(&count)
        );
    }
}

#[test]
fn typed_admission_preserves_observed_stop_errors_and_json_rejection_facts() {
    let context = Stop::none().with_cancel(std::sync::Arc::new(
        std::sync::atomic::AtomicBool::new(true),
    ));
    let query = QueryFixture {
        select: "p".into(),
        read: "text".into(),
        cardinality: Match::One,
    };
    let error = compile_query(&query, &context).err().unwrap();
    assert_eq!(error.code(), ErrorCode::Cancelled);
    assert!(serde_json::to_value(&error).unwrap().get("limit").is_none());
    assert!(
        serde_json::to_value(&error)
            .unwrap()
            .get("bound")
            .cloned()
            .is_none()
    );
    let context = support::test_stop(None).unwrap();
    for (select, read) in [
        ("[".into(), "text".into()),
        ("p".into(), format!("attr:{}", "x".repeat(257))),
        ("x".repeat(Contract::MAX_BYTES), "text".into()),
        ("\u{1}".repeat(Contract::MAX_BYTES / 2), "text".into()),
    ] {
        let query = QueryFixture {
            select,
            read,
            cardinality: Match::One,
        };
        let text =
            serde_json::json!({"select":query.select,"read":query.read,"match":"one"}).to_string();
        let typed = compile_query(&query, &context).err().unwrap();
        let json = Contract::from_json(&text, &context).err().unwrap();
        assert_eq!(typed.code(), json.code());
        let facts = |error: &Error| {
            let mut value = serde_json::to_value(error).unwrap();
            value.as_object_mut().unwrap().remove("message");
            value
        };
        assert_eq!(facts(&typed), facts(&json));
    }
    let expired = support::test_stop(Some(std::time::Duration::ZERO)).unwrap();
    let error = Value::Null
        .export(ExportFormat::Raw, 0, &expired)
        .unwrap_err();
    assert_eq!(
        serde_json::to_value(&error).unwrap()["limit"].as_str(),
        Some("time")
    );
}
