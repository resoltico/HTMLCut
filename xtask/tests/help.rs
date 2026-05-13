use std::process::Command;

use xtask::{assert_known_fuzz_target, fuzz_smoke_targets};

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
    assert!(help.contains("Run a short maintained libFuzzer smoke pass."));
    assert!(help.contains("Refresh the checked-in htmlcut-core semver baseline."));
    assert!(help.contains("cargo xtask check"));
    assert!(help.contains("cargo xtask refresh-semver-baseline --git-ref v7.0.0"));
}

#[test]
fn root_help_examples_parse_against_the_live_cli_surface() {
    let help = run_xtask_help(&["--help"]);
    let examples = xtask_help_examples(&help);

    assert_eq!(examples.len(), 6, "root help example inventory drifted");

    assert_eq!(
        examples
            .iter()
            .copied()
            .filter(|line| !line.starts_with("cargo xtask fuzz-smoke --target "))
            .collect::<Vec<_>>(),
        vec![
            "cargo xtask check",
            "cargo xtask ci-rust-gate",
            "cargo xtask semver-check",
            "cargo xtask coverage",
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

    let ci_rust_gate_help = run_xtask_help(&["ci-rust-gate", "--help"]);
    assert!(ci_rust_gate_help.contains("cross-platform Rust CI gate"));

    let coverage_help = run_xtask_help(&["coverage", "--help"]);
    assert!(coverage_help.contains("Run the curated 100% line-and-branch coverage gate"));

    let fuzz_help = run_xtask_help(&["fuzz-smoke", "--help"]);
    assert!(fuzz_help.contains("Run a short maintained libFuzzer smoke pass"));

    let semver_help = run_xtask_help(&["refresh-semver-baseline", "--help"]);
    assert!(semver_help.contains("Refresh the checked-in htmlcut-core semver baseline"));
}

#[test]
fn invalid_fuzz_target_errors_readably_without_debug_quotes() {
    let output = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["fuzz-smoke", "--target", "not-real"])
        .output()
        .expect("run xtask invalid fuzz target");

    assert!(!output.status.success(), "unknown fuzz target should fail");

    let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
    assert!(stderr.contains("xtask: unknown fuzz target `not-real`"));
    assert!(!stderr.contains("xtask: \"unknown fuzz target"));
}
