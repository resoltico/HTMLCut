// SPDX-License-Identifier: MPL-2.0
use super::*;
use std::io::{self, Read};

struct Unread;
impl Read for Unread {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        panic!("configuration failure consumed source")
    }
}

#[test]
fn invalid_options_and_retired_vocabulary_fail_before_consuming_source() {
    for extras in [
        vec![],
        vec!["--select", "p", "--match", "nth"],
        vec!["--select", "p", "--index", "1"],
        vec!["--select", "p", "--min", "0"],
        vec!["--select", "p", "--max", "1"],
        vec!["--select", "p", "--match", "nth", "--min", "0"],
        vec!["--select", "p", "--nth", "1", "--min", "0"],
        vec!["--select", "p", "--nth", "1", "--max", "1"],
        vec!["--select", "p", "--match", "nth", "--max", "1"],
        vec!["--select", "p", "--all", "--index", "1"],
        vec!["--select", "p", "--projection", "attribute"],
        vec!["--select", "p", "--projection", "document_text"],
        vec![
            "--select",
            "p",
            "--projection",
            "literal",
            "--attribute",
            "href",
        ],
        vec!["--start", "x", "--end", "y", "--projection", "literal"],
        vec!["--start", "x", "--end", "y", "--attribute", "href"],
        vec!["--select", "p", "--encoding", "utf-8"],
        vec!["--select", "p", "--audit", "unused"],
        vec!["--select", "p", "--save-run", "unused"],
        vec!["--select", "p", "--bundle", "unused", "--receipt", "unused"],
        vec!["--select", "p", "--url", "https://example.test/"],
        vec!["--select", "p", "--url-env", "UNUSED"],
    ] {
        let mut args = vec!["htmlcut", "extract", "--stdin"];
        args.extend(extras);
        let mut out = Vec::new();
        let mut err = Vec::new();
        assert_eq!(app::run(args, &mut Unread, &mut out, &mut err), 2);
        assert!(out.is_empty());
        assert!(!err.is_empty());
    }
}

#[test]
fn scalar_attributes_regex_expectations_and_explicit_empty_selection_have_current_shapes() {
    for (args, source, expected) in [
        (
            vec![
                "htmlcut",
                "extract",
                "--stdin",
                "--select",
                "a",
                "--read",
                "attr:href",
            ],
            b"<a href='next'></a>".as_slice(),
            serde_json::json!(["next"]),
        ),
        (
            vec![
                "htmlcut", "extract", "--stdin", "--select", "p", "--all", "--min", "0", "--max",
                "0",
            ],
            b"<div>x</div>".as_slice(),
            serde_json::json!([]),
        ),
        (
            vec![
                "htmlcut",
                "extract",
                "--stdin",
                "--plan-json",
                r#"{"version":7,"select":"p","expect":[{"select":"p","pattern":"(?i)^VALUE$"}]}"#,
            ],
            b"<p>value</p>".as_slice(),
            serde_json::json!(["value"]),
        ),
    ] {
        let (code, out, err) = invoke(&args, source);
        assert_eq!(code, 0, "{}", String::from_utf8_lossy(&err));
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&out).unwrap(),
            expected
        );
    }
}

#[test]
fn invalid_record_raw_plan_is_rejected_before_source() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("plan.json");
    std::fs::write(&path,br#"{"version":7,"select":"p","match":"one","fields":{"text":{"select":":scope","read":"literal"}}}"#).unwrap();
    let mut out = Vec::new();
    let mut err = Vec::new();
    assert_eq!(
        app::run(
            [
                "htmlcut",
                "extract",
                "--stdin",
                "--plan",
                path.to_str().unwrap(),
                "--raw"
            ],
            &mut Unread,
            &mut out,
            &mut err
        ),
        2
    );
    assert!(out.is_empty());
}

#[test]
fn native_help_named_schemas_and_inner_html_use_current_dispatch() {
    for name in ["extract", "inspect", "schema"] {
        let (code, out, _) = invoke(&["htmlcut", name, "--help"], b"");
        assert_eq!(code, 0);
        assert!(String::from_utf8_lossy(&out).contains("Usage:"));
    }
    for name in htmlcut_core::SCHEMA_NAMES.iter().copied() {
        let (code, out, err) = invoke(&["htmlcut", "schema", name], b"");
        assert_eq!(code, 0, "{}", String::from_utf8_lossy(&err));
        assert!(!out.is_empty());
    }
    assert_eq!(
        invoke(
            &[
                "htmlcut",
                "extract",
                "--stdin",
                "--select",
                "p",
                "--read",
                "inner-html",
                "--raw"
            ],
            b"<p>A<b>B</b></p>"
        ),
        (0, b"A<b>B</b>".to_vec(), vec![])
    );
    for name in [
        "htmlcut.run",
        "htmlcut.preview",
        "htmlcut.selector.proposal",
        "htmlcut.extraction.result",
    ] {
        assert_eq!(invoke(&["htmlcut", "schema", name], b"").0, 2);
    }
    assert_eq!(invoke(&["htmlcut", "describe", "unsupported"], b"").0, 2);
    assert_eq!(invoke(&["htmlcut", "replay", "missing.bundle"], b"").0, 2);
}

#[test]
fn invalid_base_metadata_is_rejected_before_any_source_consumption() {
    for command in ["extract", "inspect"] {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        assert_eq!(
            app::run(
                [
                    "htmlcut",
                    command,
                    "--stdin",
                    "--select",
                    "p",
                    "--base-url",
                    "relative"
                ],
                &mut Unread,
                &mut stdout,
                &mut stderr
            ),
            2
        );
        assert!(stdout.is_empty());
        let error: serde_json::Value = serde_json::from_slice(&stderr).unwrap();
        assert_eq!(error["code"], "invalid_base_url");
    }
}
