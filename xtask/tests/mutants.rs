#![forbid(unsafe_code)]

use std::fs;

fn repo_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf()
}

#[test]
fn mutation_configuration_scopes_only_first_party_runtime_source() {
    let config = fs::read_to_string(repo_root().join(".cargo").join("mutants.toml"))
        .expect("read mutation configuration");

    assert!(config.contains("all_features = true"));
    assert!(config.contains("additional_cargo_args = [\"--locked\"]"));
    assert!(config.contains("test_tool = \"cargo\""));
    assert!(config.contains("sharding = \"round-robin\""));
    assert!(config.contains("htmlcut-core\", \"htmlcut-cli\", \"htmlcut-tempdir"));
    assert!(config.contains("crates/htmlcut-core/src/**/*.rs"));
    assert!(config.contains("crates/htmlcut-cli/src/**/*.rs"));
    assert!(config.contains("crates/htmlcut-tempdir/src/**/*.rs"));
    assert!(config.contains("**/src/tests/**/*.rs"));
    assert!(!config.contains("patches/rust"));
    assert!(!config.contains("xtask/src"));
}

#[test]
fn mutation_workflow_is_scheduled_sharded_and_retains_results() {
    let workflow = fs::read_to_string(repo_root().join(".github/workflows/mutants.yml"))
        .expect("read mutation workflow");

    assert!(workflow.contains("workflow_dispatch:"));
    assert!(workflow.contains("schedule:"));
    assert!(workflow.contains("pull_request:"));
    assert!(workflow.contains("\"0/16\""));
    assert!(workflow.contains("\"15/16\""));
    assert!(!workflow.contains("\"16/16\""));
    assert!(workflow.contains("cargo-mutants-${{ matrix.shard }}"));
    assert!(workflow.contains("./scripts/xtask.sh mutants --in-place --shard"));
    assert!(workflow.contains("--in-diff"));
    assert!(workflow.contains("all(.[];"));
    assert!(workflow.contains("htmlcut_contributor_install_action_csv cargo-mutants"));
    assert!(workflow.contains("workspaces: \". -> ../.htmlcut-artifacts/target\""));
    assert!(workflow.contains("mutation-runs/mutants.out"));
    assert!(workflow.contains("${{ runner.temp }}/htmlcut-mutation-results/mutants.out"));
    assert!(workflow.contains("Verify complete shards and summarize results"));
}
