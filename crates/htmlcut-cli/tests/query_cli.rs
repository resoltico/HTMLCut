// SPDX-License-Identifier: MPL-2.0
//! Real-process query contract query and delivery checks.
use std::{
    io::Write,
    process::{Command, Stdio},
};
fn run(args: &[&str], source: &str) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_htmlcut"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(source.as_bytes());
    }
    child.wait_with_output().unwrap()
}
#[test]
fn inline_and_json_requests_share_canonical_data() {
    let source = "<article><i>T</i><b>A</b><a href=''></a></article>";
    let inline = run(
        &[
            "extract",
            "--stdin",
            "--select",
            "article",
            "--all",
            "--field",
            "title",
            "i",
            "text",
            "--optional-field",
            "href",
            "a",
            "attr:href",
            "--many-field",
            "tags",
            "b",
            "text",
        ],
        source,
    );
    assert!(
        inline.status.success(),
        "{}",
        String::from_utf8_lossy(&inline.stderr)
    );
    let query = r#"{"version":7,"select":"article","match":"all","fields":{"title":{"select":"i"},"href":{"select":"a","match":"optional","read":"attr:href"},"tags":{"select":"b","match":"all"}}}"#;
    let json = run(&["extract", "--stdin", "--plan-json", query], source);
    assert!(json.status.success());
    assert_eq!(inline.stdout, json.stdout);
    assert_eq!(
        inline.stdout,
        b"[{\"href\":\"\",\"tags\":[\"A\"],\"title\":\"T\"}]\n"
    );
    assert!(inline.stderr.is_empty());
}
#[test]
fn raw_empty_and_script_readings_have_exact_transport_framing() {
    let empty = run(&["extract", "--stdin", "--select", "p", "--raw"], "<p></p>");
    assert!(empty.status.success());
    assert!(empty.stdout.is_empty());
    let literal = run(
        &[
            "extract", "--stdin", "--select", "script", "--read", "literal", "--raw",
        ],
        "<script>{\"x\":1}</script>",
    );
    assert!(literal.status.success());
    assert_eq!(literal.stdout, br#"{"x":1}"#);
    let text = run(
        &["extract", "--stdin", "--select", "script"],
        "<script>secret</script>",
    );
    assert!(text.status.success());
    assert_eq!(text.stdout, b"[\"\"]\n");
}
#[test]
fn retired_vocabulary_and_inapplicable_flags_fail_without_data() {
    for args in [
        vec!["outline", "--stdin"],
        vec!["run", "missing"],
        vec!["describe"],
        vec!["extract", "--stdin", "--css", "p"],
        vec!["extract", "--stdin", "--select", "p", "--match", "all"],
        vec!["extract", "--stdin", "--start", "A", "--end", "B"],
        vec!["extract", "--stdin", "--select", "p", "--read", "dom_text"],
        vec!["extract", "--stdin", "--select", "p", "--min", "0"],
        vec!["inspect", "--stdin", "--select", "p", "--limit", "1"],
        vec!["inspect", "--stdin", "--samples", "1"],
        vec!["inspect", "--stdin", "--identifiers"],
        vec!["extract", "--stdin", "--plan", "-"],
    ] {
        let output = run(&args, "<p>never deliver</p>");
        assert_eq!(
            output.status.code(),
            Some(2),
            "{args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stdout.is_empty());
    }
}
#[test]
fn merged_inspection_identifies_source_and_complete_attribute_names() {
    let source = "<ul id='list'><li class='row' data-x='1'>A<script>secret</script></li><li class='row'>B</li><li class='row'>C</li></ul>";
    let targeted = run(
        &["inspect", "--stdin", "--select", "li", "--samples", "1"],
        source,
    );
    assert!(
        targeted.status.success(),
        "{}",
        String::from_utf8_lossy(&targeted.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&targeted.stdout).unwrap();
    assert_eq!(value["count"], 3);
    assert_eq!(value["samples_complete"], false);
    assert_eq!(value["samples"][0]["classes"], serde_json::json!(["row"]));
    assert_eq!(
        value["samples"][0]["attributes"],
        serde_json::json!(["class", "data-x"])
    );
    assert_eq!(value["samples"][0]["text"], "A");
    let survey = run(&["inspect", "--stdin"], source);
    assert!(survey.status.success());
    let groups: serde_json::Value = serde_json::from_slice(&survey.stdout).unwrap();
    assert_eq!(groups["group_count"], 1);
}

#[test]
fn impossible_raw_shapes_are_rejected_before_acquisition() {
    for bounds in [vec!["--min", "2"], vec!["--min", "0", "--max", "0"]] {
        let mut args = vec![
            "extract",
            "--file",
            "/missing-synthetic-htmlcut-input",
            "--select",
            "p",
            "--all",
            "--raw",
        ];
        args.extend(bounds);
        let output = run(&args, "");
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert!(!String::from_utf8_lossy(&output.stderr).contains("/missing-synthetic"));
    }
}

#[test]
fn query_stdin_is_intentional_and_field_parse_failures_are_lexical() {
    let directory = htmlcut_tempdir::tempdir().unwrap();
    let source = directory.path().join("source.html");
    std::fs::write(&source, "<p>A</p>").unwrap();
    let output = run(
        &["extract", "--file", source.to_str().unwrap(), "--plan", "-"],
        r#"{"version":7,"select":"p"}"#,
    );
    assert!(output.status.success());
    assert_eq!(output.stdout, b"[\"A\"]\n");
    let failed = run(
        &["extract", "--file", source.to_str().unwrap(), "--plan", "-"],
        "not JSON",
    );
    assert_eq!(failed.status.code(), Some(2));
    assert!(failed.stdout.is_empty());
    for flags in [
        vec![
            "--field",
            "z",
            "p",
            "unknown",
            "--optional-field",
            "a",
            "p",
            "unknown",
        ],
        vec![
            "--optional-field",
            "a",
            "p",
            "unknown",
            "--field",
            "z",
            "p",
            "unknown",
        ],
    ] {
        let mut args = vec!["extract", "--stdin", "--select", "p"];
        args.extend(flags);
        let output = run(&args, "<p>A</p>");
        assert_eq!(output.status.code(), Some(2));
        let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(error["field_name"], "a");
    }
}

#[test]
fn raw_all_bounds_containing_one_allow_real_cardinality_to_decide() {
    for bounds in [
        vec![],
        vec!["--min", "0", "--max", "1"],
        vec!["--min", "1", "--max", "1"],
    ] {
        let mut args = vec!["extract", "--stdin", "--select", "p", "--all", "--raw"];
        args.extend(bounds);
        let output = run(&args, "<p>A</p>");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, b"A");
    }
}

#[test]
fn query_stdin_byte_limit_rejects_before_source_execution() {
    let directory = htmlcut_tempdir::tempdir().unwrap();
    let source = directory.path().join("source.html");
    std::fs::write(&source, "<p>A</p>").unwrap();
    let output = run(
        &["extract", "--file", source.to_str().unwrap(), "--plan", "-"],
        &" ".repeat(256 * 1024 + 1),
    );
    assert_eq!(output.status.code(), Some(4));
    assert!(output.stdout.is_empty());
    let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["resource_counter"], "input_bytes");
    assert_eq!(error["configured_bound"], 256 * 1024);
}

#[test]
fn json_escaping_cannot_publish_data_beyond_the_encoded_byte_bound() {
    let source = format!("<article data-value='{}'></article>", "\u{1}".repeat(20)).repeat(10_000);
    let fields = (0..64)
        .map(|i| {
            (
                format!("f{i:02}{}", "x".repeat(61)),
                serde_json::json!({"select":":scope","read":"attr:data-value"}),
            )
        })
        .collect::<serde_json::Map<_, _>>();
    let plan = serde_json::json!({"version":7,"select":"article","match":"all","fields":fields,"limits":{"max_cells":1_000_000,"max_work":10_000_000}}).to_string();
    // 121,620,001 encoded bytes exceed 64 MiB despite only 12.8 MB of leaf values.
    let result = run(&["extract", "--stdin", "--plan-json", &plan], &source);
    assert_eq!(result.status.code(), Some(4));
    assert!(result.stdout.is_empty());
    let error: serde_json::Value = serde_json::from_slice(&result.stderr).unwrap();
    assert_eq!(error["resource_counter"], "encoded_bytes");
    assert_eq!(error["configured_bound"], 64 * 1024 * 1024);
}
