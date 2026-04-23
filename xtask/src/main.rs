use std::fs;
use std::path::Path;

use clap::{Parser, Subcommand};
use htmlcut_tempdir::tempdir;
use xtask::{
    CommandSpec, DEFAULT_FUZZ_SMOKE_RUNS, DynResult, assert_known_fuzz_target, check_plan,
    coverage_clean_command, coverage_command, coverage_output_path, ensure_coverage_output_dir,
    ensure_coverage_prerequisites, ensure_fuzz_smoke_prerequisites,
    ensure_repo_toolchain_prerequisites, evaluate_coverage_report, fuzz_smoke_command,
    fuzz_smoke_targets, is_semver_check_spec, read_coverage_report, remove_dir_if_exists,
    repo_root, run_spec, semver_scratch_dir, stage_fuzz_corpus, strip_dev_dependency_tables,
    tracked_files, with_workspace_stub, workspace_version,
};

#[derive(Parser)]
#[command(name = "xtask", about = "Rust-native maintenance tasks for HTMLCut.")]
struct Cli {
    #[command(subcommand)]
    command: Task,
}

#[derive(Subcommand)]
enum Task {
    Check,
    Coverage,
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
    RefreshSemverBaseline {
        /// Git tag, branch, or commit that represents the published baseline.
        #[arg(long = "git-ref", value_name = "REF")]
        git_ref: String,
    },
}

fn main() -> DynResult<()> {
    let cli = Cli::parse();
    let repo_root = repo_root();

    match cli.command {
        Task::Check => run_check(&repo_root),
        Task::Coverage => run_coverage(&repo_root),
        Task::FuzzSmoke { target, runs } => run_fuzz_smoke(&repo_root, target.as_deref(), runs),
        Task::RefreshSemverBaseline { git_ref } => refresh_semver_baseline(&repo_root, &git_ref),
    }
}

fn run_check(repo_root: &Path) -> DynResult<()> {
    ensure_repo_toolchain_prerequisites(repo_root)?;
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

    run_coverage(repo_root)
}

fn run_coverage(repo_root: &Path) -> DynResult<()> {
    ensure_coverage_prerequisites(repo_root)?;
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
            for failure in summary.failures {
                if !failure.uncovered_lines.is_empty() {
                    eprintln!(
                        "- {} lines: {}",
                        failure.file,
                        failure.uncovered_lines.join(", ")
                    );
                }
                if failure.uncovered_branch_count > 0 {
                    eprintln!(
                        "- {} branches: {} uncovered",
                        failure.file, failure.uncovered_branch_count
                    );
                }
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
    cleanup
}

fn run_fuzz_smoke(repo_root: &Path, target: Option<&str>, runs: u32) -> DynResult<()> {
    ensure_fuzz_smoke_prerequisites(repo_root)?;

    let targets = if let Some(target) = target {
        assert_known_fuzz_target(target)?;
        vec![target]
    } else {
        fuzz_smoke_targets().to_vec()
    };

    for target in targets {
        println!("==> Fuzz smoke: {target}");
        let scratch_root = tempdir()?;
        let staged_corpus = stage_fuzz_corpus(repo_root, scratch_root.path(), target)?;
        let fuzz_spec = fuzz_smoke_command(target, &staged_corpus, runs)?;
        run_spec(repo_root, &fuzz_spec)?;
    }

    Ok(())
}

fn refresh_semver_baseline(repo_root: &Path, git_ref: &str) -> DynResult<()> {
    let snapshot = tempdir()?;
    let snapshot_root = snapshot.path().join("snapshot");
    let snapshot_archive = snapshot.path().join("snapshot.tar");
    fs::create_dir_all(&snapshot_root)?;

    run_spec(
        repo_root,
        &CommandSpec::new(
            "git",
            [
                "archive",
                "--format=tar",
                "--output",
                snapshot_archive.to_string_lossy().as_ref(),
                git_ref,
            ],
            false,
            false,
        ),
    )?;

    run_spec(
        repo_root,
        &CommandSpec::new(
            "tar",
            [
                "-xf",
                snapshot_archive.to_string_lossy().as_ref(),
                "-C",
                snapshot_root.to_string_lossy().as_ref(),
            ],
            false,
            false,
        ),
    )?;

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
    fs::write(
        &snapshot_manifest,
        strip_dev_dependency_tables(&snapshot_cargo_toml),
    )?;

    run_spec(
        &snapshot_root,
        &CommandSpec::new(
            "cargo",
            [
                "package",
                "--allow-dirty",
                "--no-verify",
                "-p",
                "htmlcut-core",
            ],
            false,
            true,
        ),
    )?;

    if baseline_dir.exists() {
        fs::remove_dir_all(&baseline_dir)?;
    }
    if extracted_dir.exists() {
        fs::remove_dir_all(&extracted_dir)?;
    }
    fs::create_dir_all(&baseline_parent)?;

    run_spec(
        repo_root,
        &CommandSpec::new(
            "tar",
            [
                "-xzf",
                archive.to_string_lossy().as_ref(),
                "-C",
                baseline_parent.to_string_lossy().as_ref(),
            ],
            false,
            false,
        ),
    )?;

    let baseline_manifest = extracted_dir.join("Cargo.toml");
    let cargo_toml = fs::read_to_string(&baseline_manifest)?;
    fs::write(&baseline_manifest, with_workspace_stub(&cargo_toml))?;
    fs::rename(extracted_dir, baseline_dir)?;
    Ok(())
}
