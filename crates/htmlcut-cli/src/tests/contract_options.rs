use super::*;
use htmlcut_core::{CompiledPlan, ExtractionPlan};

fn file(root: &std::path::Path, name: &str, value: &[u8]) -> std::path::PathBuf {
    let path = root.join(name);
    std::fs::write(&path, value).unwrap();
    path
}

#[test]
fn t05_cli_mode_conflicts_fail_before_stdin_or_acquisition() {
    for args in [
        vec!["htmlcut", "extract", "--stdin"],
        vec![
            "htmlcut", "extract", "--stdin", "--css", "p", "--match", "nth",
        ],
        vec![
            "htmlcut", "extract", "--stdin", "--css", "p", "--index", "1",
        ],
        vec!["htmlcut", "extract", "--stdin", "--css", "p", "--min", "0"],
        vec![
            "htmlcut",
            "extract",
            "--stdin",
            "--css",
            "p",
            "--projection",
            "attribute",
        ],
        vec![
            "htmlcut",
            "extract",
            "--stdin",
            "--css",
            "p",
            "--attribute",
            "href",
        ],
        vec![
            "htmlcut",
            "extract",
            "--stdin",
            "--start",
            "x",
            "--end",
            "y",
            "--projection",
            "dom_text",
        ],
        vec![
            "htmlcut",
            "inspect",
            "--stdin",
            "--cursor",
            "x",
            "--preview-plan",
            "missing",
        ],
    ] {
        let (code, out, error) = invoke(&args, b"<p>A</p>");
        assert_eq!(code, 2, "{args:?}: {}", String::from_utf8_lossy(&error));
        assert!(out.is_empty());
    }
    let (code, out, error) = invoke(
        &[
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
            "--raw",
        ],
        b"xvaluey",
    );
    assert_eq!((code, out, error), (0, b"value".to_vec(), Vec::new()));
    let (code, out, error) = invoke(
        &[
            "htmlcut", "extract", "--stdin", "--css", "p", "--match", "all", "--min", "0", "--max",
            "0",
        ],
        b"<div>A</div>",
    );
    assert_eq!(code, 0);
    assert!(error.is_empty());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&out).unwrap()["values"],
        serde_json::json!([])
    );
}

#[test]
fn t22_field_selected_audit_is_complete_bounded_and_off_stdout() {
    let root = htmlcut_tempdir::tempdir().unwrap();
    let audit = root.path().join("audit.json");
    let result = root.path().join("result.json");
    let (code, out, error) = invoke(
        &[
            "htmlcut",
            "extract",
            "--stdin",
            "--css",
            "p",
            "--audit",
            audit.to_str().unwrap(),
            "--audit-field",
            "plan,source_digest,plan_digest,extraction_digest,counts,ranges,values,values",
            "--output",
            result.to_str().unwrap(),
        ],
        b"<p>180</p>",
    );
    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&error));
    assert!(out.is_empty());
    assert!(error.is_empty());
    let evidence: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&audit).unwrap()).unwrap();
    assert_eq!(evidence["values"], serde_json::json!(["180"]));
    assert_eq!(evidence["counts"]["selected"], 1);
    assert!(evidence["ranges"].is_null());
    assert_eq!(evidence["plan"]["projection"]["kind"], "dom_text");
    let payload = format!("<p>{}</p>", "x".repeat(1_048_576));
    let large = root.path().join("large.json");
    let (code, out, error) = invoke(
        &[
            "htmlcut",
            "extract",
            "--stdin",
            "--css",
            "p",
            "--audit",
            large.to_str().unwrap(),
            "--audit-field",
            "values",
        ],
        payload.as_bytes(),
    );
    assert_eq!(code, 4);
    assert!(out.is_empty());
    assert!(!large.exists());
    assert!(!error.is_empty());
    let bad_audit = root.path().join("absent/audit.json");
    let (code, out, _) = invoke(
        &[
            "htmlcut",
            "extract",
            "--stdin",
            "--css",
            "p",
            "--audit",
            bad_audit.to_str().unwrap(),
            "--audit-field",
            "counts",
        ],
        b"<p>180</p>",
    );
    assert_eq!(code, 5);
    assert!(out.is_empty());
}

#[test]
fn t03_t05_saved_run_identity_encoding_and_source_are_closed() {
    let root = htmlcut_tempdir::tempdir().unwrap();
    let path = root.path().join("run.json");
    let valid = serde_json::json!({"schema":"htmlcut.run","version":1,"source":{"kind":"stdin"},"plan":ExtractionPlan::css("p").unwrap(),"encoding":"utf-8","base_url":"https://example.test/"});
    file(
        root.path(),
        "run.json",
        &serde_json::to_vec(&valid).unwrap(),
    );
    let (code, out, error) = invoke(
        &["htmlcut", "run", path.to_str().unwrap(), "--raw"],
        b"<p>180</p>",
    );
    assert_eq!((code, out, error), (0, b"180".to_vec(), Vec::new()));
    for (key, bad) in [
        ("schema", serde_json::json!("other")),
        ("version", serde_json::json!(2)),
        ("encoding", serde_json::json!("invalid")),
        ("base_url", serde_json::json!("bad")),
    ] {
        let mut value = valid.clone();
        value[key] = bad;
        std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        let (code, out, _) = invoke(&["htmlcut", "run", path.to_str().unwrap()], b"<p>180</p>");
        assert!(code != 0);
        assert!(out.is_empty());
    }
    for source in [
        serde_json::json!({"kind":"file","path":""}),
        serde_json::json!({"kind":"http"}),
        serde_json::json!({"kind":"http","url_env":"1BAD"}),
        serde_json::json!({"kind":"http","url":"http://user:pass@example.test"}),
        serde_json::json!({"kind":"http","url_env":"X","url":"https://example.test"}),
    ] {
        let mut value = valid.clone();
        value["source"] = source;
        std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        let (code, out, _) = invoke(&["htmlcut", "run", path.to_str().unwrap()], b"<p>180</p>");
        assert_eq!(code, 2);
        assert!(out.is_empty());
    }
    let plan = file(
        root.path(),
        "plan.json",
        htmlcut_core::canonical_json(
            CompiledPlan::compile(&ExtractionPlan::css("p").unwrap())
                .unwrap()
                .plan(),
        )
        .unwrap()
        .as_bytes(),
    );
    let (code, out, _) = invoke(
        &[
            "htmlcut",
            "extract",
            "--stdin",
            "--plan",
            plan.to_str().unwrap(),
            "--encoding",
            "utf-8",
        ],
        b"<p>180</p>",
    );
    assert_eq!(code, 0);
    assert!(!out.is_empty());
}

#[test]
fn t23_preview_plan_has_its_own_dispatch_and_missing_files_fail() {
    let root = htmlcut_tempdir::tempdir().unwrap();
    let plan = file(
        root.path(),
        "plan.json",
        htmlcut_core::canonical_json(&ExtractionPlan::css("p").unwrap())
            .unwrap()
            .as_bytes(),
    );
    let (code, out, _) = invoke(
        &[
            "htmlcut",
            "inspect",
            "--stdin",
            "--preview-plan",
            plan.to_str().unwrap(),
        ],
        b"<p>180</p>",
    );
    assert_eq!(code, 0);
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&out).unwrap()["schema"],
        "htmlcut.preview"
    );
    let (code, out, _) = invoke(&["htmlcut", "run", "missing-run.json"], b"");
    assert_eq!(code, 5);
    assert!(out.is_empty());
    let (code, out, _) = invoke(&["htmlcut", "describe", "unsupported"], b"");
    assert_eq!(code, 2);
    assert!(out.is_empty());
    struct Fault;
    impl serde::Serialize for Fault {
        fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
            Err(serde::ser::Error::custom("test failure"))
        }
    }
    assert!(crate::publication::json(&Fault, 128).is_err());
}
