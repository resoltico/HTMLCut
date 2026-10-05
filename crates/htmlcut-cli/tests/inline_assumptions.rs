// SPDX-License-Identifier: MPL-2.0
//! Inline exclusions and exact context guards compile to the published plan contract.
mod support;
use serde_json::json;
use support::invoke;

#[test]
fn scalar_exclusion_has_the_same_data_and_receipt_as_a_plan() {
    let root = tempfile::tempdir().unwrap();
    let plan = root.path().join("reading.json");
    let inline_receipt = root.path().join("inline.receipt.json");
    let plan_receipt = root.path().join("plan.receipt.json");
    std::fs::write(
        &plan,
        serde_json::to_vec(&json!({
            "schema":"htmlcut.extraction.plan", "version":htmlcut_core::SCHEMA_VERSION,
            "strategy":{"kind":"css","selector":"p"},
            "projection":{"kind":"markdown"},
            "exclude":["sup.reference"]
        }))
        .unwrap(),
    )
    .unwrap();
    let source = b"<p>China<sup class=reference>[1]</sup></p>";
    let inline = invoke(
        &[
            "extract",
            "--stdin",
            "--css",
            "p",
            "--read",
            "markdown",
            "--exclude",
            "sup.reference",
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
            "--receipt",
            plan_receipt.to_str().unwrap(),
        ],
        source,
    );
    assert!(inline.status.success());
    assert!(structured.status.success());
    assert_eq!(inline.stdout, b"[\"China\"]\n");
    assert_eq!(inline.stdout, structured.stdout);
    assert_eq!(
        std::fs::read(inline_receipt).unwrap(),
        std::fs::read(plan_receipt).unwrap()
    );
}

#[test]
fn exact_header_expectations_reject_column_drift_before_data_delivery() {
    let args = [
        "extract",
        "--stdin",
        "--css",
        "tr:has(td)",
        "--field",
        "location",
        "td:nth-child(1)",
        "normalized_text",
        "--field",
        "population",
        "td:nth-child(2)",
        "normalized_text",
        "--expect-text",
        "th:nth-child(1)",
        "Location",
        "--expect-text",
        "th:nth-child(2)",
        "Population",
    ];
    let valid = invoke(
        &args,
        b"<table><tr><th>Location</th><th>Population</th></tr><tr><td>China</td><td>1</td></tr></table>",
    );
    assert!(valid.status.success());
    assert_eq!(
        valid.stdout,
        b"[{\"location\":\"China\",\"population\":\"1\"}]\n"
    );

    for source in [
        b"<table><tr><th>Population</th><th>Location</th></tr><tr><td>China</td><td>1</td></tr></table>".as_slice(),
        b"<table><tr><th>Location</th><th>Population</th></tr><tr><td>China</td><td>1</td></tr></table><table><tr><th>Other</th><th>Population</th></tr></table>".as_slice(),
    ] {
        let failure = invoke(&args, source);
        assert_eq!(failure.status.code(), Some(3));
        assert!(failure.stdout.is_empty());
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&failure.stderr).unwrap()["code"],
            "guard_failed"
        );
    }
}

#[test]
fn incompatible_inline_assumptions_fail_before_source_publication() {
    for args in [
        vec![
            "extract",
            "--stdin",
            "--css",
            "a",
            "--read",
            "attribute:href",
            "--exclude",
            "sup",
        ],
        vec![
            "extract",
            "--stdin",
            "--css",
            "p",
            "--expect-text",
            "[",
            "SECRET",
        ],
        vec![
            "extract",
            "--stdin",
            "--css",
            "p",
            "--field",
            "text",
            "p",
            "dom_text",
            "--exclude",
            "sup",
        ],
    ] {
        let failure = invoke(&args, b"<p>content</p>");
        assert_eq!(failure.status.code(), Some(2));
        assert!(failure.stdout.is_empty());
        assert!(!String::from_utf8_lossy(&failure.stderr).contains("SECRET"));
    }
}
