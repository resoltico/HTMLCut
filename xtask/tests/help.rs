use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;
use xtask::{assert_known_fuzz_target, fuzz_smoke_targets};

mod support;

use support::IsolatedArtifacts;

fn run_xtask_help(args: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(args)
        .output()
        .expect("run xtask help");

    assert!(
        output.status.success(),
        "xtask {:?} failed:\nstdout:\n{}\nstderr:\n{}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout).expect("utf8 help output")
}

fn xtask_help_examples(help: &str) -> Vec<&str> {
    help.lines()
        .map(str::trim)
        .filter(|line| line.starts_with("cargo xtask "))
        .collect()
}

fn run_xtask_example_help(example: &str) -> String {
    let args = example
        .split_ascii_whitespace()
        .skip(2)
        .chain(["--help"])
        .collect::<Vec<_>>();
    run_xtask_help(&args)
}

#[test]
fn root_help_describes_each_maintained_task() {
    let help = run_xtask_help(&["--help"]);

    assert!(help.contains("Run the full maintainer quality gate."));
    assert!(help.contains("Run the curated cross-platform Rust CI gate."));
    assert!(help.contains("Run only the curated 100% coverage gate."));
    assert!(help.contains("Run the maintained strict-provenance selector-and-slice Miri proof."));
    assert!(help.contains("Run the maintained dependency-freshness gate."));
    assert!(help.contains("Run a short maintained libFuzzer smoke pass."));
    assert!(help.contains("Inspect or repair the repository artifact hygiene policy."));
    assert!(help.contains("Inspect or enforce the first-party Rust source-structure contract."));
    assert!(help.contains("Refresh the checked-in htmlcut-core semver baseline."));
    assert!(help.contains("cargo xtask check"));
    assert!(help.contains("cargo xtask refresh-semver-baseline --git-ref v7.0.0"));
}

#[test]
fn root_help_examples_parse_against_the_live_cli_surface() {
    let help = run_xtask_help(&["--help"]);
    let examples = xtask_help_examples(&help);

    assert_eq!(examples.len(), 13, "root help example inventory drifted");

    assert_eq!(
        examples
            .iter()
            .copied()
            .filter(|line| !line.starts_with("cargo xtask fuzz-smoke --target "))
            .collect::<Vec<_>>(),
        vec![
            "cargo xtask check",
            "cargo xtask check --format json",
            "cargo xtask ci-rust-gate",
            "cargo xtask semver-check",
            "cargo xtask coverage",
            "cargo xtask miri",
            "cargo xtask outdated-check",
            "cargo xtask hygiene report",
            "cargo xtask hygiene clean --mode rebuildable",
            "cargo xtask structure report",
            "cargo xtask structure check",
            "cargo xtask refresh-semver-baseline --git-ref v7.0.0",
        ]
    );

    for example in &examples {
        let example_help = run_xtask_example_help(example);
        assert!(
            !example_help.is_empty(),
            "example should parse and emit help: {example}"
        );
    }

    let fuzz_example = examples
        .iter()
        .find(|line| line.starts_with("cargo xtask fuzz-smoke --target "))
        .expect("fuzz-smoke example");
    let target = fuzz_example
        .split_ascii_whitespace()
        .last()
        .expect("fuzz-smoke target");

    assert_known_fuzz_target(target).expect("canonical fuzz target");
    assert!(fuzz_smoke_targets().contains(&target));
}

#[test]
fn subcommand_help_explains_scope_instead_of_only_showing_usage() {
    let check_help = run_xtask_help(&["check", "--help"]);
    assert!(check_help.contains("Run the full maintainer quality gate"));
    assert!(check_help.contains("100% coverage pass"));
    assert!(check_help.contains("--format <FORMAT>"));
    assert!(check_help.contains("--verbose"));

    let ci_rust_gate_help = run_xtask_help(&["ci-rust-gate", "--help"]);
    assert!(ci_rust_gate_help.contains("cross-platform Rust CI gate"));

    let coverage_help = run_xtask_help(&["coverage", "--help"]);
    assert!(coverage_help.contains("Run the curated 100% line-and-branch coverage gate"));

    let miri_help = run_xtask_help(&["miri", "--help"]);
    assert!(
        miri_help.contains("Run the maintained strict-provenance selector-and-slice Miri proof")
    );

    let outdated_help = run_xtask_help(&["outdated-check", "--help"]);
    assert!(outdated_help.contains("Run the maintained dependency-freshness gate"));

    let hygiene_help = run_xtask_help(&["hygiene", "--help"]);
    assert!(hygiene_help.contains("artifact hygiene policy"));

    let structure_help = run_xtask_help(&["structure", "--help"]);
    assert!(
        structure_help
            .contains("role ownership, cohesion budgets, and internal dependency boundaries")
    );

    let fuzz_help = run_xtask_help(&["fuzz-smoke", "--help"]);
    assert!(fuzz_help.contains("Run a short maintained libFuzzer smoke pass"));

    let semver_help = run_xtask_help(&["refresh-semver-baseline", "--help"]);
    assert!(semver_help.contains("Refresh the checked-in htmlcut-core semver baseline"));
}

#[test]
fn invalid_fuzz_target_errors_readably_without_debug_quotes() {
    let artifacts = IsolatedArtifacts::new();
    let output = artifacts
        .xtask_command()
        .args(["fuzz-smoke", "--target", "not-real"])
        .output()
        .expect("run xtask invalid fuzz target");

    assert!(!output.status.success(), "unknown fuzz target should fail");

    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(stderr.contains("xtask: unknown fuzz target `not-real`"));
    assert!(!stderr.contains("xtask: \"unknown fuzz target"));
    assert!(
        artifacts.gate_report_dir().is_dir(),
        "rejected gate evidence must remain in the test-owned artifact root"
    );
}

#[test]
fn hygiene_report_honors_explicit_cargo_artifact_env_overrides() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    let override_root = std::env::temp_dir().join(format!("htmlcut-xtask-artifacts-{nonce}"));
    fs::create_dir_all(&override_root).expect("create artifact override tempdir");
    let target_dir = override_root.join("managed-target");
    let build_dir = override_root.join("managed-build");

    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["hygiene", "report", "--format", "json"])
        .env("CARGO_TARGET_DIR", &target_dir)
        .env("CARGO_BUILD_BUILD_DIR", &build_dir)
        .output()
        .expect("run xtask hygiene report");

    assert!(
        output.status.success(),
        "xtask hygiene report failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let report = serde_json::from_slice::<Value>(&output.stdout).expect("json hygiene report");
    let entries = report["entries"].as_array().expect("report entries");

    let managed_target = entries
        .iter()
        .find(|entry| entry["id"] == "managed-workspace-target")
        .expect("managed target entry");
    let managed_build = entries
        .iter()
        .find(|entry| entry["id"] == "managed-workspace-build")
        .expect("managed build entry");
    let managed_gate_reports = entries
        .iter()
        .find(|entry| entry["id"] == "managed-gate-reports")
        .expect("managed gate reports entry");

    assert_eq!(
        managed_target["path"].as_str(),
        Some(target_dir.to_string_lossy().as_ref())
    );
    assert_eq!(
        managed_build["path"].as_str(),
        Some(build_dir.to_string_lossy().as_ref())
    );
    assert_eq!(
        managed_gate_reports["path"].as_str(),
        Some(override_root.join("gate-runs").to_string_lossy().as_ref())
    );

    fs::remove_dir_all(&override_root).expect("remove artifact override tempdir");
}
