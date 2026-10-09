// SPDX-License-Identifier: MPL-2.0
//! Real CLI authoring uses the same closed contract as structured plans and the core.
mod support;
use serde_json::json;
use support::invoke;

#[test]
fn inline_field_exclusion_matches_a_structured_plan_and_rejects_unknown_names() {
    let directory = tempfile::tempdir().unwrap();
    let plan = directory.path().join("country.plan.json");
    std::fs::write(&plan, serde_json::to_vec(&json!({"version":7,"select":"tr","match":"one","fields":{"country":{"select":"td","exclude":["sup.reference"],"read":"text"}},"following_siblings":0})).unwrap()).unwrap();
    let source = b"<table><tr><td>China<sup class=reference>[1]</sup></td></tr></table>";
    let inline = invoke(
        &[
            "extract",
            "--stdin",
            "--select",
            "tr",
            "--field",
            "country",
            "td",
            "text",
            "--field-exclude",
            "country",
            "sup.reference",
        ],
        source,
    );
    let structured = invoke(
        &["extract", "--stdin", "--plan", plan.to_str().unwrap()],
        source,
    );
    assert!(inline.status.success());
    assert!(structured.status.success());
    assert_eq!(inline.stdout, b"[{\"country\":\"China\"}]\n");
    assert_eq!(inline.stdout, structured.stdout);

    for args in [
        vec![
            "extract",
            "--stdin",
            "--select",
            "tr",
            "--field",
            "country",
            "td",
            "text",
            "--field-exclude",
            "UNDECLARED_SECRET",
            "sup",
        ],
        vec![
            "extract",
            "--stdin",
            "--select",
            "tr",
            "--field",
            "country",
            "td",
            "attr:title",
            "--field-exclude",
            "country",
            "sup",
        ],
        vec![
            "extract",
            "--stdin",
            "--plan",
            plan.to_str().unwrap(),
            "--field-exclude",
            "country",
            "sup",
        ],
    ] {
        let failure = invoke(&args, source);
        assert_eq!(failure.status.code(), Some(2));
        assert!(failure.stdout.is_empty());
        assert!(!String::from_utf8_lossy(&failure.stderr).contains("UNDECLARED_SECRET"));
    }
}

#[test]
fn optional_inline_field_preserves_absence_and_rejects_ambiguous_or_missing_attributes() {
    let args = [
        "extract",
        "--stdin",
        "--select",
        "article",
        "--all",
        "--optional-field",
        "score",
        ".score",
        "attr:data-score",
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
fn structural_text_separates_blocks_and_breaks_without_altering_literal_text() {
    let source = b"<article><dl><dt>Name</dt><dd>Alice</dd><dt>Amount</dt><dd>180</dd></dl><p>a<br>c</p></article>";
    let normalized = invoke(
        &[
            "extract", "--stdin", "--select", "article", "--read", "text",
        ],
        source,
    );
    assert_eq!(normalized.stdout, b"[\"Name Alice Amount 180 a c\"]\n");
    let literal = invoke(
        &[
            "extract", "--stdin", "--select", "article", "--read", "literal",
        ],
        source,
    );
    assert_eq!(literal.stdout, b"[\"NameAliceAmount180ac\"]\n");
    let foreign = invoke(
        &[
            "extract", "--stdin", "--select", "article", "--read", "text",
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
            "--select",
            "article",
            "--field",
            "title",
            "[id='a@b']",
            "attr:title",
            "--field",
            "price",
            ".price",
            "text",
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
            "--select",
            "a",
            "--read",
            "url:href",
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
            "--select",
            "tr.entry",
            "--field",
            "title",
            ":scope td",
            "literal",
            "--field",
            "score",
            ":scope + tr .score",
            "text",
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
            "--select",
            "p",
            "--projection",
            "markdown",
        ],
        vec!["extract", "--stdin", "--select", "p", "--attribute", "id"],
        vec![
            "extract",
            "--stdin",
            "--select",
            "p",
            "--read",
            "UNDECLARED_SECRET",
        ],
        vec![
            "extract", "--stdin", "--select", "p", "--field", "a", "p", "source",
        ],
        vec![
            "extract", "--stdin", "--select", "p", "--field", "a", "p", "literal", "--field", "a",
            "p", "literal",
        ],
        vec![
            "extract",
            "--stdin",
            "--select",
            "p",
            "--following-siblings",
            "1",
        ],
        vec!["extract", "--stdin", "--select", "p", "--field", "a", "p"],
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
        &[
            "extract",
            "--stdin",
            "--select",
            "p",
            "--read",
            "undeclared",
        ],
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
            "--select",
            "p",
            "--read",
            "resolved-markdown",
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
        "--select".into(),
        "article".into(),
    ];
    for index in 0..65 {
        arguments.extend([
            "--field".into(),
            format!("field_{index}"),
            ":scope".into(),
            "literal".into(),
        ]);
    }
    let arguments = arguments.iter().map(String::as_str).collect::<Vec<_>>();
    let result = invoke(&arguments, b"<article>x</article>");
    assert_eq!(result.status.code(), Some(2));
    assert!(result.stdout.is_empty());
}

#[test]
fn structured_and_inline_authoring_bind_identical_data() {
    let directory = tempfile::tempdir().unwrap();
    let plan = directory.path().join("fields.json");
    std::fs::write(&plan, serde_json::to_vec(&json!({"version":7,"select":"article","match":"one","fields":{"title":{"select":"a","read":"attr:title"},"price":{"select":":scope + aside .price","read":"text"},"url":{"select":"a","read":"url:href"}},"following_siblings":1})).unwrap()).unwrap();
    let source = "<main><article><a title=T href=next>x</a></article><aside><span class=price>\u{a0}10\u{2003}</span></aside></main>".as_bytes();
    let inline = invoke(
        &[
            "extract",
            "--stdin",
            "--select",
            "article",
            "--following-siblings",
            "1",
            "--field",
            "title",
            "a",
            "attr:title",
            "--field",
            "price",
            ":scope + aside .price",
            "text",
            "--field",
            "url",
            "a",
            "url:href",
            "--base-url",
            "https://example.test/",
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
}

#[test]
fn valid_field_limit_boundary_keeps_all64values() {
    let mut arguments = vec![
        "extract".to_string(),
        "--stdin".into(),
        "--select".into(),
        "article".into(),
    ];
    let mut expected = serde_json::Map::new();
    for index in 0..64 {
        let name = format!("field_{index}");
        arguments.extend([
            "--field".into(),
            name.clone(),
            ":scope".into(),
            "literal".into(),
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
fn retired_source_read_has_no_success_payload() {
    let result = invoke(
        &["extract", "--stdin", "--select", "p", "--read", "source"],
        b"<p>value</p>",
    );
    assert_eq!(result.status.code(), Some(2));
    assert!(result.stdout.is_empty());
}
