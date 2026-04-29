use std::process::Command;

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

#[test]
fn root_help_describes_each_maintained_task() {
    let help = run_xtask_help(&["--help"]);

    assert!(help.contains("Run the full maintainer quality gate."));
    assert!(help.contains("Run only the curated 100% coverage gate."));
    assert!(help.contains("Run a short maintained libFuzzer smoke pass."));
    assert!(help.contains("Refresh the checked-in htmlcut-core semver baseline."));
    assert!(help.contains("cargo xtask check"));
    assert!(help.contains("cargo xtask refresh-semver-baseline --git-ref v6.0.0"));
}

#[test]
fn subcommand_help_explains_scope_instead_of_only_showing_usage() {
    let check_help = run_xtask_help(&["check", "--help"]);
    assert!(check_help.contains("Run the full maintainer quality gate"));
    assert!(check_help.contains("100% coverage pass"));

    let coverage_help = run_xtask_help(&["coverage", "--help"]);
    assert!(coverage_help.contains("Run the curated 100% line-and-branch coverage gate"));

    let fuzz_help = run_xtask_help(&["fuzz-smoke", "--help"]);
    assert!(fuzz_help.contains("Run a short maintained libFuzzer smoke pass"));

    let semver_help = run_xtask_help(&["refresh-semver-baseline", "--help"]);
    assert!(semver_help.contains("Refresh the checked-in htmlcut-core semver baseline"));
}
