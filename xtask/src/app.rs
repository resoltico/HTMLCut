use std::ffi::OsString;
use std::fs;
use std::path::Path;

use clap::{CommandFactory, FromArgMatches, Parser, Subcommand};
use htmlcut_tempdir::tempdir;

use crate::fuzz::FUZZ_SMOKE_EXAMPLE_TARGET;
use crate::{
    CommandArtifactLayout, CommandSpec, CommandStdout, CommandToolchainEnv, CoverageFailure,
    DEFAULT_FUZZ_SMOKE_RUNS, DynResult, HygieneCleanMode, HygieneReportFormat,
    assert_known_fuzz_target, check_plan, ci_rust_gate_plan, clean_hygiene, coverage_clean_command,
    coverage_command, coverage_output_path, ensure_coverage_output_dir,
    ensure_coverage_prerequisites, ensure_fuzz_smoke_prerequisites, ensure_hygiene,
    ensure_miri_prerequisites, ensure_repo_toolchain_prerequisites, evaluate_coverage_report,
    fuzz_smoke_command, fuzz_smoke_targets, hygiene_report, is_semver_check_spec,
    miri_selector_command, prepare_artifact_layout, read_coverage_report, remove_dir_if_exists,
    render_hygiene_report, run_outdated_check, run_spec, semver_scratch_dir, stage_fuzz_corpus,
    strip_dev_dependency_tables, tracked_files, with_workspace_stub, workspace_version,
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
    Check,
    #[command(
        about = "Run the curated cross-platform Rust CI gate.",
        long_about = "Run the maintained cross-platform Rust CI gate through cargo xtask so GitHub Actions does not duplicate command ownership."
    )]
    CiRustGate,
    #[command(
        about = "Run only the maintained htmlcut-core semver gate.",
        long_about = "Run only the maintained htmlcut-core cargo-semver-checks gate with the same baseline and release-type policy used by cargo xtask check."
    )]
    SemverCheck,
    #[command(
        about = "Run only the curated 100% coverage gate.",
        long_about = "Run the curated 100% line-and-branch coverage gate plus its prerequisite checks without rerunning the broader maintainer gate."
    )]
    Coverage,
    #[command(
        about = "Run the maintained strict-provenance selector-safety Miri proof.",
        long_about = "Run the maintained strict-provenance selector-safety Miri proof against htmlcut-core's selector validation and execution path."
    )]
    Miri,
    #[command(
        about = "Run the maintained dependency-freshness gate.",
        long_about = "Run the maintained dependency-freshness gate through a sanitized workspace snapshot so local path patches do not break the outdated check."
    )]
    OutdatedCheck,
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
    Verify,
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

/// Parses explicit `cargo xtask` arguments and runs the requested maintainer workflow.
pub(crate) fn main_entry_with<I, T>(repo_root: &Path, args: I) -> DynResult<()>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    run_task(repo_root, parse_task(args)?)
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
        "Examples:\n  cargo xtask check\n  cargo xtask ci-rust-gate\n  cargo xtask semver-check\n  cargo xtask coverage\n  cargo xtask miri\n  cargo xtask outdated-check\n  cargo xtask fuzz-smoke --target {FUZZ_SMOKE_EXAMPLE_TARGET}\n  cargo xtask hygiene report\n  cargo xtask hygiene clean --mode rebuildable\n  cargo xtask refresh-semver-baseline --git-ref v7.0.0"
    )
}

fn run_task(repo_root: &Path, task: Task) -> DynResult<()> {
    match task {
        Task::Check => run_check(repo_root),
        Task::CiRustGate => run_ci_rust_gate(repo_root),
        Task::SemverCheck => run_semver_check(repo_root),
        Task::Coverage => run_coverage(repo_root),
        Task::Miri => run_miri(repo_root),
        Task::OutdatedCheck => run_outdated_check(repo_root),
        Task::FuzzSmoke { target, runs } => run_fuzz_smoke(repo_root, target.as_deref(), runs),
        Task::RefreshSemverBaseline { git_ref } => refresh_semver_baseline(repo_root, &git_ref),
        Task::Hygiene { command } => run_hygiene(repo_root, command),
    }
}

fn run_check(repo_root: &Path) -> DynResult<()> {
    ensure_repo_toolchain_prerequisites(repo_root)?;
    ensure_miri_prerequisites(repo_root)?;
    ensure_coverage_prerequisites(repo_root)?;
    clean_hygiene(repo_root, HygieneCleanMode::Safe)?;
    prepare_artifact_layout(repo_root, CommandArtifactLayout::ManagedWorkspace)?;
    ensure_hygiene(repo_root)?;
    println!("==> Rust gate");

    for spec in check_plan(repo_root)? {
        if is_semver_check_spec(&spec) {
            remove_dir_if_exists(&semver_scratch_dir(repo_root))?;
            let result = run_spec(repo_root, &spec);
            let cleanup = remove_dir_if_exists(&semver_scratch_dir(repo_root));
            result?;
            cleanup?;
            continue;
        }

        run_spec(repo_root, &spec)?;
    }

    run_coverage(repo_root)?;
    clean_hygiene(repo_root, HygieneCleanMode::Safe)?;
    ensure_hygiene(repo_root)
}

fn run_miri(repo_root: &Path) -> DynResult<()> {
    ensure_repo_toolchain_prerequisites(repo_root)?;
    ensure_miri_prerequisites(repo_root)?;
    clean_hygiene(repo_root, HygieneCleanMode::Safe)?;
    prepare_artifact_layout(repo_root, CommandArtifactLayout::ManagedWorkspace)?;
    ensure_hygiene(repo_root)?;
    println!("==> Strict-provenance selector-safety Miri proof");
    run_spec(repo_root, &miri_selector_command())?;
    clean_hygiene(repo_root, HygieneCleanMode::Safe)?;
    ensure_hygiene(repo_root)
}

fn run_ci_rust_gate(repo_root: &Path) -> DynResult<()> {
    ensure_repo_toolchain_prerequisites(repo_root)?;
    clean_hygiene(repo_root, HygieneCleanMode::Safe)?;
    prepare_artifact_layout(repo_root, CommandArtifactLayout::ManagedWorkspace)?;
    ensure_hygiene(repo_root)?;
    println!("==> Cross-platform Rust gate");

    for spec in ci_rust_gate_plan(repo_root)? {
        if is_semver_check_spec(&spec) {
            remove_dir_if_exists(&semver_scratch_dir(repo_root))?;
            let result = run_spec(repo_root, &spec);
            let cleanup = remove_dir_if_exists(&semver_scratch_dir(repo_root));
            result?;
            cleanup?;
            continue;
        }

        run_spec(repo_root, &spec)?;
    }

    clean_hygiene(repo_root, HygieneCleanMode::Safe)?;
    ensure_hygiene(repo_root)
}

fn run_semver_check(repo_root: &Path) -> DynResult<()> {
    ensure_repo_toolchain_prerequisites(repo_root)?;
    clean_hygiene(repo_root, HygieneCleanMode::Safe)?;
    prepare_artifact_layout(repo_root, CommandArtifactLayout::ManagedWorkspace)?;
    ensure_hygiene(repo_root)?;
    let spec = semver_check_spec(check_plan(repo_root)?)?;

    remove_dir_if_exists(&semver_scratch_dir(repo_root))?;
    let result = run_spec(repo_root, &spec);
    let cleanup = remove_dir_if_exists(&semver_scratch_dir(repo_root));
    result?;
    cleanup?;
    clean_hygiene(repo_root, HygieneCleanMode::Safe)?;
    ensure_hygiene(repo_root)
}

fn semver_check_spec(plan: Vec<CommandSpec>) -> DynResult<CommandSpec> {
    plan.into_iter()
        .find(is_semver_check_spec)
        .ok_or_else(|| "semver gate step is missing from cargo xtask check".into())
}

fn run_coverage(repo_root: &Path) -> DynResult<()> {
    ensure_coverage_prerequisites(repo_root)?;
    clean_hygiene(repo_root, HygieneCleanMode::Safe)?;
    prepare_artifact_layout(repo_root, CommandArtifactLayout::ManagedCoverage)?;
    ensure_hygiene(repo_root)?;
    let coverage_clean_spec = coverage_clean_command();
    let coverage_spec = coverage_command(repo_root);
    run_spec(repo_root, &coverage_clean_spec)?;
    ensure_coverage_output_dir(repo_root)?;

    let result = (|| -> DynResult<()> {
        run_spec(repo_root, &coverage_spec)?;

        let tracked = tracked_files(repo_root)?;
        let report = read_coverage_report(&coverage_output_path(repo_root))?;
        let summary = evaluate_coverage_report(repo_root, &tracked, report)?;

        if !summary.failures.is_empty() {
            eprintln!("Rust coverage gate failed.");
            for failure in &summary.failures {
                report_coverage_failure(failure);
            }
            return Err("coverage gate failed".into());
        }

        println!(
            "Rust coverage: lines 100.00% ({0}/{0}) | branches 100.00% ({1}/{1})",
            summary.tracked_line_count, summary.tracked_branch_count
        );
        Ok(())
    })();

    let cleanup = run_spec(repo_root, &coverage_clean_spec);
    result?;
    cleanup?;
    clean_hygiene(repo_root, HygieneCleanMode::Safe)?;
    ensure_hygiene(repo_root)
}

fn run_fuzz_smoke(repo_root: &Path, target: Option<&str>, runs: u32) -> DynResult<()> {
    if let Some(target) = target {
        assert_known_fuzz_target(target)?;
    }

    ensure_fuzz_smoke_prerequisites(repo_root)?;
    clean_hygiene(repo_root, HygieneCleanMode::Safe)?;
    prepare_artifact_layout(repo_root, CommandArtifactLayout::ManagedWorkspace)?;
    ensure_hygiene(repo_root)?;

    let targets = target
        .map(|target| vec![target])
        .unwrap_or_else(|| fuzz_smoke_targets().to_vec());

    for target in targets {
        println!("==> Fuzz smoke: {target}");
        let scratch_root = tempdir()?;
        let staged_corpus = stage_fuzz_corpus(repo_root, scratch_root.path(), target)?;
        let fuzz_spec = fuzz_smoke_command(target, &staged_corpus, runs)?;
        run_spec(repo_root, &fuzz_spec)?;
    }

    clean_hygiene(repo_root, HygieneCleanMode::Safe)?;
    ensure_hygiene(repo_root)
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
        HygieneTask::Verify => ensure_hygiene(repo_root),
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

fn refresh_semver_baseline(repo_root: &Path, git_ref: &str) -> DynResult<()> {
    let snapshot = tempdir()?;
    let snapshot_root = snapshot.path().join("snapshot");
    let snapshot_archive = snapshot.path().join("snapshot.tar");
    fs::create_dir_all(&snapshot_root)?;

    let snapshot_archive_spec = snapshot_archive_command(&snapshot_archive, git_ref);
    run_spec(repo_root, &snapshot_archive_spec)?;

    let unpack_snapshot_spec = unpack_snapshot_command(&snapshot_archive, &snapshot_root);
    run_spec(repo_root, &unpack_snapshot_spec)?;

    let version = workspace_version(&snapshot_root)?;
    let baseline_parent = repo_root.join("semver-baseline");
    let baseline_dir = baseline_parent.join("htmlcut-core");
    let extracted_dir = baseline_parent.join(format!("htmlcut-core-{version}"));
    let snapshot_manifest = snapshot_root
        .join("crates")
        .join("htmlcut-core")
        .join("Cargo.toml");
    let archive = snapshot_root
        .join("target")
        .join("package")
        .join(format!("htmlcut-core-{version}.crate"));

    let snapshot_cargo_toml = fs::read_to_string(&snapshot_manifest)?;
    let stripped_snapshot_cargo_toml = strip_dev_dependency_tables(&snapshot_cargo_toml);
    fs::write(&snapshot_manifest, stripped_snapshot_cargo_toml)?;

    let package_snapshot_spec = package_snapshot_command();
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

    let baseline_manifest = extracted_dir.join("Cargo.toml");
    let cargo_toml = fs::read_to_string(&baseline_manifest)?;
    fs::write(&baseline_manifest, with_workspace_stub(&cargo_toml))?;
    fs::rename(extracted_dir, baseline_dir)?;
    let provenance_path = baseline_parent.join("htmlcut-core").join("BASELINE.toml");
    let provenance = semver_baseline_provenance(git_ref, &version);
    fs::write(provenance_path, provenance)?;
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

fn report_coverage_failure(failure: &CoverageFailure) {
    if !failure.uncovered_lines.is_empty() {
        eprintln!(
            "- {} lines: {}",
            failure.file,
            failure.uncovered_lines.join(", ")
        );
    }

    if failure.uncovered_branch_count == 0 {
        return;
    }

    eprintln!(
        "- {} branches: {} uncovered",
        failure.file, failure.uncovered_branch_count
    );
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

fn package_snapshot_command() -> CommandSpec {
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
    semver_check_spec(plan)
}

#[cfg(test)]
pub(crate) fn run_coverage_for_tests(repo_root: &Path) -> DynResult<()> {
    run_coverage(repo_root)
}

#[cfg(test)]
pub(crate) fn refresh_semver_baseline_for_tests(repo_root: &Path, git_ref: &str) -> DynResult<()> {
    refresh_semver_baseline(repo_root, git_ref)
}
