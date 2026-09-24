use htmlcut_tempdir::tempdir;

use super::*;

#[test]
fn reconciliation_rejects_duplicate_worker_mutants() {
    let root = tempdir().expect("temporary output root");
    let expected = vec![json!({"name": "duplicate"})];
    let workers = (0..2)
        .map(|index| worker_fixture(root.path(), index, "duplicate"))
        .collect::<Vec<_>>();
    let error = reconcile_workers(root.path(), &workers, &expected)
        .expect_err("duplicate worker mutations must be rejected");
    assert!(error.to_string().contains("duplicate mutant"));
}

#[test]
fn reconciliation_rejects_mismatched_counts_missing_outcomes_and_missing_runner_logs() {
    let root = tempdir().expect("temporary output root");
    let mismatch = worker_fixture(root.path(), 0, "one");
    let expected = vec![json!({"name": "one"})];
    let mismatched_worker = CompletedMutationWorker {
        expected_names: ["other".to_owned()].into_iter().collect(),
        ..mismatch
    };
    assert!(reconcile_workers(root.path(), &[mismatched_worker], &expected).is_err());

    let root = tempdir().expect("temporary output root");
    let worker = worker_fixture(root.path(), 0, "one");
    let outcome = worker.output_root.join("mutants.out/outcomes.json");
    fs::write(
        &outcome,
        serde_json::to_vec(&json!({
            "cargo_mutants_version":"27.1.0", "caught":0,
            "end_time":"done", "missed":0,
            "outcomes":[{"scenario":"Baseline", "summary":"Success"}],
            "start_time":"start", "timeout":0, "total_mutants":0, "unviable":0,
        }))
        .expect("outcome JSON"),
    )
    .expect("write count mismatch");
    assert!(reconcile_workers(root.path(), &[worker], &expected).is_err());

    let root = tempdir().expect("temporary output root");
    let worker = worker_fixture(root.path(), 0, "one");
    fs::write(
        worker.output_root.join("mutants.out/outcomes.json"),
        serde_json::to_vec(&json!({
            "cargo_mutants_version":"27.1.0", "caught":1,
            "end_time":"done", "missed":0,
            "start_time":"start", "timeout":0, "total_mutants":1, "unviable":0,
        }))
        .expect("outcome JSON"),
    )
    .expect("write outcomes omission");
    assert!(reconcile_workers(root.path(), &[worker], &expected).is_err());

    let root = tempdir().expect("temporary output root");
    let worker = worker_fixture(root.path(), 0, "one");
    fs::remove_file(worker.output_root.join("runner.stderr.log")).expect("remove runner log");
    assert!(reconcile_workers(root.path(), &[worker], &expected).is_err());
}

#[test]
fn reconciliation_reports_a_failed_baseline_before_count_or_inventory_errors() {
    let root = tempdir().expect("temporary output root");
    let worker = worker_fixture(root.path(), 0, "one");
    let outcome = worker.output_root.join("mutants.out/outcomes.json");
    fs::write(
        &outcome,
        serde_json::to_vec(&json!({
            "cargo_mutants_version":"27.1.0", "caught":0,
            "end_time":"done", "missed":0,
            "outcomes":[{"scenario":"Baseline", "summary":"Failure"}],
            "start_time":"start", "timeout":0, "total_mutants":0, "unviable":0,
        }))
        .expect("outcome JSON"),
    )
    .expect("write failed baseline outcome");

    let error = reconcile_workers(root.path(), &[worker], &[json!({"name": "one"})])
        .expect_err("failed baseline must reject the worker evidence");
    assert!(error.to_string().contains("baseline failed"));
    assert!(error.to_string().contains("log/baseline.log"));
}

#[test]
fn reconciliation_rejects_non_string_outcome_paths_after_preserving_worker_evidence() {
    let root = tempdir().expect("temporary output root");
    let worker = worker_fixture(root.path(), 0, "one");
    let outcome_path = worker.output_root.join("mutants.out/outcomes.json");
    let mut document: Value =
        serde_json::from_slice(&fs::read(&outcome_path).expect("read outcome"))
            .expect("parse outcome");
    document["outcomes"] = json!([
        {"scenario":"Baseline", "summary":"Success"},
        {"log_path": 7}
    ]);
    fs::write(
        &outcome_path,
        serde_json::to_vec(&document).expect("serialize outcome"),
    )
    .expect("write invalid outcome");

    assert!(reconcile_workers(root.path(), &[worker], &[json!({"name":"one"})]).is_err());
}

#[test]
fn aggregate_document_persistence_writes_all_documents_and_fails_closed() {
    let root = tempdir().expect("aggregate root");
    persist_aggregate_documents(
        root.path(),
        json!([]),
        json!({"outcomes": []}),
        json!({"shards": []}),
    )
    .expect("persist aggregate documents");
    for name in ["mutants.json", "outcomes.json", "local-aggregate.json"] {
        assert!(root.path().join(name).is_file());
    }

    let blocked = tempdir().expect("blocked aggregate root");
    fs::write(blocked.path().join("mutants.json"), "not a directory").expect("block output");
    assert!(
        persist_aggregate_documents(
            &blocked.path().join("mutants.json"),
            json!([]),
            json!({}),
            json!({}),
        )
        .is_err()
    );
}

#[test]
fn reconciliation_propagates_aggregate_persistence_failures() {
    let root = tempdir().expect("output root");
    let worker = worker_fixture(root.path(), 0, "one");
    let error = reconcile_workers_with(
        root.path(),
        &[worker],
        &[json!({"name":"one"})],
        |_root, _mutants, _outcomes, _manifest| Err("persistence denied".into()),
    )
    .expect_err("persistence failure");
    assert!(error.to_string().contains("persistence denied"));
}

#[test]
fn successful_baseline_evidence_allows_normal_reconciliation() {
    let root = tempdir().expect("temporary output root");
    let worker = worker_fixture(root.path(), 0, "one");
    let document = json!({"outcomes":[{"scenario":"Baseline", "summary":"Success"}]});
    ensure_worker_baseline_succeeded(&worker, &document, &worker.output_root.join("mutants.out"))
        .expect("successful baseline");
}

#[test]
fn baseline_and_outcome_parsers_reject_missing_or_inconsistent_contract_fields() {
    let root = tempdir().expect("temporary output root");
    let worker = worker_fixture(root.path(), 0, "one");
    let shard_root = worker.output_root.join("mutants.out");
    assert!(
        ensure_worker_baseline_succeeded(&worker, &json!({"outcomes": []}), &shard_root).is_err()
    );
    assert!(ensure_worker_baseline_succeeded(&worker, &json!({}), &shard_root).is_err());

    let inconsistent = json!({
        "cargo_mutants_version":"27.1.0", "caught":1,
        "end_time":"end", "missed":0, "start_time":"start",
        "timeout":0, "total_mutants":2, "unviable":0,
    });
    assert!(parse_worker_summary(&inconsistent, &shard_root).is_err());
    let empty_string = json!({
        "cargo_mutants_version":"", "caught":1,
        "end_time":"end", "missed":0, "start_time":"start",
        "timeout":0, "total_mutants":1, "unviable":0,
    });
    assert!(parse_worker_summary(&empty_string, &shard_root).is_err());
    let overflowing = json!({
        "cargo_mutants_version":"27.1.0", "caught":0,
        "end_time":"end", "missed":0, "start_time":"start",
        "timeout":u64::MAX, "total_mutants":0, "unviable":1,
    });
    assert!(parse_worker_summary(&overflowing, &shard_root).is_err());
    let aggregate_overflowing = json!({
        "cargo_mutants_version":"27.1.0", "caught":u64::MAX,
        "end_time":"end", "missed":0, "start_time":"start",
        "timeout":1, "total_mutants":0, "unviable":0,
    });
    assert!(parse_worker_summary(&aggregate_overflowing, &shard_root).is_err());
    assert!(ensure_unique_mutant_names(&[json!({})], "fixture").is_err());
    assert!(
        ensure_unique_mutant_names(
            &[json!({"name":"duplicate"}), json!({"name":"duplicate"})],
            "fixture",
        )
        .is_err()
    );
}

#[test]
fn reconciliation_rejects_missing_global_coverage_and_mixed_tool_versions() {
    let root = tempdir().expect("temporary output root");
    let worker = worker_fixture(root.path(), 0, "one");
    assert!(
        reconcile_workers(
            root.path(),
            &[worker],
            &[json!({"name": "one"}), json!({"name": "two"})]
        )
        .is_err()
    );

    let root = tempdir().expect("temporary output root");
    let first = worker_fixture(root.path(), 0, "one");
    let second = worker_fixture(root.path(), 1, "two");
    let second_outcome = second.output_root.join("mutants.out/outcomes.json");
    let mut document: Value =
        serde_json::from_slice(&fs::read(&second_outcome).expect("read second outcome"))
            .expect("parse second outcome");
    document["cargo_mutants_version"] = Value::String("different-version".to_owned());
    fs::write(
        &second_outcome,
        serde_json::to_vec(&document).expect("outcome JSON"),
    )
    .expect("write second outcome");
    assert!(
        reconcile_workers(
            root.path(),
            &[first, second],
            &[json!({"name": "one"}), json!({"name": "two"})],
        )
        .is_err()
    );
}

#[test]
fn reconciliation_emits_one_canonical_result_tree_with_exact_worker_evidence() {
    let root = tempdir().expect("temporary output root");
    let expected = vec![json!({"name": "one"}), json!({"name": "two"})];
    let workers = vec![
        worker_fixture(root.path(), 0, "one"),
        worker_fixture(root.path(), 1, "two"),
    ];
    let summary = reconcile_workers(root.path(), &workers, &expected)
        .expect("reconcile complete local mutation workers");
    assert_eq!(summary.total_mutants, 2);
    assert_eq!(summary.caught, 2);
    let canonical_root = root.path().join("mutants.out");
    assert!(!canonical_root.join("shards/0/outcomes.json").exists());
    assert!(canonical_root.join("shards/1/runner.stderr.log").is_file());
    assert!(!root.path().join("workers").exists());
    let aggregate: Value = serde_json::from_slice(
        &fs::read(canonical_root.join("outcomes.json")).expect("aggregate outcome JSON"),
    )
    .expect("parse aggregate outcome JSON");
    assert_eq!(aggregate["total_mutants"], 2);
    assert_eq!(aggregate["caught"], 2);
    let manifest: Value = serde_json::from_slice(
        &fs::read(canonical_root.join("local-aggregate.json")).expect("aggregate manifest JSON"),
    )
    .expect("parse aggregate manifest JSON");
    assert_eq!(manifest["schema"], LOCAL_AGGREGATE_SCHEMA);
    assert_eq!(manifest["expected_mutant_count"], 2);
}

#[test]
fn outcome_counts_are_exact_overflow_checked_and_clean_only_without_failures() {
    let summary = WorkerSummary {
        cargo_mutants_version: "27.1.0".to_owned(),
        caught: 3,
        end_time: "2026-09-08T00:00:01Z".to_owned(),
        missed: 2,
        start_time: "2026-09-08T00:00:00Z".to_owned(),
        timed_out: 1,
        total_mutants: 7,
        unviable: 1,
    };
    let mut counts = OutcomeCounts::default();
    counts.add(&summary).expect("aggregate counts");
    assert_eq!(counts.caught, 3);
    assert_eq!(counts.missed, 2);
    assert_eq!(counts.timed_out, 1);
    assert_eq!(counts.unviable, 1);
    assert_eq!(counts.total_mutants, 7);
    assert!(!counts.is_clean());
    assert!(
        !OutcomeCounts {
            caught: 1,
            missed: 0,
            timed_out: 1,
            total_mutants: 2,
            unviable: 0,
        }
        .is_clean()
    );
    assert!(
        OutcomeCounts {
            caught: 1,
            missed: 0,
            timed_out: 0,
            total_mutants: 1,
            unviable: 0,
        }
        .is_clean()
    );
    assert!(checked_count_sum(u64::MAX, 1).is_err());
    assert!(
        LocalMutationSummary {
            total_mutants: 1,
            caught: 1,
            missed: 0,
            timed_out: 0,
            unviable: 0
        }
        .is_clean()
    );
}

#[test]
fn worker_outcome_parsing_rejects_missing_malformed_and_inconsistent_contracts() {
    let root = tempdir().expect("temporary outcome root");
    let valid = json!({"cargo_mutants_version":"27.1.0","caught":1,"end_time":"2026-09-08T00:00:01Z","missed":0,"start_time":"2026-09-08T00:00:00Z","timeout":0,"total_mutants":1,"unviable":0});
    assert_eq!(
        parse_worker_summary(&valid, root.path())
            .expect("valid summary")
            .caught,
        1
    );
    assert!(
        parse_worker_summary(
            &json!({"total_mutants":1,"caught":1,"missed":0,"timeout":0,"unviable":1}),
            root.path()
        )
        .is_err()
    );
    assert!(parse_worker_summary(&json!({"total_mutants":"one"}), root.path()).is_err());
}

#[test]
fn outcome_rewriting_preserves_nulls_and_rejects_non_path_values() {
    let mut aggregate = Vec::new();
    append_worker_outcomes(
        &mut aggregate,
        &[
            json!({"log_path":"log/one.log","diff_path":null}),
            json!({"log_path":null,"diff_path":"diff/one.diff"}),
        ],
        4,
    )
    .expect("rewrite outcomes");
    assert_eq!(aggregate[0]["log_path"], "shards/4/log/one.log");
    assert_eq!(aggregate[1]["diff_path"], "shards/4/diff/one.diff");
    assert!(append_worker_outcomes(&mut aggregate, &[json!({"log_path":17})], 4).is_err());
}

#[test]
fn reconciliation_drops_non_actionable_logs_but_retains_actionable_paths() {
    let root = tempdir().expect("worker evidence root");
    fs::create_dir_all(root.path().join("log")).expect("log root");
    fs::create_dir_all(root.path().join("diff")).expect("diff root");
    for path in [
        "log/caught.log",
        "diff/caught.diff",
        "log/unviable.log",
        "log/missed.log",
        "diff/missed.diff",
    ] {
        fs::write(root.path().join(path), "evidence").expect("write evidence");
    }
    for name in [
        "outcomes.json",
        "mutants.json",
        "caught.txt",
        "missed.txt",
        "timeout.txt",
        "unviable.txt",
    ] {
        fs::write(root.path().join(name), "duplicate").expect("write duplicate evidence");
    }
    let outcomes = vec![
        json!({"summary":"CaughtMutant", "log_path":"log/caught.log", "diff_path":"diff/caught.diff"}),
        json!({"summary":"Unviable", "log_path":"log/unviable.log", "diff_path":null}),
        json!({"summary":"MissedMutant", "log_path":"log/missed.log", "diff_path":"diff/missed.diff"}),
    ];
    let mut aggregate = Vec::new();
    append_worker_outcomes(&mut aggregate, &outcomes, 3).expect("append aggregate outcomes");
    prune_live_non_actionable_evidence(root.path()).expect("live evidence pruning");
    retain_actionable_worker_evidence(root.path(), &outcomes).expect("trim worker evidence");

    assert!(aggregate[0]["log_path"].is_null());
    assert!(aggregate[0]["diff_path"].is_null());
    assert!(aggregate[1]["log_path"].is_null());
    assert_eq!(aggregate[2]["log_path"], "shards/3/log/missed.log");
    assert_eq!(aggregate[2]["diff_path"], "shards/3/diff/missed.diff");
    assert!(!root.path().join("log/caught.log").exists());
    assert!(!root.path().join("diff/caught.diff").exists());
    assert!(!root.path().join("log/unviable.log").exists());
    assert!(root.path().join("log/missed.log").is_file());
    assert!(root.path().join("diff/missed.diff").is_file());
    assert!(!root.path().join("outcomes.json").exists());
    assert!(!root.path().join("mutants.json").exists());
}

#[test]
fn live_pruning_tolerates_incomplete_publication_and_drops_only_completed_non_actionable_evidence()
{
    let root = tempdir().expect("live worker evidence root");
    assert!(prune_live_non_actionable_evidence(root.path()).is_ok());
    fs::write(root.path().join("outcomes.json"), "{").expect("incomplete outcome publication");
    assert!(prune_live_non_actionable_evidence(root.path()).is_ok());

    fs::create_dir_all(root.path().join("log")).expect("log root");
    fs::write(root.path().join("log/caught.log"), "evidence").expect("caught evidence");
    fs::write(root.path().join("log/missed.log"), "evidence").expect("missed evidence");
    fs::write(
        root.path().join("outcomes.json"),
        serde_json::to_vec(&json!({
            "outcomes": [
                {"summary":"CaughtMutant", "log_path":"log/caught.log"},
                {"summary":"MissedMutant", "log_path":"log/missed.log"}
            ]
        }))
        .expect("outcome JSON"),
    )
    .expect("write complete outcome publication");

    prune_live_non_actionable_evidence(root.path()).expect("prune completed outcome evidence");
    assert!(!root.path().join("log/caught.log").exists());
    assert!(root.path().join("log/missed.log").is_file());
}

#[test]
fn live_pruning_distinguishes_unreadable_and_shape_incomplete_outcomes() {
    let root = tempdir().expect("live worker evidence root");
    fs::create_dir(root.path().join("outcomes.json")).expect("outcome directory");
    assert!(prune_live_non_actionable_evidence(root.path()).is_err());
    fs::remove_dir(root.path().join("outcomes.json")).expect("remove outcome directory");
    fs::write(root.path().join("outcomes.json"), "{}").expect("shape-incomplete outcomes");
    assert!(prune_live_non_actionable_evidence(root.path()).is_ok());
}

#[test]
fn retention_rejects_unsafe_evidence_and_tolerates_already_removed_duplicates() {
    let root = tempdir().expect("worker root");
    retain_actionable_worker_evidence(root.path(), &[])
        .expect("missing duplicate files are harmless");

    for value in [json!(false), json!("/absolute.log"), json!("../escape.log")] {
        let outcomes = vec![json!({"summary":"CaughtMutant", "log_path":value})];
        assert!(retain_actionable_worker_evidence(root.path(), &outcomes).is_err());
    }

    fs::create_dir(root.path().join("outcomes.json")).expect("blocking directory");
    assert!(retain_actionable_worker_evidence(root.path(), &[]).is_err());

    fs::remove_dir(root.path().join("outcomes.json")).expect("remove blocking directory");
    fs::create_dir_all(root.path().join("log/caught.log")).expect("blocking evidence directory");
    let outcomes = vec![json!({"summary":"CaughtMutant", "log_path":"log/caught.log"})];
    assert!(retain_actionable_worker_evidence(root.path(), &outcomes).is_err());
}

#[test]
fn worker_readers_and_outcome_lists_fail_closed_on_bad_evidence() {
    let root = tempdir().expect("temporary worker root");
    assert!(read_worker_outcome(root.path()).is_err());
    assert!(read_worker_mutants(root.path()).is_err());
    fs::write(root.path().join("outcomes.json"), "not json").expect("bad outcome");
    fs::write(root.path().join("mutants.json"), "not json").expect("bad inventory");
    assert!(read_worker_outcome(root.path()).is_err());
    assert!(read_worker_mutants(root.path()).is_err());
    let mut lists = MutationOutcomeLists::default();
    assert!(lists.extend_from_worker(root.path()).is_err());
    for name in ["caught", "missed", "timeout", "unviable"] {
        fs::write(root.path().join(format!("{name}.txt")), "repeat\nrepeat\n")
            .expect("outcome list");
    }
    lists.extend_from_worker(root.path()).expect("read lists");
    let aggregate = root.path().join("aggregate");
    fs::create_dir_all(&aggregate).expect("aggregate root");
    lists.write_to(&aggregate).expect("write lists");
    assert_eq!(
        fs::read_to_string(aggregate.join("caught.txt")).expect("deduplicated list"),
        "repeat\n"
    );
}

fn worker_fixture(root: &Path, index: usize, mutant_name: &str) -> CompletedMutationWorker {
    let output_root = root.join("workers").join(index.to_string());
    let shard_root = output_root.join("mutants.out");
    fs::create_dir_all(shard_root.join("log")).expect("worker log root");
    fs::write(
        output_root.join("runner.stdout.log"),
        "runner standard output\n",
    )
    .expect("stdout log");
    fs::write(
        output_root.join("runner.stderr.log"),
        "runner standard error\n",
    )
    .expect("stderr log");
    fs::write(shard_root.join("outcomes.json"), serde_json::to_vec(&json!({"cargo_mutants_version":"27.1.0","caught":1,"end_time":"2026-09-08T00:00:01Z","missed":0,"outcomes":[{"scenario":"Baseline","summary":"Success"}],"start_time":"2026-09-08T00:00:00Z","timeout":0,"total_mutants":1,"unviable":0})).expect("outcome JSON")).expect("write outcome");
    fs::write(
        shard_root.join("mutants.json"),
        serde_json::to_vec(&vec![json!({"name":mutant_name})]).expect("mutants JSON"),
    )
    .expect("write mutants");
    for name in ["caught", "missed", "timeout", "unviable"] {
        fs::write(shard_root.join(format!("{name}.txt")), "").expect("outcome list");
    }
    CompletedMutationWorker {
        index,
        selector: format!("{index}/2"),
        expected_names: [mutant_name.to_owned()].into_iter().collect(),
        output_root,
    }
}
