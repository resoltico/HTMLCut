#![forbid(unsafe_code)]

use std::fs;
use std::process::Command;

#[cfg(unix)]
use htmlcut_tempdir::tempdir;
use serde_json::Value;
#[cfg(unix)]
use serde_json::json;

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
    let runtime_members = default_runtime_members();

    assert!(config.contains("all_features = true"));
    assert!(config.contains("additional_cargo_args = [\"--locked\"]"));
    assert!(config.contains("test_tool = \"cargo\""));
    assert!(config.contains("sharding = \"round-robin\""));
    for (package, member_path) in runtime_members {
        assert!(config.contains(&format!("\"{package}\"")));
        assert!(config.contains(&format!("{member_path}/src/**/*.rs")));
    }
    assert!(config.contains("**/src/tests/**/*.rs"));
    assert!(!config.contains("patches/rust"));
    assert!(!config.contains("xtask/src"));
}

fn default_runtime_members() -> Vec<(String, String)> {
    let output = Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .current_dir(repo_root())
        .output()
        .expect("run cargo metadata");
    assert!(
        output.status.success(),
        "cargo metadata failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metadata = serde_json::from_slice::<Value>(&output.stdout).expect("parse cargo metadata");
    let default_members = metadata["workspace_default_members"]
        .as_array()
        .expect("metadata workspace default members")
        .iter()
        .filter_map(Value::as_str)
        .collect::<Vec<_>>();
    metadata["packages"]
        .as_array()
        .expect("metadata packages")
        .iter()
        .filter(|package| {
            package["id"]
                .as_str()
                .is_some_and(|id| default_members.contains(&id))
        })
        .map(|package| {
            let manifest = std::path::Path::new(
                package["manifest_path"]
                    .as_str()
                    .expect("metadata package manifest path"),
            );
            let member_path = manifest
                .parent()
                .expect("package manifest parent")
                .strip_prefix(repo_root())
                .expect("default member under workspace root")
                .to_string_lossy()
                .into_owned();
            let name = package["name"]
                .as_str()
                .expect("metadata package name")
                .to_owned();
            (name, member_path)
        })
        .collect()
}

#[test]
fn mutation_workflow_is_scheduled_sharded_and_retains_results() {
    let workflow = fs::read_to_string(repo_root().join(".github/workflows/mutants.yml"))
        .expect("read mutation workflow");

    assert!(workflow.contains("workflow_dispatch:"));
    assert!(workflow.contains("schedule:"));
    assert!(workflow.contains("pull_request:"));
    assert!(!workflow.contains("    paths:"));
    assert!(workflow.contains("cancel-in-progress: ${{ github.event_name == 'pull_request' }}"));
    assert!(workflow.contains("shards=\"$(./scripts/mutation-shard-plan.sh)\""));
    assert_eq!(
        workflow
            .matches("./scripts/verify-mutation-scope.sh")
            .count(),
        2,
        "full and pull-request planning must share the canonical scope verifier"
    );
    assert!(workflow.contains("shard: ${{ fromJSON(needs.mutation-plan.outputs.shards) }}"));
    assert!(workflow.contains("mutation-diff-plan:"));
    assert!(workflow.contains(
        "cargo mutants --config .cargo/mutants.toml --in-diff \"$diff_path\" --list --json"
    ));
    assert!(workflow.contains("if (( mutants_status != 0 && mutants_status != 4 )); then"));
    assert!(workflow.contains("if [[ ! -s \"$RUNNER_TEMP/mutants.json\" ]]; then"));
    assert!(workflow.contains("printf '[]\\n' > \"$RUNNER_TEMP/mutants.json\""));
    assert!(workflow.contains("shards=\"$(./scripts/mutation-shard-plan.sh \"$shard_count\")\""));
    assert!(workflow.contains("shard: ${{ fromJSON(needs.mutation-diff-plan.outputs.shards) }}"));
    assert!(workflow.contains("needs.mutation-diff-plan.outputs.has_mutants == 'true'"));
    assert!(workflow.contains("--shard \"${{ matrix.shard.selector }}\""));
    assert!(workflow.contains("--shard \"${{ matrix.shard.selector }}\" --in-diff \"$diff_path\""));
    assert!(workflow.contains("name: ${{ matrix.shard.artifact_name }}"));
    assert!(!workflow.contains("cargo-mutants-${{ matrix.shard }}"));
    assert!(!workflow.contains("cargo-mutants-pr-diff"));
    assert!(workflow.contains("--in-diff"));
    assert!(workflow.contains("htmlcut_contributor_install_action_csv cargo-mutants"));
    assert!(workflow.contains("workspaces: \". -> ../.htmlcut-artifacts/target\""));
    assert!(workflow.contains("cache-directories: ../.htmlcut-artifacts/build"));
    assert!(workflow.contains("shared-key: htmlcut-mutation-workspace"));
    assert!(workflow.contains("save-if: false"));
    assert!(workflow.contains("mutation-runs/mutants.out"));
    assert!(workflow.contains("path: ${{ runner.temp }}/htmlcut-mutation-results\n"));
    assert!(workflow.contains("pattern: cargo-mutants-shard-*-of-*"));
    assert!(workflow.contains("merge-multiple: false"));
    assert!(workflow.contains("mutation-diff-summary:"));
    assert!(workflow.contains("MUTATION_DIFF_HAS_MUTANTS"));
    assert!(workflow.contains("MUTATION_EXPECTED_TOTAL"));
    assert!(workflow.contains("needs.mutation-diff-plan.result"));
    assert!(workflow.contains("needs.mutation-plan.result"));
    assert!(workflow.contains("No production Rust mutants overlap this pull request."));
    assert!(
        workflow
            .contains("- name: Verify complete shards and summarize results\n        if: always()")
    );
    assert!(workflow.contains("./scripts/summarize-mutation-results.sh"));
    assert!(!workflow.contains("expected outcomes from 16 shards"));
}

#[cfg(unix)]
#[test]
fn mutation_scope_verifier_tracks_cargo_default_members_and_rejects_drift() {
    let root = tempdir().expect("scope verifier fixture");
    let valid_path = root.path().join("valid-mutants.json");
    let invalid_path = root.path().join("invalid-mutants.json");
    let members = default_runtime_members();
    let mutants = members
        .iter()
        .map(|(package, member_path)| {
            json!({
                "package": package,
                "file": format!("{member_path}/src/lib.rs"),
            })
        })
        .collect::<Vec<_>>();
    fs::write(
        &valid_path,
        serde_json::to_vec(&mutants).expect("serialize valid mutation fixture"),
    )
    .expect("write valid mutation fixture");
    let mut invalid_mutants = mutants;
    invalid_mutants.pop();
    fs::write(
        &invalid_path,
        serde_json::to_vec(&invalid_mutants).expect("serialize invalid mutation fixture"),
    )
    .expect("write invalid mutation fixture");

    for (path, expected_success) in [(valid_path, true), (invalid_path, false)] {
        let output = Command::new("bash")
            .arg(repo_root().join("scripts").join("verify-mutation-scope.sh"))
            .arg(&path)
            .output()
            .expect("run mutation scope verifier");
        assert_eq!(
            output.status.success(),
            expected_success,
            "scope verifier stderr:\n{}\nfixture:\n{}",
            String::from_utf8_lossy(&output.stderr),
            // The fixture is included only when this regression fails, to make metadata/path drift actionable.
            fs::read_to_string(path).expect("read scope verifier fixture")
        );
    }
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

    let output = run_mutation_summary(&plan, &artifact_root, &summary_path, 16);
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
    expected_mutant_count: u64,
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
        .arg(expected_mutant_count.to_string())
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

    let output = run_mutation_summary(&plan, &artifact_root, &summary_path, 5);
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
fn mutation_summary_rejects_completed_zero_outcomes_when_the_enumerated_corpus_is_nonzero() {
    let root = tempdir().expect("mutation summary fixture");
    let artifact_root = root.path().join("artifacts");
    let summary_path = root.path().join("summary.md");
    let plan = two_shard_plan();
    for artifact_name in ["cargo-mutants-shard-0-of-2", "cargo-mutants-shard-1-of-2"] {
        write_outcome(
            &artifact_root,
            artifact_name,
            "mutants.out/outcomes.json",
            [0, 0, 0, 0, 0],
        );
    }

    let output = run_mutation_summary(&plan, &artifact_root, &summary_path, 2);

    assert!(
        !output.status.success(),
        "zero outcomes must not certify a nonzero corpus"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("expected 2 mutants, summarized 0"),
        "corpus mismatch should explain the incomplete campaign"
    );
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
        4,
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

    let output = run_mutation_summary(&plan, &artifact_root, &root.path().join("summary.md"), 4);
    assert!(
        !output.status.success(),
        "flattened artifact layout must fail"
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("mutants.out/outcomes.json"));
}
