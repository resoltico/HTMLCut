// SPDX-License-Identifier: MPL-2.0
use htmlcut_core::*;

#[test]
fn public_documentation_example_has_a_regular_mutation_detector() {
    let limits = Limits::default();
    let context = ExecutionContext::new(limits.time_limit).unwrap();
    let document = Document::parse(
        Source::new(b"<h1>Snapshot</h1>"),
        &limits.preparation(),
        &context,
    )
    .unwrap();
    let contract = Contract::from_json(r#""h1""#, &context).unwrap();
    assert_eq!(
        document
            .extract(&contract, &limits.execution(), &context)
            .unwrap(),
        Value::String("Snapshot".into())
    );
}
#[test]
fn exact_engine_types_are_shareable_and_one_context_spans_phases() {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    fn share<T: Send + Sync>() {}
    share::<Document>();
    share::<Contract>();
    share::<Value>();
    share::<OutlineReport>();
    let limits = Limits::default();
    let stopped = Arc::new(AtomicBool::new(false));
    let observed = stopped.clone();
    let context = ExecutionContext::with_checkpoint(move || {
        if observed.load(Ordering::SeqCst) {
            Err(Error::limit("time", 0))
        } else {
            Ok(())
        }
    });
    let contract = Contract::from_json("\"p\"", &context).unwrap();
    let document =
        Document::parse(Source::new(b"<p>x</p>"), &limits.preparation(), &context).unwrap();
    stopped.store(true, Ordering::SeqCst);
    let error = document
        .extract(&contract, &limits.execution(), &context)
        .unwrap_err();
    assert_eq!(error.error, ErrorCode::LimitExceeded);
    assert_eq!(error.limit.as_deref(), Some("time"));
    assert_eq!(error.bound, Some(serde_json::json!(0)));
    let fresh = ExecutionContext::new(None).unwrap();
    assert_eq!(
        document
            .extract(&contract, &limits.execution(), &fresh)
            .unwrap(),
        Value::String("x".into())
    );
}
#[test]
fn quick_queries_and_json_share_independently_expected_reading_cardinality_counts() {
    let context = ExecutionContext::new(None).unwrap();
    let limits = Limits::default();
    let mut checked = 0;
    for (reading, literal) in [
        ("text", "X"),
        ("text-content", "X"),
        ("outer-html", "<p title=\"T\" href=\"/x\">X</p>"),
        ("attr:title", "T"),
        ("url:href", "https://origin.test/x"),
    ] {
        for (cardinality, name, expected_range) in [
            (QueryCardinality::One, "one", "1"),
            (QueryCardinality::Optional, "optional", "0..1"),
            (QueryCardinality::All, "all", "1.."),
        ] {
            for count in 0..=2 {
                let page = "<p title=\"T\" href=\"/x\">X</p>".repeat(count);
                let mut source = Source::new(page.as_bytes());
                source.base_url = Some("https://origin.test/");
                let document = Document::parse(source, &limits.preparation(), &context).unwrap();
                let query = QuickQuery {
                    select: "p".into(),
                    read: reading.into(),
                    cardinality,
                };
                let text =
                    serde_json::json!({"select":"p","read":reading,"match":name}).to_string();
                for contract in [
                    Contract::quick(&query, &context),
                    Contract::from_json(&text, &context),
                ] {
                    let actual =
                        document.extract(&contract.unwrap(), &limits.execution(), &context);
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
        let query = QuickQuery {
            select: "p".into(),
            read: reading.into(),
            cardinality: QueryCardinality::One,
        };
        let text = serde_json::json!({"select":"p","read":reading,"match":"one"}).to_string();
        for result in [
            Contract::quick(&query, &context),
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
        let document = Document::parse(
            Source::new(page.as_bytes()),
            &limits.preparation(),
            &context,
        )
        .unwrap();
        assert_eq!(
            document
                .extract(&contract, &limits.execution(), &context)
                .is_ok(),
            (1..=3).contains(&count)
        );
    }
}

#[test]
fn typed_admission_preserves_observed_stop_errors_and_json_rejection_facts() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    let checks = Arc::new(AtomicUsize::new(0));
    let observed = checks.clone();
    let context = ExecutionContext::with_checkpoint(move || {
        if observed.fetch_add(1, Ordering::SeqCst) == 0 {
            Ok(())
        } else {
            Err(Error::new(
                ErrorCode::Cancelled,
                "cancelled during compiler admission",
            ))
        }
    });
    let query = QuickQuery {
        select: "p".into(),
        read: "text".into(),
        cardinality: QueryCardinality::One,
    };
    let error = Contract::quick(&query, &context).err().unwrap();
    assert_eq!(error.error, ErrorCode::Cancelled);
    assert!(error.limit.is_none());
    assert!(error.bound.is_none());
    let context = ExecutionContext::new(None).unwrap();
    for (select, read) in [
        ("[".into(), "text".into()),
        ("p".into(), format!("attr:{}", "x".repeat(257))),
        ("x".repeat(Contract::MAX_BYTES), "text".into()),
        ("\u{1}".repeat(Contract::MAX_BYTES / 2), "text".into()),
    ] {
        let query = QuickQuery {
            select,
            read,
            cardinality: QueryCardinality::One,
        };
        let text =
            serde_json::json!({"select":query.select,"read":query.read,"match":"one"}).to_string();
        let typed = Contract::quick(&query, &context).err().unwrap();
        let json = Contract::from_json(&text, &context).err().unwrap();
        assert_eq!(typed.error, json.error);
        assert_eq!(
            serde_json::to_value(typed.facts).unwrap(),
            serde_json::to_value(json.facts).unwrap()
        );
    }
    let expired = ExecutionContext::new(Some(std::time::Duration::ZERO)).unwrap();
    let error = Value::Null
        .export(ExportFormat::Raw, 0, &expired)
        .unwrap_err();
    assert_eq!(error.limit.as_deref(), Some("time"));
}
