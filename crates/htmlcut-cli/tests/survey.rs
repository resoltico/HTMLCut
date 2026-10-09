// SPDX-License-Identifier: MPL-2.0
//! Black-box survey discovery keeps candidate evidence bounded and failures atomic.
mod support;
use support::invoke;

#[test]
fn small_groups_use_targeted_inspection_while_survey_requires_three_members() {
    let help = invoke(&["inspect", "--help"], b"");
    assert!(help.status.success());
    let help = String::from_utf8(help.stdout).unwrap();
    for fact in [
        "at least three",
        "direct HTML siblings",
        "--select",
        "--samples",
        "outer-html",
    ] {
        assert!(help.contains(fact), "{fact} missing from inspect help");
    }
    let pair = b"<section><article class=record>First</article><article class=record>Second</article></section>";
    let survey = invoke(&["inspect", "--stdin"], pair);
    assert!(survey.status.success());
    let survey: serde_json::Value = serde_json::from_slice(&survey.stdout).unwrap();
    assert_eq!(survey["group_count"], 0);
    assert_eq!(survey["groups_complete"], true);
    assert_eq!(survey["groups"], serde_json::json!([]));
    let targeted = invoke(
        &[
            "inspect",
            "--stdin",
            "--select",
            "article.record",
            "--samples",
            "2",
        ],
        pair,
    );
    assert!(targeted.status.success());
    let targeted: serde_json::Value = serde_json::from_slice(&targeted.stdout).unwrap();
    assert_eq!(targeted["count"], 2);
    assert_eq!(targeted["samples"].as_array().unwrap().len(), 2);
    let triple = b"<section><article class=record>First</article><article class=record>Second</article><article class=record>Third</article></section>";
    let survey = invoke(&["inspect", "--stdin"], triple);
    assert!(survey.status.success());
    let survey: serde_json::Value = serde_json::from_slice(&survey.stdout).unwrap();
    assert_eq!(survey["group_count"], 1);
    assert_eq!(survey["groups"][0]["count"], 3);
    let selector = survey["groups"][0]["selector"].as_str().unwrap();
    let members = invoke(
        &["extract", "--stdin", "--select", selector, "--all"],
        triple,
    );
    assert!(members.status.success());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&members.stdout).unwrap(),
        serde_json::json!(["First", "Second", "Third"])
    );
}

#[test]
fn survey_surfaces_a_table_group_with_header_and_row_shape_evidence() {
    let source = b"<table id=population><tr><th>Location</th><th>Population</th></tr><tr><td>India</td><td>1</td></tr><tr><td>China</td><td>2</td></tr></table>";
    let result = invoke(&["inspect", "--stdin"], source);
    assert!(result.status.success());
    let value: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(value["group_count"], 1);
    assert_eq!(value["groups_complete"], true);
    assert_eq!(value["groups"][0]["selector"], "#population tr");
    assert_eq!(value["groups"][0]["count"], 3);
    assert_eq!(
        value["groups"][0]["table"]["headers"],
        serde_json::json!(["Location", "Population"])
    );
    assert_eq!(value["groups"][0]["table"]["data_rows"], 2);
}

#[test]
fn survey_scope_recovers_content_after_larger_navigation_groups() {
    let navigation = (0..12)
        .map(|i| {
            format!(
                "<ul class=nav{i}>{}</ul>",
                "<li>Navigation</li>".repeat(20 + i)
            )
        })
        .collect::<String>();
    let source = format!(
        "<body>{navigation}<section id=content>{}</section></body>",
        "<article>Target</article>".repeat(10)
    );
    let broad = invoke(&["inspect", "--stdin"], source.as_bytes());
    assert!(broad.status.success());
    let broad: serde_json::Value = serde_json::from_slice(&broad.stdout).unwrap();
    assert_eq!(broad["groups_complete"], false);
    assert!(
        broad["groups"]
            .as_array()
            .unwrap()
            .iter()
            .all(|group| group["item_tag"] == "li")
    );

    let focused = invoke(
        &["inspect", "--stdin", "--within", "#content"],
        source.as_bytes(),
    );
    assert!(focused.status.success());
    let focused: serde_json::Value = serde_json::from_slice(&focused.stdout).unwrap();
    assert_eq!(focused["group_count"], 1);
    assert_eq!(focused["groups"][0]["selector"], "#content > article");
    assert_eq!(focused["groups"][0]["count"], 10);
}

#[test]
fn malformed_scope_limit_and_missing_input_publish_no_survey() {
    for args in [
        vec!["inspect", "--stdin", "--limit", "0"],
        vec!["inspect", "--stdin", "--within", "["],
        vec!["inspect", "--stdin", "--within", ".scope"],
    ] {
        let source = if args.last() == Some(&".scope") {
            b"<section class=scope></section><section class=scope></section>".as_slice()
        } else {
            b"<p>x</p>".as_slice()
        };
        let result = invoke(&args, source);
        assert_eq!(
            result.status.code(),
            Some(if args.last() == Some(&".scope") { 3 } else { 2 })
        );
        assert!(result.stdout.is_empty());
    }
    let root = tempfile::tempdir().unwrap();
    let missing = root.path().join("missing.html");
    let result = invoke(&["inspect", "--file", missing.to_str().unwrap()], b"");
    assert_eq!(result.status.code(), Some(5));
    assert!(result.stdout.is_empty());
}

#[test]
fn survey_encoding_exhaustion_emits_no_partial_answer() {
    let classes = (0..8)
        .map(|i| format!("c{i}{}", "x".repeat(62)))
        .collect::<Vec<_>>()
        .join(" ");
    let source = (0..16)
        .map(|i| {
            let id = format!("p{i:02}{}", "x".repeat(125));
            let articles = format!(
                "<article class='{classes}'>{}</article>",
                "\u{1}".repeat(64)
            )
            .repeat(3);
            format!("<section id='{id}'>{articles}</section>")
        })
        .collect::<String>();
    assert!(
        invoke(&["inspect", "--stdin", "--limit", "1"], source.as_bytes())
            .status
            .success()
    );
    let result = invoke(&["inspect", "--stdin", "--limit", "16"], source.as_bytes());
    assert_eq!(result.status.code(), Some(4));
    assert!(result.stdout.is_empty());
    let error: serde_json::Value = serde_json::from_slice(&result.stderr).unwrap();
    assert_eq!(error["resource_counter"], "encoded_bytes");
}
