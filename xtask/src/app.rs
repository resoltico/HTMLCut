use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::Path;

use clap::{CommandFactory, FromArgMatches, Parser, Subcommand};
use htmlcut_tempdir::tempdir;

use crate::fuzz::FUZZ_SMOKE_EXAMPLE_TARGET;
use crate::gate_report::{GateOutputOptions, with_gate_report};
use crate::plan::INERT_BASELINE_MANIFEST_NAME;
use crate::{
    CommandSpec, CommandStdout, CommandToolchainEnv, DEFAULT_FUZZ_SMOKE_RUNS, DynResult,
    HygieneCleanMode, HygieneReportFormat, check_source_structure, clean_hygiene, ensure_hygiene,
    hygiene_report, render_hygiene_report, report_source_structure,
    restore_vendored_dependency_paths_in_baseline_manifest, run_outdated_check, run_spec,
    sanitize_snapshot_workspace_manifest_for_packaging, snapshot_uses_vendored_selector_stack,
    strip_dev_dependency_tables, with_workspace_stub, workspace_version,
};

mod gates;

const VENDORED_SELECTOR_STACK_DIRECTORIES: &[&str] = &[
    "html5ever",
    "markup5ever",
    "scraper",
    "selectors",
    "servo_arc",
    "tendril",
];

use self::gates::{
    run_check, run_ci_rust_gate, run_coverage, run_fuzz_smoke, run_miri, run_semver_check,
};

#[derive(Parser)]
#[command(name = "xtask", about = "Rust-native maintenance tasks for HTMLCut.")]
struct Cli {
    #[command(subcommand)]
    command: Task,
}

#[derive(Subcommand)]
enum Task {
    #[command(
        about = "Run the full maintainer quality gate.",
        long_about = "Run the full maintainer quality gate, including formatting, docs, dependency policy, tests, and the final curated 100% coverage pass."
    )]
    Check {
        #[command(flatten)]
        output: GateOutputOptions,
    },
    #[command(
        about = "Run the curated cross-platform Rust CI gate.",
        long_about = "Run the maintained cross-platform Rust CI gate through cargo xtask so GitHub Actions does not duplicate command ownership."
    )]
    CiRustGate {
        #[command(flatten)]
        output: GateOutputOptions,
    },
    #[command(
        about = "Run only the maintained htmlcut-core semver gate.",
        long_about = "Run only the maintained htmlcut-core cargo-semver-checks gate with the same baseline and release-type policy used by cargo xtask check."
    )]
    SemverCheck {
        #[command(flatten)]
        output: GateOutputOptions,
    },
    #[command(
        about = "Run only the curated 100% coverage gate.",
        long_about = "Run the curated 100% line-and-branch coverage gate plus its prerequisite checks without rerunning the broader maintainer gate."
    )]
    Coverage {
        #[command(flatten)]
        output: GateOutputOptions,
    },
    #[command(
        about = "Run the maintained strict-provenance selector-and-slice Miri proof.",
        long_about = "Run the maintained strict-provenance selector-and-slice Miri proof against htmlcut-core's selector validation plus delimiter slice execution path."
    )]
    Miri {
        #[command(flatten)]
        output: GateOutputOptions,
    },
    #[command(
        about = "Run the maintained dependency-freshness gate.",
        long_about = "Run the maintained dependency-freshness gate through a sanitized workspace snapshot so local path patches do not break the outdated check."
    )]
    OutdatedCheck {
        #[command(flatten)]
        output: GateOutputOptions,
    },
    #[command(
        about = "Run a short maintained libFuzzer smoke pass.",
        long_about = "Run a short maintained libFuzzer smoke pass over one target or the whole maintained target inventory."
    )]
    FuzzSmoke {
        /// One maintained fuzz target to run. Omit to run the full smoke inventory.
        #[arg(long = "target", value_name = "TARGET")]
        target: Option<String>,
        /// libFuzzer iteration budget for each smoke run.
        #[arg(
            long = "runs",
            default_value_t = DEFAULT_FUZZ_SMOKE_RUNS,
            value_name = "COUNT",
            value_parser = clap::value_parser!(u32).range(1..)
        )]
        runs: u32,
        #[command(flatten)]
        output: GateOutputOptions,
    },
    #[command(
        about = "Refresh the checked-in htmlcut-core semver baseline.",
        long_about = "Refresh the checked-in htmlcut-core semver baseline from one published Git tag, branch, or commit."
    )]
    RefreshSemverBaseline {
        /// Git tag, branch, or commit that represents the published baseline.
        #[arg(long = "git-ref", value_name = "REF")]
        git_ref: String,
    },
    #[command(
        about = "Inspect or repair the repository artifact hygiene policy.",
        long_about = "Inspect or repair the maintained repository artifact hygiene policy, including managed Cargo artifact roots, repo-local scratch, and accidental legacy target trees."
    )]
    Hygiene {
        #[command(subcommand)]
        command: HygieneTask,
    },
    #[command(
        about = "Inspect or enforce the first-party Rust source-structure contract.",
        long_about = "Measure or fail-closed enforce role ownership, cohesion budgets, and internal dependency boundaries for maintained first-party Rust code."
    )]
    Structure {
        #[command(subcommand)]
        command: StructureTask,
    },
}

#[derive(Subcommand)]
enum HygieneTask {
    #[command(
        about = "Report the current artifact inventory.",
        long_about = "Render the current repository artifact inventory, including managed Cargo caches, repo-local scratch, budgets, and policy violations."
    )]
    Report {
        /// Output format for the report.
        #[arg(long = "format", value_enum, default_value_t = HygieneReportFormat::Text)]
        format: HygieneReportFormat,
    },
    #[command(
        about = "Fail if the current artifact inventory violates policy.",
        long_about = "Fail if the current repository artifact inventory violates the maintained hygiene policy."
    )]
    Verify {
        #[command(flatten)]
        output: GateOutputOptions,
    },
    #[command(
        about = "Delete disposable artifact roots.",
        long_about = "Delete disposable artifact roots. `safe` removes repo-local temporary workspace state, legacy repo-local target trees, and other disposable scratch; `rebuildable` also deletes the managed Cargo caches."
    )]
    Clean {
        /// Cleanup profile.
        #[arg(long = "mode", value_enum, default_value_t = HygieneCleanMode::Safe)]
        mode: HygieneCleanMode,
    },
}

#[derive(Subcommand)]
enum StructureTask {
    #[command(
        about = "Fail if maintained first-party Rust source has no role, exceeds a budget, or crosses a dependency boundary."
    )]
    Check {
        #[command(flatten)]
        output: GateOutputOptions,
    },
    #[command(
        about = "Print measured source shape and resolved ownership for maintained first-party Rust source."
    )]
    Report,
}

/// Parses explicit `cargo xtask` arguments and runs the requested maintainer workflow.
pub(crate) fn main_entry_with<I, T>(repo_root: &Path, args: I) -> DynResult<()>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let task = parse_task(args)?;
    if let Some((gate, output)) = task.gate_context() {
        with_gate_report(repo_root, gate, output, || run_task(repo_root, task))
    } else {
        run_task(repo_root, task)
    }
}

fn parse_task<I, T>(args: I) -> Result<Task, clap::Error>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let matches = cli_command().try_get_matches_from(args)?;
    Ok(Cli::from_arg_matches(&matches)?.command)
}

fn cli_command() -> clap::Command {
    Cli::command().after_help(xtask_after_help())
}

fn xtask_after_help() -> String {
    format!(
        "Examples:\n  cargo xtask check\n  cargo xtask check --format json\n  cargo xtask ci-rust-gate\n  cargo xtask semver-check\n  cargo xtask coverage\n  cargo xtask miri\n  cargo xtask outdated-check\n  cargo xtask fuzz-smoke --target {FUZZ_SMOKE_EXAMPLE_TARGET}\n  cargo xtask hygiene report\n  cargo xtask hygiene clean --mode rebuildable\n  cargo xtask structure report\n  cargo xtask structure check\n  cargo xtask refresh-semver-baseline --git-ref v7.0.0"
    )
}

fn run_task(repo_root: &Path, task: Task) -> DynResult<()> {
    match task {
        Task::Check { .. } => run_check(repo_root),
        Task::CiRustGate { .. } => run_ci_rust_gate(repo_root),
        Task::SemverCheck { .. } => run_semver_check(repo_root),
        Task::Coverage { .. } => run_coverage(repo_root),
        Task::Miri { .. } => run_miri(repo_root),
        Task::OutdatedCheck { .. } => run_outdated_check(repo_root),
        Task::FuzzSmoke { target, runs, .. } => run_fuzz_smoke(repo_root, target.as_deref(), runs),
        Task::RefreshSemverBaseline { git_ref } => refresh_semver_baseline(repo_root, &git_ref),
        Task::Hygiene { command } => run_hygiene(repo_root, command),
        Task::Structure { command } => run_structure(repo_root, command),
    }
}

impl Task {
    fn gate_context(&self) -> Option<(&'static str, GateOutputOptions)> {
        match self {
            Self::Check { output } => Some(("check", *output)),
            Self::CiRustGate { output } => Some(("ci-rust-gate", *output)),
            Self::SemverCheck { output } => Some(("semver-check", *output)),
            Self::Coverage { output } => Some(("coverage", *output)),
            Self::Miri { output } => Some(("miri", *output)),
            Self::OutdatedCheck { output } => Some(("outdated-check", *output)),
            Self::FuzzSmoke { output, .. } => Some(("fuzz-smoke", *output)),
            Self::Hygiene {
                command: HygieneTask::Verify { output },
            } => Some(("hygiene-verify", *output)),
            Self::Structure {
                command: StructureTask::Check { output },
            } => Some(("structure-check", *output)),
            Self::RefreshSemverBaseline { .. }
            | Self::Hygiene {
                command: HygieneTask::Report { .. } | HygieneTask::Clean { .. },
            }
            | Self::Structure {
                command: StructureTask::Report,
            } => None,
        }
    }
}

fn run_hygiene(repo_root: &Path, command: HygieneTask) -> DynResult<()> {
    match command {
        HygieneTask::Report { format } => {
            let report = hygiene_report(repo_root)?;
            match format {
                HygieneReportFormat::Text => println!("{}", render_hygiene_report(&report)),
                HygieneReportFormat::Json => println!("{}", serde_json::to_string_pretty(&report)?),
            }
            Ok(())
        }
        HygieneTask::Verify { .. } => ensure_hygiene(repo_root),
        HygieneTask::Clean { mode } => {
            let result = clean_hygiene(repo_root, mode)?;
            println!(
                "Removed {} artifact roots and reclaimed {} bytes.",
                result.removed_paths.len(),
                result.reclaimed_bytes
            );
            for path in result.removed_paths {
                println!("- {}", path.display());
            }
            Ok(())
        }
    }
}

fn run_structure(repo_root: &Path, command: StructureTask) -> DynResult<()> {
    match command {
        StructureTask::Check { .. } => check_source_structure(repo_root),
        StructureTask::Report => report_source_structure(repo_root),
    }
}

fn refresh_semver_baseline(repo_root: &Path, git_ref: &str) -> DynResult<()> {
    let snapshot = tempdir()?;
    let snapshot_root = snapshot.path().join("snapshot");
    let snapshot_archive = snapshot.path().join("snapshot.tar");
    let package_target_root = snapshot.path().join("cargo-target");
    let package_build_root = snapshot.path().join("cargo-build");
    fs::create_dir_all(&snapshot_root)?;

    let snapshot_archive_spec = snapshot_archive_command(&snapshot_archive, git_ref);
    run_spec(repo_root, &snapshot_archive_spec)?;

    let unpack_snapshot_spec = unpack_snapshot_command(&snapshot_archive, &snapshot_root);
    run_spec(repo_root, &unpack_snapshot_spec)?;

    let version = workspace_version(&snapshot_root)?;
    let baseline_parent = repo_root.join("semver-baseline");
    let baseline_dir = baseline_parent.join("htmlcut-core");
    let extracted_dir = baseline_parent.join(format!("htmlcut-core-{version}"));
    let snapshot_workspace_manifest = snapshot_root.join("Cargo.toml");
    let snapshot_manifest = snapshot_root
        .join("crates")
        .join("htmlcut-core")
        .join("Cargo.toml");
    let archive = package_target_root
        .join("package")
        .join(format!("htmlcut-core-{version}.crate"));

    let snapshot_workspace_cargo_toml = fs::read_to_string(&snapshot_workspace_manifest)?;
    let published_vendored_stack = snapshot.path().join("published-vendored-selector-stack");
    if snapshot_uses_vendored_selector_stack(&snapshot_workspace_cargo_toml)? {
        copy_published_vendored_selector_stack(
            &snapshot_root.join("patches/rust"),
            &published_vendored_stack,
        )?;
    }
    let sanitized_workspace_manifest =
        sanitize_snapshot_workspace_manifest_for_packaging(&snapshot_workspace_cargo_toml)?;
    fs::write(&snapshot_workspace_manifest, sanitized_workspace_manifest)?;

    let snapshot_cargo_toml = fs::read_to_string(&snapshot_manifest)?;
    let stripped_snapshot_cargo_toml = strip_dev_dependency_tables(&snapshot_cargo_toml);
    fs::write(&snapshot_manifest, stripped_snapshot_cargo_toml)?;

    let package_snapshot_spec = package_snapshot_command(&package_target_root, &package_build_root);
    run_spec(&snapshot_root, &package_snapshot_spec)?;

    if baseline_dir.exists() {
        fs::remove_dir_all(&baseline_dir)?;
    }
    if extracted_dir.exists() {
        fs::remove_dir_all(&extracted_dir)?;
    }
    fs::create_dir_all(&baseline_parent)?;

    let extract_baseline_spec = extract_baseline_command(&archive, &baseline_parent);
    run_spec(repo_root, &extract_baseline_spec)?;

    fs::rename(extracted_dir, &baseline_dir)?;
    let baseline_manifest = baseline_dir.join("Cargo.toml");
    let cargo_toml = fs::read_to_string(&baseline_manifest)?;
    if let Some(restored_manifest) =
        restore_vendored_dependency_paths_in_baseline_manifest(&cargo_toml)?
    {
        fs::write(&baseline_manifest, with_workspace_stub(&restored_manifest))?;
        copy_published_vendored_selector_stack(
            &published_vendored_stack,
            &baseline_dir.join("vendor"),
        )?;
    } else {
        fs::write(&baseline_manifest, with_workspace_stub(&cargo_toml))?;
    }
    let provenance_path = baseline_parent.join("htmlcut-core").join("BASELINE.toml");
    let provenance = semver_baseline_provenance(git_ref, &version);
    fs::write(provenance_path, provenance)?;
    Ok(())
}

fn copy_published_vendored_selector_stack(
    source_stack_root: &Path,
    destination_stack_root: &Path,
) -> DynResult<()> {
    for directory in VENDORED_SELECTOR_STACK_DIRECTORIES {
        copy_directory_recursively(
            &source_stack_root.join(directory),
            &destination_stack_root.join(directory),
        )?;
    }
    Ok(())
}

fn copy_directory_recursively(source: &Path, destination: &Path) -> DynResult<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let entry_name = entry.file_name();
        let destination_name = if entry_name == OsStr::new("Cargo.toml") {
            OsStr::new(INERT_BASELINE_MANIFEST_NAME)
        } else {
            &entry_name
        };
        let destination_path = destination.join(destination_name);
        let entry_type = entry.file_type()?;
        if entry_type.is_dir() {
            copy_directory_recursively(&source_path, &destination_path)?;
        } else if entry_type.is_file() {
            fs::copy(source_path, destination_path)?;
        } else {
            return Err(format!(
                "published vendored selector-stack entry is neither a regular file nor directory: {}",
                source_path.display()
            )
            .into());
        }
    }
    Ok(())
}

fn semver_baseline_provenance(git_ref: &str, version: &str) -> String {
    format!(
        concat!(
            "schema = \"htmlcut.semver_baseline_provenance@1\"\n",
            "package = \"htmlcut-core\"\n",
            "package_version = \"{version}\"\n",
            "source_git_ref = \"{git_ref}\"\n",
            "refresh_command = \"cargo xtask refresh-semver-baseline --git-ref {git_ref}\"\n",
        ),
        git_ref = git_ref,
        version = version,
    )
}

fn snapshot_archive_command(snapshot_archive: &Path, git_ref: &str) -> CommandSpec {
    CommandSpec::new(
        "git",
        [
            "archive",
            "--format=tar",
            "--output",
            snapshot_archive.to_string_lossy().as_ref(),
            git_ref,
        ],
        CommandStdout::Inherit,
        CommandToolchainEnv::Inherit,
    )
}

fn unpack_snapshot_command(snapshot_archive: &Path, snapshot_root: &Path) -> CommandSpec {
    CommandSpec::new(
        "tar",
        [
            "-xf",
            snapshot_archive.to_string_lossy().as_ref(),
            "-C",
            snapshot_root.to_string_lossy().as_ref(),
        ],
        CommandStdout::Inherit,
        CommandToolchainEnv::Inherit,
    )
}

fn package_snapshot_command(target_root: &Path, build_root: &Path) -> CommandSpec {
    CommandSpec::new(
        "cargo",
        [
            "package",
            "--allow-dirty",
            "--no-verify",
            "-p",
            "htmlcut-core",
        ],
        CommandStdout::Inherit,
        CommandToolchainEnv::ForceClang,
    )
    .with_env("CARGO_TARGET_DIR", target_root.to_string_lossy())
    .with_env("CARGO_BUILD_BUILD_DIR", build_root.to_string_lossy())
}

fn extract_baseline_command(archive: &Path, baseline_parent: &Path) -> CommandSpec {
    CommandSpec::new(
        "tar",
        [
            "-xzf",
            archive.to_string_lossy().as_ref(),
            "-C",
            baseline_parent.to_string_lossy().as_ref(),
        ],
        CommandStdout::Inherit,
        CommandToolchainEnv::Inherit,
    )
}

#[cfg(test)]
pub(crate) fn semver_check_spec_for_tests(plan: Vec<CommandSpec>) -> DynResult<CommandSpec> {
    gates::semver_check_spec(plan)
}

#[cfg(test)]
pub(crate) fn semver_spec_with_materialized_baseline_for_tests(
    spec: CommandSpec,
    materialized_baseline: &Path,
) -> DynResult<CommandSpec> {
    gates::with_materialized_baseline(spec, materialized_baseline)
}

#[cfg(test)]
pub(crate) fn run_coverage_for_tests(repo_root: &Path) -> DynResult<()> {
    run_coverage(repo_root)
}

#[cfg(test)]
pub(crate) fn refresh_semver_baseline_for_tests(repo_root: &Path, git_ref: &str) -> DynResult<()> {
    refresh_semver_baseline(repo_root, git_ref)
}
