use std::path::{Path, PathBuf};

use crate::model::{
    COVERAGE_TOOLCHAIN, COVERAGE_TOOLCHAIN_NAME, CommandSpec, CoveragePreflightFailure, DynResult,
};
use crate::plan::cargo_target_dir;

const COVERAGE_PACKAGES: &[&str] = &["htmlcut-core", "htmlcut-cli", "xtask"];

/// Builds the `cargo llvm-cov` command used by the one-ring coverage gate.
pub fn coverage_command(repo_root: &Path) -> CommandSpec {
    let mut args = vec![
        COVERAGE_TOOLCHAIN.to_owned(),
        "llvm-cov".to_owned(),
        "--branch".to_owned(),
    ];
    for package in COVERAGE_PACKAGES {
        args.push("-p".to_owned());
        args.push((*package).to_owned());
    }
    args.extend(
        [
            "--all-targets",
            "--all-features",
            "--locked",
            "--json",
            "--output-path",
        ]
        .into_iter()
        .map(str::to_owned),
    );
    args.push(
        coverage_output_path(repo_root)
            .to_string_lossy()
            .into_owned(),
    );

    CommandSpec::new("cargo", args, false, true)
}

/// Builds the cleanup command that clears stale `llvm-cov` state before measurement.
pub fn coverage_clean_command() -> CommandSpec {
    CommandSpec::new(
        "cargo",
        [COVERAGE_TOOLCHAIN, "llvm-cov", "clean", "--workspace"],
        false,
        false,
    )
}

/// Returns missing nightly prerequisites for the branch-coverage gate.
pub fn coverage_preflight_failures(
    toolchains_output: &str,
    installed_components_output: &str,
) -> Vec<CoveragePreflightFailure> {
    let has_nightly_toolchain = toolchains_output
        .lines()
        .map(str::trim)
        .any(|line| line.starts_with(COVERAGE_TOOLCHAIN_NAME));
    if !has_nightly_toolchain {
        return vec![
            CoveragePreflightFailure::MissingNightlyToolchain,
            CoveragePreflightFailure::MissingNightlyLlvmTools,
        ];
    }

    let has_llvm_tools = installed_components_output
        .lines()
        .map(str::trim)
        .any(|line| line.starts_with("llvm-tools"));

    if has_llvm_tools {
        Vec::new()
    } else {
        vec![CoveragePreflightFailure::MissingNightlyLlvmTools]
    }
}

/// Formats the actionable preflight error shown before coverage work starts.
pub fn coverage_preflight_message(failures: &[CoveragePreflightFailure]) -> String {
    let missing_nightly = failures.contains(&CoveragePreflightFailure::MissingNightlyToolchain);
    let missing_llvm_tools = failures.contains(&CoveragePreflightFailure::MissingNightlyLlvmTools);

    let mut message = String::from(
        "Rust coverage preflight failed. HTMLCut keeps stable as the default toolchain, but the coverage gate still requires `cargo +nightly llvm-cov --branch` for true branch coverage.\n",
    );

    if missing_nightly {
        message.push_str(
            "\nInstall the nightly coverage toolchain first:\n  rustup toolchain install nightly --profile minimal --component llvm-tools-preview\n",
        );
        return message;
    }

    if missing_llvm_tools {
        message.push_str(
            "\nNightly is installed, but `llvm-tools-preview` is missing:\n  rustup component add llvm-tools-preview --toolchain nightly\n",
        );
    }

    message
}

/// Returns the JSON file that `cargo llvm-cov` writes for later scoring.
pub fn coverage_output_path(repo_root: &Path) -> PathBuf {
    coverage_target_dir(repo_root).join("coverage.json")
}

/// Ensures the target directory that will receive `coverage.json` already exists.
pub fn ensure_coverage_output_dir(repo_root: &Path) -> DynResult<()> {
    std::fs::create_dir_all(coverage_target_dir(repo_root))?;
    Ok(())
}

fn coverage_target_dir(repo_root: &Path) -> PathBuf {
    cargo_target_dir(repo_root)
}

#[cfg(test)]
pub(crate) fn coverage_output_path_for_tests(
    repo_root: &Path,
    target_dir: Option<&Path>,
) -> PathBuf {
    crate::plan::cargo_target_dir_for_tests(repo_root, target_dir).join("coverage.json")
}

#[cfg(test)]
pub(crate) fn coverage_target_dir_for_tests(
    repo_root: &Path,
    target_dir: Option<&Path>,
) -> PathBuf {
    crate::plan::cargo_target_dir_for_tests(repo_root, target_dir)
}
