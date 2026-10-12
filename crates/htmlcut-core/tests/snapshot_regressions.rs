// SPDX-License-Identifier: MPL-2.0
//! Retained authored snapshots protect original-tree repair and lexical row values.
use htmlcut_core::{Contract, Document, Limits, Source, Value};
#[test]
fn authored_snapshots_match_upstream_repair_and_keep_complete_rows() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/pages");
    let contract = Contract::from_json(r#"{"select":":scope","read":"outer-html"}"#).unwrap();
    for name in [
        "books.html",
        "context.html",
        "malformed.html",
        "rendered.html",
        "technical.html",
    ] {
        let bytes = std::fs::read(root.join(name)).unwrap();
        let document = Document::parse(Source::new(&bytes), &Limits::default()).unwrap();
        let source = std::str::from_utf8(&bytes).unwrap();
        assert_eq!(
            document.extract(&contract).unwrap(),
            Value::String(scraper::Html::parse_document(source).root_element().html()),
            "{name}"
        );
    }
    let bytes = std::fs::read(root.join("books.html")).unwrap();
    let document = Document::parse(Source::new(&bytes), &Limits::default()).unwrap();
    let rows = Contract::from_json(r#"{"select":"article.book","match":"all","min":20,"max":20,"fields":{"title":{"select":"a","read":"attr:title"},"price":".price"}}"#).unwrap();
    let Value::List(records) = document.extract(&rows).unwrap() else {
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
