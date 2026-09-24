//! Disposable source-workspace materialization and worker process ownership.

use std::collections::BTreeMap;
use std::fs::{self};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::Duration;

use htmlcut_tempdir::{TempDir, tempdir};
#[cfg(test)]
use std::collections::BTreeSet;

#[cfg(test)]
use crate::CommandArtifactLayout;
#[cfg(test)]
use crate::model::MAINTAINED_NIGHTLY_TOOLCHAIN_NAME;
use crate::{CommandSpec, DynResult, XtaskError};

use super::super::{mutants_command, mutants_package_command};

/// Cargo inherits GNU Make's jobserver when these variables are present. An outer launcher can
/// leave descriptors that are no longer serviced; local mutation workers must rely exclusively
/// on their explicit `CARGO_BUILD_JOBS` budget instead of inheriting that dead coordination state.
const INHERITED_JOBSERVER_ENVIRONMENT: &[&str] = &["MAKEFLAGS", "MFLAGS", "CARGO_MAKEFLAGS"];
const LIVE_WORKER_SUPERVISION_INTERVAL: Duration = Duration::from_secs(1);

mod integrity;
mod materialization;
mod protection;
mod runner;

use integrity::SourceTreeFingerprint;
use materialization::materialize_mutation_workspace;
use protection::{
    prepare_worker_cargo_home_at, protect_worker_source_tree, restore_worker_source_tree,
};
use runner::run_worker;
#[cfg(all(test, unix))]
use runner::{combine_worker_execution, run_worker_with, supervise_worker_with};

/// One independently disposable source workspace and its result root.
pub(super) struct LocalMutationWorker {
    /// Zero-based worker position within the local campaign.
    pub(super) index: usize,
    /// Canonical cargo-mutants partition selector owned by this worker.
    pub(super) selector: String,
    /// Exact planned mutant names this worker alone must account for.
    pub(super) expected_names: std::collections::BTreeSet<String>,
    pub(super) workspace: TempDir,
    /// Private offline Cargo home whose locks cannot contend with another worker.
    pub(super) cargo_home: PathBuf,
    /// Digest of every immutable source input that must be restored before this workspace can
    /// execute another partition.
    source_fingerprint: SourceTreeFingerprint,
    /// Parent passed to cargo-mutants, which receives its `mutants.out` child.
    pub(super) output_root: PathBuf,
    pub(super) command: CommandSpec,
}

/// Retained result metadata after the disposable source workspace has been released.
pub(super) struct CompletedMutationWorker {
    pub(super) index: usize,
    pub(super) selector: String,
    pub(super) expected_names: std::collections::BTreeSet<String>,
    pub(super) output_root: PathBuf,
}

impl LocalMutationWorker {
    pub(super) fn completed(&self) -> CompletedMutationWorker {
        CompletedMutationWorker {
            index: self.index,
            selector: self.selector.clone(),
            expected_names: self.expected_names.clone(),
            output_root: self.output_root.clone(),
        }
    }

    /// Reuses this verified-clean source workspace for one later exact mutation partition.
    pub(super) fn retarget(
        &mut self,
        worker_root: &Path,
        index: usize,
        partition: &LocalMutationPartition,
        in_diff: Option<&Path>,
    ) -> DynResult<()> {
        let output_root = worker_root.join(index.to_string());
        fs::create_dir_all(&output_root)?;
        self.index = index;
        self.selector = partition.label.clone();
        self.expected_names = partition.expected_names.clone();
        self.output_root = output_root.clone();
        self.command = command_for_partition(partition, &output_root, in_diff);
        Ok(())
    }
}

/// Stages one exact partition at its stable campaign index.
pub(super) fn stage_worker(
    repo_root: &Path,
    worker_root: &Path,
    index: usize,
    partition: &LocalMutationPartition,
    in_diff: Option<&Path>,
) -> DynResult<LocalMutationWorker> {
    let workspace = tempdir()?;
    let workspace_root = workspace.path().join("workspace");
    materialize_mutation_workspace(repo_root, &workspace_root)?;
    let cargo_home = prepare_worker_cargo_home_at(&workspace_root)?;
    let source_fingerprint = SourceTreeFingerprint::from_workspace(&workspace_root)?;
    let mut worker = LocalMutationWorker {
        index,
        workspace,
        cargo_home,
        source_fingerprint,
        selector: String::new(),
        expected_names: std::collections::BTreeSet::new(),
        output_root: PathBuf::new(),
        command: CommandSpec::new(
            "cargo",
            ["mutants"],
            crate::CommandStdout::Quiet,
            crate::CommandToolchainEnv::Inherit,
        ),
    };
    worker.retarget(worker_root, index, partition, in_diff)?;
    Ok(worker)
}

fn command_for_partition(
    partition: &LocalMutationPartition,
    output_root: &Path,
    in_diff: Option<&Path>,
) -> CommandSpec {
    match &partition.kind {
        LocalMutationPartitionKind::WorkspaceShard(selector) => {
            mutants_command(output_root, false, Some(selector), in_diff)
        }
        LocalMutationPartitionKind::PackageShard {
            packages,
            shard_selector,
        } => mutants_package_command(output_root, packages, shard_selector, in_diff),
    }
}

/// One exact mutation partition assigned to a disposable local worker.
pub(super) struct LocalMutationPartition {
    /// Human-readable partition identity retained in aggregate evidence.
    pub(super) label: String,
    /// Source-contract scope and zero-based shard selector owned by the worker.
    pub(super) kind: LocalMutationPartitionKind,
    /// Exact names that the partition must produce.
    pub(super) expected_names: std::collections::BTreeSet<String>,
}

/// The source scope used by one local mutation worker.
pub(super) enum LocalMutationPartitionKind {
    /// An explicit caller-selected workspace shard.
    WorkspaceShard(String),
    /// One or more low-coupling packages split across independent local shards.
    PackageShard {
        packages: Vec<String>,
        shard_selector: String,
    },
}

/// Runs all already-staged workers concurrently while their disposable workspaces stay alive.
pub(super) fn run_workers(
    workers: &[LocalMutationWorker],
) -> Vec<DynResult<std::process::ExitStatus>> {
    let available_parallelism = std::thread::available_parallelism()
        .map(std::num::NonZeroUsize::get)
        .unwrap_or(1);
    let cargo_build_jobs = bounded_cargo_build_jobs(available_parallelism, workers.len());
    thread::scope(|scope| {
        let joins = workers
            .iter()
            .map(|worker| scope.spawn(|| run_worker(worker, cargo_build_jobs.get())))
            .collect::<Vec<_>>();
        joins
            .into_iter()
            .map(|join| {
                join.join()
                    .map_err(|_| XtaskError::from("local cargo-mutants worker panicked"))?
            })
            .collect()
    })
}

fn clear_inherited_jobserver(command: &mut Command) {
    for name in INHERITED_JOBSERVER_ENVIRONMENT {
        command.env_remove(name);
    }
}

fn bounded_cargo_build_jobs(available: usize, worker_count: usize) -> std::num::NonZeroUsize {
    let workers = worker_count.max(1);
    std::num::NonZeroUsize::new((available.max(1) / workers).max(1))
        .expect("the worker build-job budget is always at least one")
}

fn worker_environment(
    worker: &LocalMutationWorker,
    cargo_build_jobs: usize,
    cargo_home: &Path,
) -> BTreeMap<String, String> {
    let workspace_root = worker.workspace.path().join("workspace");
    worker
        .command
        .env
        .iter()
        .map(|(key, value)| {
            let value = if matches!(key.as_str(), "CARGO_TARGET_DIR" | "CARGO_BUILD_BUILD_DIR") {
                let path = Path::new(value);
                if path.is_absolute() {
                    value.clone()
                } else {
                    workspace_root.join(path).to_string_lossy().into_owned()
                }
            } else {
                value.clone()
            };
            (key.clone(), value)
        })
        .chain(std::iter::once((
            "CARGO_BUILD_JOBS".to_owned(),
            cargo_build_jobs.max(1).to_string(),
        )))
        .chain([
            (
                "CARGO_HOME".to_owned(),
                cargo_home.to_string_lossy().into_owned(),
            ),
            ("CARGO_NET_OFFLINE".to_owned(), "true".to_owned()),
        ])
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn materialization_copies_regular_source_and_excludes_generated_roots() {
        let source = tempdir().expect("temporary source root");
        let destination = tempdir().expect("temporary destination parent");
        fs::create_dir_all(source.path().join("nested")).expect("nested source root");
        fs::create_dir_all(source.path().join("target")).expect("generated target root");
        fs::write(source.path().join("Cargo.toml"), "[workspace]\n").expect("Cargo manifest");
        fs::write(
            source.path().join("nested/source.rs"),
            "pub fn subject() {}\n",
        )
        .expect("source file");
        fs::write(source.path().join("target/ignored"), "generated").expect("generated file");

        let workspace = destination.path().join("workspace");
        materialize_mutation_workspace(source.path(), &workspace).expect("materialize workspace");

        assert_eq!(
            fs::read_to_string(workspace.join("nested/source.rs")).expect("copied source file"),
            "pub fn subject() {}\n"
        );
        assert!(!workspace.join("target").exists());
    }

    #[test]
    fn materialization_keeps_ignored_regular_files_that_runtime_contracts_read() {
        let source = tempdir().expect("temporary source root");
        let destination = tempdir().expect("temporary destination parent");
        fs::write(source.path().join(".gitignore"), "runtime.conf\n").expect("ignore file");
        fs::write(
            source.path().join("runtime.conf"),
            "local runtime setting\n",
        )
        .expect("runtime file");

        let workspace = destination.path().join("workspace");
        materialize_mutation_workspace(source.path(), &workspace).expect("materialize workspace");

        assert_eq!(
            fs::read_to_string(workspace.join("runtime.conf")).expect("copied runtime file"),
            "local runtime setting\n"
        );
    }

    #[test]
    fn worker_environment_absolutizes_workspace_local_cargo_roots_and_budgets_build_jobs() {
        let workspace = tempdir().expect("temporary workspace");
        let workspace_root = workspace.path().join("workspace");
        let shared_cargo_home = workspace.path().join("shared-cargo-home");
        let output_root = workspace.path().join("output");
        let explicit_build = workspace.path().join("explicit-build");
        fs::create_dir_all(&workspace_root).expect("create workspace root");
        let worker = LocalMutationWorker {
            index: 0,
            selector: "0/1".to_owned(),
            expected_names: BTreeSet::new(),
            workspace,
            cargo_home: shared_cargo_home,
            source_fingerprint: SourceTreeFingerprint::from_workspace(&workspace_root)
                .expect("source fingerprint"),
            output_root,
            command: CommandSpec::new(
                "cargo",
                ["mutants"],
                crate::CommandStdout::Quiet,
                crate::CommandToolchainEnv::Inherit,
            )
            .with_env("CARGO_TARGET_DIR", ".htmlcut-mutant-cargo/target")
            .with_env(
                "CARGO_BUILD_BUILD_DIR",
                explicit_build.to_string_lossy().into_owned(),
            )
            .with_env("RUSTUP_TOOLCHAIN", MAINTAINED_NIGHTLY_TOOLCHAIN_NAME),
        };

        let cargo_home = worker.workspace.path().join("worker-cargo-home");
        let environment = worker_environment(&worker, 3, &cargo_home);
        assert_eq!(
            environment.get("CARGO_TARGET_DIR"),
            Some(
                &worker
                    .workspace
                    .path()
                    .join("workspace/.htmlcut-mutant-cargo/target")
                    .to_string_lossy()
                    .into_owned()
            )
        );
        assert_eq!(
            environment.get("CARGO_BUILD_BUILD_DIR"),
            Some(&explicit_build.to_string_lossy().into_owned())
        );
        assert_eq!(
            environment.get("RUSTUP_TOOLCHAIN"),
            Some(&MAINTAINED_NIGHTLY_TOOLCHAIN_NAME.to_owned())
        );
        assert_eq!(environment.get("CARGO_BUILD_JOBS"), Some(&"3".to_owned()));
        assert_eq!(
            environment.get("CARGO_HOME"),
            Some(&cargo_home.to_string_lossy().into_owned())
        );
        assert_eq!(
            environment.get("CARGO_NET_OFFLINE"),
            Some(&"true".to_owned())
        );
    }

    #[test]
    fn workers_do_not_inherit_an_outer_gnu_make_jobserver() {
        let mut command = Command::new("cargo");
        for name in INHERITED_JOBSERVER_ENVIRONMENT {
            command.env(name, "stale-jobserver");
        }
        clear_inherited_jobserver(&mut command);

        for name in INHERITED_JOBSERVER_ENVIRONMENT {
            assert!(
                command
                    .get_envs()
                    .any(|(key, value)| key.to_str() == Some(name) && value.is_none()),
                "worker command must explicitly remove inherited {name}"
            );
        }
    }

    #[test]
    fn worker_build_job_budget_never_oversubscribes_the_available_cores() {
        assert_eq!(bounded_cargo_build_jobs(1, 8).get(), 1);
        assert_eq!(bounded_cargo_build_jobs(10, 8).get(), 1);
        assert_eq!(bounded_cargo_build_jobs(10, 3).get(), 3);
        assert_eq!(bounded_cargo_build_jobs(10, 1).get(), 10);
    }

    #[cfg(unix)]
    #[test]
    fn worker_runner_preserves_one_success_status_per_valid_worker() {
        let worker_root = tempdir().expect("worker root");
        let workspace_root = worker_root.path().join("workspace");
        let output_root = worker_root.path().join("output");
        fs::create_dir_all(&workspace_root).expect("create workspace");
        fs::create_dir_all(&output_root).expect("create output root");
        let worker = LocalMutationWorker {
            index: 0,
            selector: "test-worker".to_owned(),
            expected_names: BTreeSet::new(),
            workspace: worker_root,
            cargo_home: PathBuf::from("/tmp/htmlcut-worker-cargo-home"),
            source_fingerprint: SourceTreeFingerprint::from_workspace(&workspace_root)
                .expect("source fingerprint"),
            output_root,
            command: CommandSpec::new(
                "sh",
                ["-c", "exit 0"],
                crate::CommandStdout::Quiet,
                crate::CommandToolchainEnv::Inherit,
            ),
        };

        assert!(
            run_worker_with(&worker, 1, |command, _output_root| {
                command.status().map_err(Into::into)
            })
            .expect("worker execution result")
            .success()
        );
    }

    #[cfg(unix)]
    #[test]
    fn worker_supervisor_returns_a_completed_child_status_after_live_pruning() {
        let output_root = tempdir().expect("output root");
        let headroom_marker = output_root.path().join("headroom-checked");
        let mut command = Command::new("sh");
        command
            .arg("-c")
            .arg("count=0; while [ \"$count\" -lt 100 ]; do [ -f \"$1\" ] && exit 0; count=$((count + 1)); sleep 0.01; done; exit 1")
            .arg("sh")
            .arg(&headroom_marker);
        let prunes = std::cell::Cell::new(0_usize);
        let headroom_checks = std::cell::Cell::new(0_usize);
        let status = supervise_worker_with(
            &mut command,
            output_root.path(),
            |_| {
                prunes.set(prunes.get() + 1);
                Ok(())
            },
            || {
                headroom_checks.set(headroom_checks.get() + 1);
                fs::write(&headroom_marker, "ready")?;
                Ok(())
            },
            Duration::ZERO,
        )
        .expect("completed child status");
        assert!(status.success());
        assert!(prunes.get() >= 1);
        assert!(headroom_checks.get() >= 1);
    }

    #[test]
    fn staging_a_workspace_shard_preserves_its_selector_and_rejects_invalid_artifact_layouts() {
        let repository = tempdir().expect("temporary repository");
        fs::write(repository.path().join("Cargo.toml"), "[workspace]\n")
            .expect("write workspace manifest");
        let output_root = tempdir().expect("temporary output root");
        let partition = LocalMutationPartition {
            label: "workspace-shard:0/1".to_owned(),
            kind: LocalMutationPartitionKind::WorkspaceShard("0/1".to_owned()),
            expected_names: BTreeSet::new(),
        };
        let mut worker = stage_worker(repository.path(), output_root.path(), 7, &partition, None)
            .expect("stage workspace shard");
        let workspace_path = worker.workspace.path().to_path_buf();
        assert_eq!(worker.index, 7);
        assert!(
            worker
                .command
                .args
                .windows(2)
                .any(|args| args == ["--shard", "0/1"])
        );

        let second_partition = LocalMutationPartition {
            label: "workspace-shard:1/2".to_owned(),
            kind: LocalMutationPartitionKind::WorkspaceShard("1/2".to_owned()),
            expected_names: BTreeSet::from(["later-mutant".to_owned()]),
        };
        worker
            .retarget(output_root.path(), 8, &second_partition, None)
            .expect("reuse staged workspace for later partition");
        assert_eq!(worker.workspace.path(), workspace_path);
        assert_eq!(worker.index, 8);
        assert_eq!(worker.selector, "workspace-shard:1/2");
        assert_eq!(
            worker.expected_names,
            BTreeSet::from(["later-mutant".to_owned()])
        );
        assert_eq!(worker.output_root, output_root.path().join("8"));
        assert!(
            worker
                .command
                .args
                .windows(2)
                .any(|args| args == ["--shard", "1/2"])
        );

        let invalid = LocalMutationWorker {
            command: worker
                .command
                .clone()
                .with_artifact_layout(CommandArtifactLayout::ManagedWorkspace),
            ..worker
        };
        assert!(run_worker(&invalid, 1).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn worker_refuses_to_reuse_a_workspace_when_a_completed_partition_changes_source() {
        let worker_root = tempdir().expect("worker root");
        let workspace_root = worker_root.path().join("workspace");
        let source_file = workspace_root.join("src/lib.rs");
        let output_root = worker_root.path().join("output");
        fs::create_dir_all(source_file.parent().expect("source parent"))
            .expect("create source directory");
        fs::create_dir_all(&output_root).expect("create output root");
        fs::write(&source_file, "pub fn original() {}\n").expect("write source");
        let worker = LocalMutationWorker {
            index: 0,
            selector: "mutation".to_owned(),
            expected_names: BTreeSet::new(),
            source_fingerprint: SourceTreeFingerprint::from_workspace(&workspace_root)
                .expect("source fingerprint"),
            workspace: worker_root,
            cargo_home: PathBuf::from("/tmp/htmlcut-worker-cargo-home"),
            output_root,
            command: CommandSpec::new(
                "sh",
                ["-c", "printf 'pub fn mutated() {}\\n' > src/lib.rs"],
                crate::CommandStdout::Quiet,
                crate::CommandToolchainEnv::Inherit,
            ),
        };

        let error = run_worker_with(&worker, 1, |command, _output_root| {
            command.status().map_err(Into::into)
        })
        .expect_err("changed source must reject workspace reuse");
        assert!(
            error
                .to_string()
                .contains("did not restore disposable source workspace")
        );
    }

    #[cfg(unix)]
    #[test]
    fn worker_result_combination_preserves_every_process_and_restoration_failure_shape() {
        let success = Command::new("sh")
            .args(["-c", "exit 0"])
            .status()
            .expect("successful status");
        let spawn_error = || std::io::Error::new(std::io::ErrorKind::NotFound, "missing worker");
        let restore_error = || Err("source restore failed".into());

        assert!(combine_worker_execution(Ok(success), Ok(())).is_ok());
        assert!(combine_worker_execution(Err(spawn_error().into()), Ok(())).is_err());
        assert!(combine_worker_execution(Ok(success), restore_error()).is_err());
        let combined = combine_worker_execution(Err(spawn_error().into()), restore_error())
            .expect_err("combined worker failure");
        assert!(combined.to_string().contains("could not run its command"));
        assert!(combined.to_string().contains("could not restore"));
    }

    #[cfg(unix)]
    #[test]
    fn materialization_rejects_symlinked_source_files() {
        let repository = tempdir().expect("temporary repository");
        let destination = tempdir().expect("temporary destination");
        let source_file = repository.path().join("source.rs");
        fs::write(&source_file, "pub fn source() {}\n").expect("write source");
        std::os::unix::fs::symlink(&source_file, repository.path().join("linked.rs"))
            .expect("create source symlink");

        assert!(
            materialize_mutation_workspace(
                repository.path(),
                &destination.path().join("workspace")
            )
            .is_err()
        );
    }
}
