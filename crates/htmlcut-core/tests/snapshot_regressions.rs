// SPDX-License-Identifier: MPL-2.0
//! Retained authored snapshots protect original-tree repair and lexical row values.
use htmlcut_core::{Contract, Document, Limits, Source, Value};
#[test]
fn authored_book_rows_keep_complete_lexical_values() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../conformance/contract/authored-books");
    let bytes = std::fs::read(root.join("page.html")).unwrap();
    let document = Document::parse(
        Source::new(&bytes),
        &Limits::default().preparation(),
        &htmlcut_core::ExecutionContext::new(Limits::default().time_limit).unwrap(),
    )
    .unwrap();
    let rows = Contract::from_json(r#"{"select":"article.book","match":"all","min":20,"max":20,"fields":{"title":{"select":"a","read":"attr:title"},"price":".price"}}"#, &htmlcut_core::ExecutionContext::new(None).unwrap()).unwrap();
    let Value::List(records) = document
        .extract(
            &rows,
            &htmlcut_core::Limits::default().execution(),
            &htmlcut_core::ExecutionContext::new((htmlcut_core::Limits::default()).time_limit)
                .unwrap(),
        )
        .unwrap()
    else {
        panic!("not records");
    };
    assert_eq!(records.len(), 20);
    for (index, record) in records.iter().enumerate() {
        let Value::Record(fields) = record else {
            panic!("not a record");
        };
        assert_eq!(
            fields["title"],
            Value::String(format!("Complete title {} — Alice's book", index + 1))
        );
        assert_eq!(
            fields["price"],
            Value::String(format!("GBP {}.00", index + 1))
        );
    }
}
