//! Supervised execution of one disposable local mutation worker.

use std::fs::File;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use crate::{CommandArtifactLayout, DynResult};

use super::super::capacity::ensure_runtime_batch_headroom;
use super::super::reconcile::prune_live_non_actionable_evidence;
use super::{
    LIVE_WORKER_SUPERVISION_INTERVAL, LocalMutationWorker, clear_inherited_jobserver,
    protect_worker_source_tree, restore_worker_source_tree, worker_environment,
};

pub(super) fn run_worker(
    worker: &LocalMutationWorker,
    cargo_build_jobs: usize,
) -> DynResult<std::process::ExitStatus> {
    run_worker_with(worker, cargo_build_jobs, supervise_worker)
}

pub(super) fn run_worker_with(
    worker: &LocalMutationWorker,
    cargo_build_jobs: usize,
    supervise: impl FnOnce(&mut Command, &Path) -> DynResult<std::process::ExitStatus>,
) -> DynResult<std::process::ExitStatus> {
    if !matches!(
        worker.command.artifact_layout,
        CommandArtifactLayout::Inherit
    ) {
        return Err("local mutation workers require workspace-local Cargo artifacts".into());
    }

    let workspace_root = worker.workspace.path().join("workspace");
    let _protection = protect_worker_source_tree(&workspace_root)?;
    let stdout = File::create(worker.output_root.join("runner.stdout.log"))?;
    let stderr = File::create(worker.output_root.join("runner.stderr.log"))?;
    let mut command = Command::new(&worker.command.program);
    command
        .current_dir(&workspace_root)
        .args(&worker.command.args)
        .envs(worker_environment(
            worker,
            cargo_build_jobs,
            &worker.cargo_home,
        ));
    clear_inherited_jobserver(&mut command);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr));
    let status = supervise(&mut command, &worker.output_root);
    let thaw = restore_worker_source_tree(&workspace_root);
    let restored = thaw.and_then(|_| worker.source_fingerprint.verify(&workspace_root));
    combine_worker_execution(status, restored)
}

fn supervise_worker(
    command: &mut Command,
    output_root: &Path,
) -> DynResult<std::process::ExitStatus> {
    supervise_worker_with(
        command,
        output_root,
        prune_live_non_actionable_evidence,
        || ensure_runtime_batch_headroom(&std::env::temp_dir()),
        LIVE_WORKER_SUPERVISION_INTERVAL,
    )
}

pub(super) fn supervise_worker_with(
    command: &mut Command,
    output_root: &Path,
    mut prune_evidence: impl FnMut(&Path) -> DynResult<()>,
    mut ensure_headroom: impl FnMut() -> DynResult<()>,
    interval: Duration,
) -> DynResult<std::process::ExitStatus> {
    let mut child = command.spawn()?;
    loop {
        if let Some(status) = child.try_wait()? {
            prune_evidence(&output_root.join("mutants.out"))?;
            return Ok(status);
        }
        if let Err(error) = prune_evidence(&output_root.join("mutants.out")) {
            terminate_worker_child(&mut child);
            return Err(error);
        }
        if let Err(error) = ensure_headroom() {
            terminate_worker_child(&mut child);
            return Err(error);
        }
        thread::sleep(interval);
    }
}

fn terminate_worker_child(child: &mut std::process::Child) {
    let _ = child.kill();
    let _ = child.wait();
}

pub(super) fn combine_worker_execution(
    status: DynResult<std::process::ExitStatus>,
    restored: DynResult<()>,
) -> DynResult<std::process::ExitStatus> {
    match (status, restored) {
        (Ok(status), Ok(())) => Ok(status),
        (Err(status_error), Ok(())) => Err(status_error),
        (Ok(_), Err(restore_error)) => Err(restore_error),
        (Err(status_error), Err(restore_error)) => Err(format!(
            "local mutation worker could not run its command ({status_error}) and could not restore its pristine source workspace ({restore_error})"
        )
        .into()),
    }
}

#[cfg(test)]
mod tests {
    use htmlcut_tempdir::tempdir;

    use super::*;

    #[test]
    fn supervisor_allows_a_completed_child_when_pruning_and_headroom_are_healthy() {
        let output_root = tempdir().expect("output root");
        #[cfg(unix)]
        let mut command = Command::new("sh");
        #[cfg(unix)]
        command.args(["-c", "exit 0"]);
        #[cfg(windows)]
        let mut command = Command::new("cmd");
        #[cfg(windows)]
        command.args(["/C", "exit 0"]);
        let status = supervise_worker_with(
            &mut command,
            output_root.path(),
            |_| Ok(()),
            || Ok(()),
            Duration::ZERO,
        )
        .expect("healthy supervision");
        assert!(status.success());
    }

    #[cfg(unix)]
    #[test]
    fn supervisor_terminates_children_on_pruning_or_headroom_failure() {
        let output_root = tempdir().expect("output root");
        for (expected, prune_evidence, ensure_headroom) in [
            (
                "evidence pruning failed",
                Box::new(|_: &Path| Err("evidence pruning failed".into()))
                    as Box<dyn FnMut(&Path) -> DynResult<()>>,
                Box::new(|| Ok(())) as Box<dyn FnMut() -> DynResult<()>>,
            ),
            (
                "runtime headroom failed",
                Box::new(|_: &Path| Ok(())) as Box<dyn FnMut(&Path) -> DynResult<()>>,
                Box::new(|| Err("runtime headroom failed".into()))
                    as Box<dyn FnMut() -> DynResult<()>>,
            ),
        ] {
            let mut command = Command::new("sh");
            command.args(["-c", "sleep 60"]);
            let error = supervise_worker_with(
                &mut command,
                output_root.path(),
                prune_evidence,
                ensure_headroom,
                Duration::ZERO,
            )
            .expect_err("supervisor must stop the child on a safety failure");
            assert!(error.to_string().contains(expected));
        }
    }

    #[cfg(unix)]
    #[test]
    fn terminating_a_worker_waits_for_the_child_to_exit() {
        let mut child = Command::new("sh")
            .args(["-c", "sleep 60"])
            .spawn()
            .expect("spawn child");

        terminate_worker_child(&mut child);

        assert!(child.try_wait().expect("poll child").is_some());
    }
}
