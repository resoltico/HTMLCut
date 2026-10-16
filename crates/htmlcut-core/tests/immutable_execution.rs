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
fn exact_engine_types_are_shareable_and_one_deadline_spans_phases() {
    fn share<T: Send + Sync>() {}
    share::<Document>();
    share::<Contract>();
    share::<Value>();
    share::<OutlineReport>();
    let limits = Limits::default();
    let context = ExecutionContext::new(Some(std::time::Duration::from_millis(30))).unwrap();
    let contract = Contract::from_json("\"p\"", &context).unwrap();
    let document =
        Document::parse(Source::new(b"<p>x</p>"), &limits.preparation(), &context).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(40));
    assert_eq!(
        document
            .extract(&contract, &limits.execution(), &context)
            .unwrap_err()
            .limit
            .as_deref(),
        Some("time")
    );
    let fresh = ExecutionContext::new(None).unwrap();
    assert_eq!(
        document
            .extract(&contract, &limits.execution(), &fresh)
            .unwrap(),
        Value::String("x".into())
    );
}
#[test]
fn quick_queries_and_json_share_reading_cardinality_and_rejection_rules() {
    let context = ExecutionContext::new(None).unwrap();
    let limits = Limits::default();
    let document = Document::parse(
        Source::new(b"<p title='x'>one</p><p title='y'>two</p>"),
        &limits.preparation(),
        &context,
    )
    .unwrap();
    for reading in [
        "text",
        "text-content",
        "outer-html",
        "attr:title",
        "url:title",
        "html",
        "attr:",
    ] {
        for (cardinality, name) in [
            (QueryCardinality::One, "one"),
            (QueryCardinality::Optional, "optional"),
            (QueryCardinality::All, "all"),
        ] {
            let query = QuickQuery {
                select: "p".into(),
                read: reading.into(),
                cardinality,
            };
            let quick = Contract::quick(&query, &context)
                .and_then(|contract| document.extract(&contract, &limits.execution(), &context));
            let text = serde_json::json!({"select":"p","read":reading,"match":name}).to_string();
            let json = Contract::from_json(&text, &context)
                .and_then(|contract| document.extract(&contract, &limits.execution(), &context));
            match (quick, json) {
                (Ok(a), Ok(b)) => assert_eq!(a, b),
                (Err(a), Err(b)) => {
                    assert_eq!(a.error, b.error);
                    assert_eq!(a.to_json_line(), b.to_json_line());
                }
                _ => panic!("typed/JSON drift for {text}"),
            }
        }
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
