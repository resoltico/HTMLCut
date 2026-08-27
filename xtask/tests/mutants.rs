#![forbid(unsafe_code)]

use std::fs;

#[cfg(unix)]
use std::process::Command;

#[cfg(unix)]
use htmlcut_tempdir::tempdir;
#[cfg(unix)]
use serde_json::{Value, json};

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
    assert!(workflow.contains("shards=\"$(./scripts/mutation-shard-plan.sh)\""));
    assert!(workflow.contains("shard: ${{ fromJSON(needs.mutation-plan.outputs.shards) }}"));
    assert!(workflow.contains("mutation-diff-plan:"));
    assert!(workflow.contains("cargo mutants --config .cargo/mutants.toml --in-diff \"$diff_path\" --list --json"));
    assert!(workflow.contains("shards=\"$(./scripts/mutation-shard-plan.sh \"$shard_count\")\""));
    assert!(workflow.contains("shard: ${{ fromJSON(needs.mutation-diff-plan.outputs.shards) }}"));
    assert!(workflow.contains("needs.mutation-diff-plan.outputs.has_mutants == 'true'"));
    assert!(workflow.contains("--shard \"${{ matrix.shard.selector }}\""));
    assert!(workflow.contains("--shard \"${{ matrix.shard.selector }}\" --in-diff \"$diff_path\""));
    assert!(workflow.contains("name: ${{ matrix.shard.artifact_name }}"));
    assert!(!workflow.contains("cargo-mutants-${{ matrix.shard }}"));
    assert!(!workflow.contains("cargo-mutants-pr-diff"));
    assert!(workflow.contains("--in-diff"));
    assert!(workflow.contains("all(.[];"));
    assert!(workflow.contains("htmlcut_contributor_install_action_csv cargo-mutants"));
    assert!(workflow.contains("workspaces: \". -> ../.htmlcut-artifacts/target\""));
    assert!(workflow.contains("mutation-runs/mutants.out"));
    assert!(workflow.contains("path: ${{ runner.temp }}/htmlcut-mutation-results\n"));
    assert!(workflow.contains("pattern: cargo-mutants-shard-*-of-*"));
    assert!(workflow.contains("merge-multiple: false"));
    assert!(workflow.contains("mutation-diff-summary:"));
    assert!(workflow.contains("MUTATION_DIFF_HAS_MUTANTS"));
    assert!(workflow.contains("No production Rust mutants overlap this pull request."));
    assert!(
        workflow
            .contains("- name: Verify complete shards and summarize results\n        if: always()")
    );
    assert!(workflow.contains("./scripts/summarize-mutation-results.sh"));
    assert!(!workflow.contains("expected outcomes from 16 shards"));
}

#[cfg(unix)]
fn two_shard_plan() -> Value {
    json!([
        {
            "selector": "0/2",
            "artifact_name": "cargo-mutants-shard-0-of-2"
        },
        {
            "selector": "1/2",
            "artifact_name": "cargo-mutants-shard-1-of-2"
        }
    ])
}

#[cfg(unix)]
fn generated_shard_plan() -> Value {
    generated_shard_plan_with_total(None)
}

#[cfg(unix)]
fn generated_shard_plan_with_total(shard_total: Option<usize>) -> Value {
    let mut command = Command::new("bash");
    command.arg(repo_root().join("scripts").join("mutation-shard-plan.sh"));
    if let Some(shard_total) = shard_total {
        command.arg(shard_total.to_string());
    }
    let output = command.output().expect("run mutation shard plan");
    assert!(
        output.status.success(),
        "mutation shard plan failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    serde_json::from_slice::<Value>(&output.stdout).expect("parse mutation shard plan")
}

#[cfg(unix)]
#[test]
fn mutation_shard_plan_covers_every_selector_with_safe_unique_artifact_identity() {
    let plan = generated_shard_plan();
    let shards = plan.as_array().expect("mutation shard plan array");
    assert_eq!(shards.len(), 16);
    for (index, shard) in shards.iter().enumerate() {
        assert_eq!(shard["selector"], format!("{index}/16"));
        assert_eq!(
            shard["artifact_name"],
            format!("cargo-mutants-shard-{index}-of-16")
        );
        assert!(
            !shard["artifact_name"]
                .as_str()
                .expect("artifact name")
                .contains('/')
        );
    }
}

#[cfg(unix)]
#[test]
fn mutation_shard_plan_accepts_a_requested_nonzero_shard_total() {
    let plan = generated_shard_plan_with_total(Some(2));
    assert_eq!(
        plan,
        json!([
            {
                "selector": "0/2",
                "artifact_name": "cargo-mutants-shard-0-of-2"
            },
            {
                "selector": "1/2",
                "artifact_name": "cargo-mutants-shard-1-of-2"
            }
        ])
    );
}

#[cfg(unix)]
#[test]
fn mutation_shard_plan_rejects_an_invalid_requested_total() {
    let output = Command::new("bash")
        .arg(repo_root().join("scripts").join("mutation-shard-plan.sh"))
        .arg("0")
        .output()
        .expect("run invalid mutation shard plan");

    assert!(!output.status.success(), "zero shard count must fail");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("positive integer"),
        "invalid shard count should be actionable"
    );
}

#[cfg(unix)]
#[test]
fn complete_generated_shard_plan_composes_with_the_summary_contract() {
    let root = tempdir().expect("mutation summary fixture");
    let artifact_root = root.path().join("artifacts");
    let summary_path = root.path().join("summary.md");
    let plan = generated_shard_plan();
    for shard in plan.as_array().expect("mutation shard plan array") {
        write_outcome(
            &artifact_root,
            shard["artifact_name"].as_str().expect("artifact name"),
            "mutants.out/outcomes.json",
            [1, 1, 0, 0, 0],
        );
    }

    let output = run_mutation_summary(&plan, &artifact_root, &summary_path);
    assert!(
        output.status.success(),
        "complete mutation summary failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let summary = fs::read_to_string(summary_path).expect("read mutation summary");
    assert_eq!(summary.matches("| `").count(), 16);
    assert!(summary.contains("| `0/16` |"));
    assert!(summary.contains("| `15/16` |"));
    assert!(summary.contains("Total: 16 mutants; 16 caught;"));
}

#[cfg(unix)]
fn write_outcome(
    artifact_root: &std::path::Path,
    artifact_name: &str,
    relative_outcome_path: &str,
    counts: [u64; 5],
) {
    let outcome_path = artifact_root
        .join(artifact_name)
        .join(relative_outcome_path);
    fs::create_dir_all(outcome_path.parent().expect("outcome parent"))
        .expect("create outcome parent");
    fs::write(
        outcome_path,
        serde_json::to_vec_pretty(&json!({
            "end_time": "2026-08-25T12:00:00Z",
            "total_mutants": counts[0],
            "caught": counts[1],
            "missed": counts[2],
            "timeout": counts[3],
            "unviable": counts[4]
        }))
        .expect("serialize outcome"),
    )
    .expect("write outcome");
}

#[cfg(unix)]
fn run_mutation_summary(
    plan: &Value,
    artifact_root: &std::path::Path,
    summary_path: &std::path::Path,
) -> std::process::Output {
    Command::new("bash")
        .arg(
            repo_root()
                .join("scripts")
                .join("summarize-mutation-results.sh"),
        )
        .arg(plan.to_string())
        .arg(artifact_root)
        .arg(summary_path)
        .output()
        .expect("run mutation summary")
}

#[cfg(unix)]
#[test]
fn mutation_summary_verifies_exact_artifacts_and_aggregates_completed_outcomes() {
    let root = tempdir().expect("mutation summary fixture");
    let artifact_root = root.path().join("artifacts");
    let summary_path = root.path().join("summary.md");
    let plan = two_shard_plan();
    write_outcome(
        &artifact_root,
        "cargo-mutants-shard-0-of-2",
        "mutants.out/outcomes.json",
        [2, 1, 1, 0, 0],
    );
    write_outcome(
        &artifact_root,
        "cargo-mutants-shard-1-of-2",
        "mutants.out/outcomes.json",
        [3, 2, 0, 1, 0],
    );

    let output = run_mutation_summary(&plan, &artifact_root, &summary_path);
    assert!(
        output.status.success(),
        "mutation summary failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let summary = fs::read_to_string(summary_path).expect("read mutation summary");
    assert!(summary.contains("| `0/2` | 2 | 1 | 1 | 0 | 0 |"));
    assert!(summary.contains("| `1/2` | 3 | 2 | 0 | 1 | 0 |"));
    assert_eq!(
        summary.matches("Total:").count(),
        1,
        "aggregate summary line must appear exactly once"
    );
    assert!(summary.contains("Total: 5 mutants; 3 caught; 1 missed; 1 timed out; 0 unviable."));
}

#[cfg(unix)]
#[test]
fn mutation_summary_rejects_a_count_correct_but_wrong_artifact_identity_set() {
    let root = tempdir().expect("mutation summary fixture");
    let artifact_root = root.path().join("artifacts");
    fs::create_dir_all(artifact_root.join("cargo-mutants-shard-0-of-2"))
        .expect("create expected artifact");
    fs::create_dir_all(artifact_root.join("cargo-mutants-shard-2-of-2"))
        .expect("create unexpected artifact");

    let output = run_mutation_summary(
        &two_shard_plan(),
        &artifact_root,
        &root.path().join("summary.md"),
    );
    assert!(!output.status.success(), "wrong artifact set must fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("missing: cargo-mutants-shard-1-of-2"));
    assert!(stderr.contains("unexpected: cargo-mutants-shard-2-of-2"));
}

#[cfg(unix)]
#[test]
fn mutation_summary_rejects_an_upload_that_flattened_the_mutants_out_root() {
    let root = tempdir().expect("mutation summary fixture");
    let artifact_root = root.path().join("artifacts");
    let plan = two_shard_plan();
    write_outcome(
        &artifact_root,
        "cargo-mutants-shard-0-of-2",
        "outcomes.json",
        [2, 2, 0, 0, 0],
    );
    write_outcome(
        &artifact_root,
        "cargo-mutants-shard-1-of-2",
        "outcomes.json",
        [2, 2, 0, 0, 0],
    );

    let output = run_mutation_summary(&plan, &artifact_root, &root.path().join("summary.md"));
    assert!(
        !output.status.success(),
        "flattened artifact layout must fail"
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("mutants.out/outcomes.json"));
}
