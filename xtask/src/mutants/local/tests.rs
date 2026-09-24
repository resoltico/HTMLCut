#[cfg(unix)]
use std::cell::Cell;
#[cfg(unix)]
use std::collections::BTreeSet;
use std::path::Path;
#[cfg(unix)]
use std::{fs, process::Command};

#[cfg(unix)]
use super::batching::run_partition_batches_with_lanes;
use super::*;

fn inventory(package: &str, count: usize) -> Vec<Value> {
    (0..count)
        .map(|index| serde_json::json!({"name": format!("{package}-{index}"), "package": package}))
        .collect()
}

#[test]
fn local_partitions_cover_requested_and_package_scoped_inventories_exactly() {
    let expected = inventory("fixture", 2);
    let requested = local_partitions(Path::new("/fixture"), &expected, Some("0/1"), None)
        .expect("requested partition");
    assert_eq!(requested.len(), 1);
    assert!(matches!(
        requested[0].kind,
        LocalMutationPartitionKind::WorkspaceShard(ref selector) if selector == "0/1"
    ));
    let package_inventory = serde_json::to_vec(&expected).expect("inventory JSON");
    let package_partitions = crate::command_exec::with_capture_command_output_override(
        move |_root, spec| {
            (spec.args.first().map(String::as_str) == Some("mutants"))
                .then(|| Ok(package_inventory.clone()))
        },
        || local_partitions(Path::new("/fixture"), &expected, None, None),
    )
    .expect("package partitions");
    assert_eq!(package_partitions.len(), 1);
    assert_eq!(package_partitions[0].expected_names.len(), 2);
}

#[test]
fn empty_inventory_finishes_without_staging_a_mutation_workspace() {
    let repository = htmlcut_tempdir::tempdir().expect("repository root");
    let output = htmlcut_tempdir::tempdir().expect("output root");
    crate::command_exec::with_capture_command_output_override(
        |_root, _spec| Some(Ok(Vec::new())),
        || {
            let summary = run_local_mutation_campaign(repository.path(), output.path(), None, None)
                .expect("empty mutation inventory");
            assert_eq!(summary.total_mutants, 0);
            assert!(!output.path().join("local-shard-runs").exists());
        },
    );
}

#[cfg(unix)]
#[test]
fn partition_batches_reuse_a_bounded_lane_for_later_partitions() {
    let repository = htmlcut_tempdir::tempdir().expect("repository root");
    fs::write(repository.path().join("Cargo.toml"), "[workspace]\n").expect("workspace manifest");
    let output = htmlcut_tempdir::tempdir().expect("output root");
    let partitions = (0..5)
        .map(|index| LocalMutationPartition {
            label: format!("fixture:{index}"),
            kind: LocalMutationPartitionKind::WorkspaceShard(format!("{index}/5")),
            expected_names: BTreeSet::from([format!("mutant-{index}")]),
        })
        .collect::<Vec<_>>();
    let mut batch_sizes = Vec::new();
    let (completed, outcomes) = run_partition_batches_with_lanes(
        repository.path(),
        output.path(),
        &partitions,
        None,
        1,
        |workers| {
            batch_sizes.push(workers.len());
            workers
                .iter()
                .map(|_| {
                    Command::new("sh")
                        .args(["-c", "exit 0"])
                        .status()
                        .map_err(Into::into)
                })
                .collect()
        },
        || Ok(()),
    )
    .expect("bounded partition batches");

    assert!(
        batch_sizes.len() >= 2,
        "five partitions must reuse at least one lane"
    );
    assert_eq!(batch_sizes.iter().sum::<usize>(), 5);
    assert!(batch_sizes.iter().all(|size| (1..=4).contains(size)));
    assert_eq!(completed.len(), 5);
    assert_eq!(
        completed
            .iter()
            .map(|worker| worker.index)
            .collect::<Vec<_>>(),
        vec![0, 1, 2, 3, 4]
    );
    assert_eq!(outcomes.len(), 5);
    assert!(
        outcomes
            .into_iter()
            .all(|status| status.expect("status").success())
    );
}

#[cfg(unix)]
#[test]
fn partition_batches_check_runtime_headroom_before_every_later_batch() {
    let repository = htmlcut_tempdir::tempdir().expect("repository root");
    fs::write(repository.path().join("Cargo.toml"), "[workspace]\n").expect("workspace manifest");
    let output = htmlcut_tempdir::tempdir().expect("output root");
    let partitions = (0..2)
        .map(|index| LocalMutationPartition {
            label: format!("fixture:{index}"),
            kind: LocalMutationPartitionKind::WorkspaceShard(format!("{index}/2")),
            expected_names: BTreeSet::from([format!("mutant-{index}")]),
        })
        .collect::<Vec<_>>();
    let checks = Cell::new(0_usize);
    let executed = Cell::new(0_usize);

    let result = run_partition_batches_with_lanes(
        repository.path(),
        output.path(),
        &partitions,
        None,
        1,
        |workers| {
            executed.set(executed.get() + workers.len());
            workers
                .iter()
                .map(|_| {
                    Command::new("sh")
                        .args(["-c", "exit 0"])
                        .status()
                        .map_err(Into::into)
                })
                .collect()
        },
        || {
            let prior = checks.get();
            checks.set(prior + 1);
            (prior == 0)
                .then_some(())
                .ok_or_else(|| "runtime mutation headroom exhausted".into())
        },
    );

    assert!(result.is_err());
    assert_eq!(checks.get(), 2);
    assert_eq!(executed.get(), 1);
}

#[test]
fn mutation_inventory_and_partitioning_fail_closed_on_command_failure_or_duplicate_shards() {
    crate::command_exec::with_capture_command_output_override(
        |_root, _spec| Some(Err("inventory unavailable".into())),
        || {
            assert!(list_package_mutants(Path::new("/fixture"), &[], "0/1", None).is_err());
        },
    );

    let expected = inventory("fixture", 128);
    let repeated_inventory = serde_json::to_vec(&expected).expect("inventory JSON");
    let result = crate::command_exec::with_capture_command_output_override(
        move |_root, _spec| Some(Ok(repeated_inventory.clone())),
        || local_partitions(Path::new("/fixture"), &expected, None, None),
    );
    let error = match result {
        Ok(_) => panic!("duplicate shard inventory must fail"),
        Err(error) => error,
    };
    assert!(error.to_string().contains("duplicate mutant"));
}
