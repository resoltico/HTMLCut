use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use clap::{Parser, Subcommand};
use tempfile::tempdir;
use xtask::{
    CommandSpec, CoveragePreflightFailure, DynResult, check_plan, coverage_clean_command,
    coverage_command, coverage_output_path, coverage_preflight_failures,
    coverage_preflight_message, evaluate_coverage_report, read_coverage_report, tracked_files,
    with_workspace_stub, workspace_version,
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
        Task::RefreshSemverBaseline { git_ref } => refresh_semver_baseline(&repo_root, &git_ref),
    }
}

fn run_check(repo_root: &Path) -> DynResult<()> {
    println!("==> Rust gate");

    for spec in check_plan(repo_root)? {
        run_spec(repo_root, &spec)?;
    }

    run_coverage(repo_root)
}

fn run_coverage(repo_root: &Path) -> DynResult<()> {
    ensure_coverage_prerequisites(repo_root)?;
    let coverage_clean_spec = coverage_clean_command();
    let coverage_spec = coverage_command(repo_root);
    run_spec(repo_root, &coverage_clean_spec)?;
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
    let archive = snapshot_root
        .join("target")
        .join("package")
        .join(format!("htmlcut-core-{version}.crate"));

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

fn ensure_coverage_prerequisites(repo_root: &Path) -> DynResult<()> {
    let toolchains = capture_command_output(
        repo_root,
        &CommandSpec::new("rustup", ["toolchain", "list"], false, false),
    )
    .map_err(|error| format!("coverage preflight could not query rustup toolchains: {error}"))?;
    let toolchains = String::from_utf8(toolchains)
        .map_err(|error| format!("coverage preflight received invalid rustup output: {error}"))?;

    let toolchain_failures = coverage_preflight_failures(&toolchains, "");
    if toolchain_failures.contains(&CoveragePreflightFailure::MissingNightlyToolchain) {
        return Err(coverage_preflight_message(&toolchain_failures).into());
    }

    let components = capture_command_output(
        repo_root,
        &CommandSpec::new(
            "rustup",
            ["component", "list", "--toolchain", "nightly", "--installed"],
            false,
            false,
        ),
    )
    .map_err(|error| format!("coverage preflight could not query nightly components: {error}"))?;
    let components = String::from_utf8(components).map_err(|error| {
        format!("coverage preflight received invalid component output: {error}")
    })?;

    let failures = coverage_preflight_failures(&toolchains, &components);
    if failures.is_empty() {
        Ok(())
    } else {
        Err(coverage_preflight_message(&failures).into())
    }
}

fn run_spec(repo_root: &Path, spec: &CommandSpec) -> DynResult<()> {
    let mut command = Command::new(&spec.program);
    command.current_dir(repo_root);
    command.args(&spec.args);
    command.stdin(Stdio::inherit());
    if spec.quiet_stdout {
        command.stdout(Stdio::null());
    } else {
        command.stdout(Stdio::inherit());
    }
    command.stderr(Stdio::inherit());
    if spec.force_clang {
        command.env("CC", "clang");
    }

    let status = command.status()?;
    if status.success() {
        return Ok(());
    }

    Err(format!("command failed with status {status}").into())
}

fn capture_command_output(repo_root: &Path, spec: &CommandSpec) -> DynResult<Vec<u8>> {
    let mut command = Command::new(&spec.program);
    command.current_dir(repo_root);
    command.args(&spec.args);
    command.stdin(Stdio::null());
    command.stdout(Stdio::piped());
    command.stderr(Stdio::inherit());
    if spec.force_clang {
        command.env("CC", "clang");
    }

    let output = command.output()?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(format!("command failed with status {}", output.status).into())
    }
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask should live directly under the workspace root")
        .to_path_buf()
}
