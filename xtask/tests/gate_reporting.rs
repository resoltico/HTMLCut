#![forbid(unsafe_code)]

use serde_json::Value;

mod support;

use support::IsolatedArtifacts;

#[test]
fn structure_gate_json_is_one_machine_readable_report() {
    let artifacts = IsolatedArtifacts::new();
    let output = artifacts
        .xtask_command()
        .args(["structure", "check", "--format", "json"])
        .output()
        .expect("run structured source gate");

    assert!(
        output.status.success(),
        "structure gate failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "JSON mode must not write progress to stderr"
    );

    let report = serde_json::from_slice::<Value>(&output.stdout).expect("parse JSON report");
    assert_eq!(report["schema"], "htmlcut.gate_run@1");
    assert_eq!(report["gate"], "structure-check");
    assert_eq!(report["outcome"], "passed");
    assert!(
        report["steps"]
            .as_array()
            .is_some_and(|steps| !steps.is_empty())
    );
    let report_path = report["report_path"]
        .as_str()
        .map(std::path::Path::new)
        .expect("JSON report path");
    assert!(
        report_path.is_file(),
        "JSON report must point to retained evidence"
    );
    assert!(
        report_path.starts_with(artifacts.gate_report_dir()),
        "integration-test gate evidence must remain isolated: {}",
        report_path.display()
    );
}

#[test]
fn rejected_gate_input_still_emits_a_machine_readable_failure_report() {
    let artifacts = IsolatedArtifacts::new();
    let output = artifacts
        .xtask_command()
        .args(["fuzz-smoke", "--target", "not-real", "--format", "json"])
        .output()
        .expect("run rejected fuzz smoke");

    assert!(!output.status.success(), "invalid target must fail");
    let report = serde_json::from_slice::<Value>(&output.stdout).expect("parse JSON report");
    assert_eq!(report["schema"], "htmlcut.gate_run@1");
    assert_eq!(report["gate"], "fuzz-smoke");
    assert_eq!(report["outcome"], "failed");
    assert!(
        report["failure"]["message"]
            .as_str()
            .is_some_and(|message| message.contains("unknown fuzz target `not-real`"))
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("unknown fuzz target `not-real`"));
    let report_path = report["report_path"]
        .as_str()
        .map(std::path::Path::new)
        .expect("JSON report path");
    assert!(
        report_path.starts_with(artifacts.gate_report_dir()),
        "integration-test gate evidence must remain isolated: {}",
        report_path.display()
    );
}
