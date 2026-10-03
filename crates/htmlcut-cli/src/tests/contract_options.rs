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
        vec!["--css", "p", "--match", "nth"],
        vec!["--css", "p", "--index", "1"],
        vec!["--css", "p", "--min", "0"],
        vec!["--css", "p", "--max", "1"],
        vec!["--css", "p", "--match", "nth", "--min", "0"],
        vec!["--css", "p", "--match", "nth", "--index", "1", "--min", "0"],
        vec!["--css", "p", "--match", "nth", "--index", "1", "--max", "1"],
        vec!["--css", "p", "--match", "nth", "--max", "1"],
        vec!["--css", "p", "--match", "all", "--index", "1"],
        vec!["--css", "p", "--projection", "attribute"],
        vec!["--css", "p", "--projection", "document_text"],
        vec![
            "--css",
            "p",
            "--projection",
            "dom_text",
            "--attribute",
            "href",
        ],
        vec!["--start", "x", "--end", "y", "--projection", "dom_text"],
        vec!["--start", "x", "--end", "y", "--attribute", "href"],
        vec!["--css", "p", "--encoding", "utf-8"],
        vec!["--css", "p", "--audit", "unused"],
        vec!["--css", "p", "--save-run", "unused"],
        vec!["--css", "p", "--bundle", "unused", "--receipt", "unused"],
        vec!["--css", "p", "--url", "https://example.test/"],
        vec!["--css", "p", "--url-env", "UNUSED"],
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
fn scalar_attribute_regex_and_explicit_empty_selection_have_current_shapes() {
    for (args, source, expected) in [
        (
            vec![
                "htmlcut",
                "extract",
                "--stdin",
                "--css",
                "a",
                "--attribute",
                "href",
            ],
            b"<a href='next'></a>".as_slice(),
            serde_json::json!(["next"]),
        ),
        (
            vec![
                "htmlcut", "extract", "--stdin", "--css", "p", "--match", "all", "--min", "0",
                "--max", "0",
            ],
            b"<div>x</div>".as_slice(),
            serde_json::json!([]),
        ),
        (
            vec![
                "htmlcut",
                "extract",
                "--stdin",
                "--start",
                "X",
                "--end",
                "Y",
                "--regex",
                "--regex-flags",
                "i",
            ],
            b"xvaluey".as_slice(),
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
fn receipt_is_fixed_complete_and_off_success_stdout() {
    let root = htmlcut_tempdir::tempdir().unwrap();
    let receipt = root.path().join("receipt.json");
    let result = root.path().join("result.json");
    let (code, out, err) = invoke(
        &[
            "htmlcut",
            "extract",
            "--stdin",
            "--css",
            "p",
            "--receipt",
            receipt.to_str().unwrap(),
            "--output",
            result.to_str().unwrap(),
        ],
        b"<p>180</p>",
    );
    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&err));
    assert!(out.is_empty());
    let evidence: serde_json::Value =
        serde_json::from_slice(&std::fs::read(receipt).unwrap()).unwrap();
    assert_eq!(evidence["schema"], "htmlcut.extraction.receipt");
    assert_eq!(evidence["selected_count"], 1);
    assert!(evidence.get("values").is_none());
    assert!(evidence.get("plan").is_none());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&std::fs::read(result).unwrap()).unwrap(),
        serde_json::json!(["180"])
    );
}

#[test]
fn invalid_record_raw_plan_is_rejected_before_source() {
    let root = htmlcut_tempdir::tempdir().unwrap();
    let path = root.path().join("plan.json");
    std::fs::write(&path,br#"{"schema":"htmlcut.extraction.plan","version":3,"strategy":{"kind":"css","selector":"p"},"projection":{"kind":"records","fields":[{"name":"text","selector":":scope"}]}}"#).unwrap();
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
fn descriptions_named_schemas_and_inner_html_use_the_current_dispatch() {
    for name in ["extract", "run", "inspect", "describe", "schema"] {
        let (code, out, _) = invoke(&["htmlcut", "describe", name], b"");
        assert_eq!(code, 0);
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&out).unwrap()["name"],
            name
        );
    }
    for name in htmlcut_core::SCHEMA_NAMES
        .iter()
        .copied()
        .chain(["htmlcut.bundle"])
    {
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
                "--css",
                "p",
                "--projection",
                "inner_html",
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
    assert_eq!(invoke(&["htmlcut", "run", "missing.bundle"], b"").0, 5);
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
                    "--css",
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
