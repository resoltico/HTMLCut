// SPDX-License-Identifier: MPL-2.0
//! Real CLI authoring uses the same closed contract as structured plans and the core.
mod support;
use serde_json::json;
use support::invoke;

#[test]
fn optional_inline_field_preserves_absence_and_rejects_ambiguous_or_missing_attributes() {
    let args = [
        "extract",
        "--stdin",
        "--css",
        "article",
        "--match",
        "all",
        "--field",
        "score?",
        ".score",
        "attribute:data-score",
    ];
    let result = invoke(
        &args,
        b"<article><b class=score data-score=10></b></article><article></article>",
    );
    assert!(result.status.success());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&result.stdout).unwrap(),
        json!([{"score":"10"},{"score":null}])
    );

    for source in [
        b"<article><b class=score data-score=10></b><b class=score data-score=20></b></article>"
            .as_slice(),
        b"<article><b class=score></b></article>".as_slice(),
    ] {
        let result = invoke(&args, source);
        assert_eq!(result.status.code(), Some(3));
        assert!(result.stdout.is_empty());
    }
}

#[test]
fn normalized_text_separates_blocks_and_breaks_without_altering_literal_text() {
    let source = b"<article><dl><dt>Name</dt><dd>Alice</dd><dt>Amount</dt><dd>180</dd></dl><p>a<br>c</p></article>";
    let normalized = invoke(
        &[
            "extract",
            "--stdin",
            "--css",
            "article",
            "--read",
            "normalized_text",
        ],
        source,
    );
    assert_eq!(normalized.stdout, b"[\"Name Alice Amount 180 a c\"]\n");
    let literal = invoke(&["extract", "--stdin", "--css", "article"], source);
    assert_eq!(literal.stdout, b"[\"NameAliceAmount180ac\"]\n");
    let foreign = invoke(
        &[
            "extract",
            "--stdin",
            "--css",
            "article",
            "--read",
            "normalized_text",
        ],
        b"<article>a<svg><text>b</text></svg>c</article>",
    );
    assert_eq!(foreign.stdout, b"[\"abc\"]\n");
}

#[test]
fn fixed_arity_fields_keep_css_and_attribute_suffixes_opaque() {
    let source = b"<article><a id='a@b' title='Title'>x</a><p class=price> 10 </p></article>";
    let result = invoke(
        &[
            "extract",
            "--stdin",
            "--css",
            "article",
            "--field",
            "title",
            "[id='a@b']",
            "attribute:title",
            "--field",
            "price",
            ".price",
            "normalized_text",
        ],
        source,
    );
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&result.stdout).unwrap(),
        json!([{"title":"Title","price":"10"}])
    );
}

#[test]
fn scalar_resolved_attribute_read_needs_no_plan_file() {
    let result = invoke(
        &[
            "extract",
            "--stdin",
            "--css",
            "a",
            "--read",
            "resolved_attribute:href",
            "--base-url",
            "https://example.test/docs/",
        ],
        b"<a href=guide>x</a>",
    );
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(result.stdout, b"[\"https://example.test/docs/guide\"]\n");
}

#[test]
fn sibling_group_inline_request_has_complete_named_values() {
    let result = invoke(
        &[
            "extract",
            "--stdin",
            "--css",
            "tr.entry",
            "--field",
            "title",
            ":scope td",
            "dom_text",
            "--field",
            "score",
            ":scope + tr .score",
            "normalized_text",
            "--following-siblings",
            "1",
        ],
        b"<table><tr class=entry><td>Title</td></tr><tr><td class=score> 10 </td></tr></table>",
    );
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&result.stdout).unwrap(),
        json!([{"title":"Title","score":"10"}])
    );
}

#[test]
fn retired_ambiguous_and_invalid_reading_requests_have_no_success_data() {
    for arguments in [
        vec![
            "extract",
            "--stdin",
            "--css",
            "p",
            "--projection",
            "markdown",
        ],
        vec!["extract", "--stdin", "--css", "p", "--attribute", "id"],
        vec![
            "extract",
            "--stdin",
            "--css",
            "p",
            "--read",
            "UNDECLARED_SECRET",
        ],
        vec![
            "extract", "--stdin", "--css", "p", "--field", "a", "p", "source",
        ],
        vec![
            "extract", "--stdin", "--css", "p", "--field", "a", "p", "dom_text", "--field", "a",
            "p", "dom_text",
        ],
        vec![
            "extract",
            "--stdin",
            "--css",
            "p",
            "--following-siblings",
            "1",
        ],
        vec!["extract", "--stdin", "--css", "p", "--field", "a", "p"],
    ] {
        let result = invoke(&arguments, b"<p>x</p>");
        assert_eq!(result.status.code(), Some(2));
        assert!(result.stdout.is_empty());
        assert!(!String::from_utf8_lossy(&result.stderr).contains("UNDECLARED_SECRET"));
    }
}

#[test]
fn invalid_reading_request_rejects_large_input_before_consuming_it() {
    let result = invoke(
        &["extract", "--stdin", "--css", "p", "--read", "undeclared"],
        &vec![b'x'; 1024 * 1024],
    );
    assert_eq!(result.status.code(), Some(2));
    assert!(result.stdout.is_empty());
}

#[test]
fn resolved_markdown_and_field_count_limit_use_the_public_grammar() {
    let result = invoke(
        &[
            "extract",
            "--stdin",
            "--css",
            "p",
            "--read",
            "resolved_markdown",
            "--base-url",
            "https://example.test/",
        ],
        b"<p><a href=next>Next</a></p>",
    );
    assert!(result.status.success());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&result.stdout).unwrap(),
        json!(["[Next](<https://example.test/next>)"])
    );
    let mut arguments = vec![
        "extract".to_string(),
        "--stdin".into(),
        "--css".into(),
        "article".into(),
    ];
    for index in 0..65 {
        arguments.extend([
            "--field".into(),
            format!("field_{index}"),
            ":scope".into(),
            "dom_text".into(),
        ]);
    }
    let arguments = arguments.iter().map(String::as_str).collect::<Vec<_>>();
    let result = invoke(&arguments, b"<article>x</article>");
    assert_eq!(result.status.code(), Some(2));
    assert!(result.stdout.is_empty());
}

#[test]
fn structured_and_inline_authoring_bind_identical_plan_and_receipt() {
    let directory = tempfile::tempdir().unwrap();
    let plan = directory.path().join("fields.json");
    let inline_receipt = directory.path().join("inline.receipt.json");
    let structured_receipt = directory.path().join("structured.receipt.json");
    std::fs::write(&plan, serde_json::to_vec(&json!({
        "schema":"htmlcut.extraction.plan", "version":htmlcut_core::SCHEMA_VERSION,
        "strategy":{"kind":"css","selector":"article"},
        "projection":{"kind":"records","following_siblings":1,"fields":[
            {"name":"title","selector":"a","projection":{"kind":"attribute","name":"title"}},
            {"name":"price","selector":":scope + aside .price","transforms":[{"kind":"normalize_whitespace"}]},
            {"name":"url","selector":"a","projection":{"kind":"attribute","name":"href"},"transforms":[{"kind":"resolve_urls"}]}
        ]}
    })).unwrap()).unwrap();
    let source = "<main><article><a title=T href=next>x</a></article><aside><span class=price>\u{a0}10\u{2003}</span></aside></main>".as_bytes();
    let inline = invoke(
        &[
            "extract",
            "--stdin",
            "--css",
            "article",
            "--following-siblings",
            "1",
            "--field",
            "title",
            "a",
            "attribute:title",
            "--field",
            "price",
            ":scope + aside .price",
            "normalized_text",
            "--field",
            "url",
            "a",
            "resolved_attribute:href",
            "--base-url",
            "https://example.test/",
            "--receipt",
            inline_receipt.to_str().unwrap(),
        ],
        source,
    );
    let structured = invoke(
        &[
            "extract",
            "--stdin",
            "--plan",
            plan.to_str().unwrap(),
            "--base-url",
            "https://example.test/",
            "--receipt",
            structured_receipt.to_str().unwrap(),
        ],
        source,
    );
    assert!(
        inline.status.success(),
        "{}",
        String::from_utf8_lossy(&inline.stderr)
    );
    assert!(
        structured.status.success(),
        "{}",
        String::from_utf8_lossy(&structured.stderr)
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&inline.stdout).unwrap(),
        json!([{"title":"T","price":"10","url":"https://example.test/next"}])
    );
    assert_eq!(inline.stdout, structured.stdout);
    assert_eq!(
        std::fs::read(inline_receipt).unwrap(),
        std::fs::read(structured_receipt).unwrap()
    );
}

#[test]
fn valid_field_limit_boundary_keeps_all64values() {
    let mut arguments = vec![
        "extract".to_string(),
        "--stdin".into(),
        "--css".into(),
        "article".into(),
    ];
    let mut expected = serde_json::Map::new();
    for index in 0..64 {
        let name = format!("field_{index}");
        arguments.extend([
            "--field".into(),
            name.clone(),
            ":scope".into(),
            "dom_text".into(),
        ]);
        expected.insert(name, json!("V"));
    }
    let arguments = arguments.iter().map(String::as_str).collect::<Vec<_>>();
    let result = invoke(&arguments, b"<article>V</article>");
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&result.stdout).unwrap(),
        serde_json::Value::Array(vec![serde_json::Value::Object(expected)])
    );
}

#[test]
fn explicit_source_read_protects_byte_payload() {
    let result = invoke(
        &[
            "extract", "--stdin", "--start", "BEGIN", "--end", "END", "--read", "source", "--raw",
        ],
        "BEGINé\r\nEND".as_bytes(),
    );
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(result.stdout, "é\r\n".as_bytes());
}
