// SPDX-License-Identifier: MPL-2.0
use super::*;

#[test]
fn first_partition_staging_failure_cannot_start_a_partial_batch() {
    use super::super::workspace::LocalMutationPartitionKind;
    let root = htmlcut_tempdir::tempdir().unwrap();
    let partitions = vec![LocalMutationPartition {
        label: "fixture".into(),
        kind: LocalMutationPartitionKind::WorkspaceShard("0/1".into()),
        expected_names: Default::default(),
    }];
    let result = run_partition_batches_with_lanes(
        &root.path().join("missing-source"),
        root.path(),
        &partitions,
        None,
        1,
        |_| panic!("failed staging must not execute workers"),
        || Ok(()),
    );
    assert!(result.is_err());
}
