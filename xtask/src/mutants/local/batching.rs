//! Capacity-aware execution and reuse of local mutation-workspace lanes.

use std::path::Path;

use crate::DynResult;

use super::MAX_SIMULTANEOUS_LOCAL_MUTATION_WORKSPACES;
use super::capacity::{available_workspace_lanes, ensure_runtime_batch_headroom};
use super::workspace::{
    CompletedMutationWorker, LocalMutationPartition, LocalMutationWorker, run_workers, stage_worker,
};

pub(super) fn run_partition_batches(
    repo_root: &Path,
    worker_root: &Path,
    partitions: &[LocalMutationPartition],
    in_diff: Option<&Path>,
) -> DynResult<(
    Vec<CompletedMutationWorker>,
    Vec<DynResult<std::process::ExitStatus>>,
)> {
    let requested_lanes = partitions
        .len()
        .min(MAX_SIMULTANEOUS_LOCAL_MUTATION_WORKSPACES);
    let workspace_lanes = available_workspace_lanes(requested_lanes, &std::env::temp_dir())?.get();
    run_partition_batches_with_lanes(
        repo_root,
        worker_root,
        partitions,
        in_diff,
        workspace_lanes,
        run_workers,
        || ensure_runtime_batch_headroom(&std::env::temp_dir()),
    )
}

pub(super) fn run_partition_batches_with_lanes(
    repo_root: &Path,
    worker_root: &Path,
    partitions: &[LocalMutationPartition],
    in_diff: Option<&Path>,
    workspace_lanes: usize,
    mut execute_workers: impl FnMut(&[LocalMutationWorker]) -> Vec<DynResult<std::process::ExitStatus>>,
    mut ensure_runtime_headroom: impl FnMut() -> DynResult<()>,
) -> DynResult<(
    Vec<CompletedMutationWorker>,
    Vec<DynResult<std::process::ExitStatus>>,
)> {
    assert!(
        workspace_lanes > 0,
        "local mutation execution requires a lane"
    );
    let mut completed_workers = Vec::with_capacity(partitions.len());
    let mut worker_results = Vec::with_capacity(partitions.len());
    let mut pending = partitions.iter().enumerate();
    let mut workers = pending
        .by_ref()
        .take(workspace_lanes)
        .map(|(index, partition)| stage_worker(repo_root, worker_root, index, partition, in_diff))
        .collect::<DynResult<Vec<_>>>()?;

    while !workers.is_empty() {
        ensure_runtime_headroom()?;
        let batch_results = execute_workers(&workers);
        let mut next_workers = Vec::with_capacity(workers.len());
        for (mut worker, result) in workers.into_iter().zip(batch_results) {
            let status = result?;
            worker_results.push(Ok(status));
            completed_workers.push(worker.completed());
            if let Some((index, partition)) = pending.next() {
                worker.retarget(worker_root, index, partition, in_diff)?;
                next_workers.push(worker);
            }
        }
        workers = next_workers;
    }

    completed_workers.sort_by_key(|worker| worker.index);
    Ok((completed_workers, worker_results))
}
