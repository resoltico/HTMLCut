use std::fs;
use std::path::{Path, PathBuf};

use crate::model::{CommandSpec, DynResult};

const FUZZ_SMOKE_TARGETS: [&str; 4] = [
    "parse_document_bytes",
    "selector_parsing",
    "slice_boundaries",
    "extraction_request_building",
];

/// Default libFuzzer iteration budget for the short smoke workflow.
pub const DEFAULT_FUZZ_SMOKE_RUNS: u32 = 200;

/// One actionable prerequisite that the fuzz-smoke command checks before running.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FuzzSmokePreflightFailure {
    /// The nightly toolchain is not installed.
    MissingNightlyToolchain,
    /// The `cargo-fuzz` runner is not installed.
    MissingCargoFuzz,
}

/// Returns the maintained fuzz targets in their canonical smoke-run order.
pub fn fuzz_smoke_targets() -> &'static [&'static str] {
    &FUZZ_SMOKE_TARGETS
}

/// Validates that one fuzz target name belongs to the maintained inventory.
pub fn assert_known_fuzz_target(target: &str) -> DynResult<()> {
    if FUZZ_SMOKE_TARGETS.contains(&target) {
        return Ok(());
    }

    Err(format!(
        "unknown fuzz target `{target}`. Valid targets: {}.",
        FUZZ_SMOKE_TARGETS.join(", ")
    )
    .into())
}

/// Returns the checked-in corpus directory for one maintained fuzz target.
pub fn fuzz_corpus_dir(repo_root: &Path, target: &str) -> PathBuf {
    repo_root.join("fuzz").join("corpus").join(target)
}

/// Copies one checked-in corpus into disposable scratch space for a smoke run.
pub fn stage_fuzz_corpus(
    repo_root: &Path,
    scratch_root: &Path,
    target: &str,
) -> DynResult<PathBuf> {
    assert_known_fuzz_target(target)?;

    let source = fuzz_corpus_dir(repo_root, target);
    let destination = scratch_root.join(target);
    copy_dir_recursive(&source, &destination).map_err(|error| {
        format!(
            "failed to stage fuzz corpus for `{target}` from {} into {}: {error}",
            source.display(),
            destination.display()
        )
    })?;
    Ok(destination)
}

/// Returns missing prerequisites for the live fuzz-smoke workflow.
pub fn fuzz_smoke_preflight_failures(
    toolchains_output: &str,
    cargo_fuzz_installed: bool,
) -> Vec<FuzzSmokePreflightFailure> {
    let has_nightly_toolchain = toolchains_output
        .lines()
        .map(str::trim)
        .any(|line| line.starts_with("nightly"));

    let mut failures = Vec::new();
    if !has_nightly_toolchain {
        failures.push(FuzzSmokePreflightFailure::MissingNightlyToolchain);
    }
    if !cargo_fuzz_installed {
        failures.push(FuzzSmokePreflightFailure::MissingCargoFuzz);
    }

    failures
}

/// Formats the actionable preflight error shown before fuzz-smoke starts.
pub fn fuzz_smoke_preflight_message(failures: &[FuzzSmokePreflightFailure]) -> String {
    let missing_nightly = failures.contains(&FuzzSmokePreflightFailure::MissingNightlyToolchain);
    let missing_cargo_fuzz = failures.contains(&FuzzSmokePreflightFailure::MissingCargoFuzz);

    let mut message = String::from(
        "Rust fuzz-smoke preflight failed. HTMLCut keeps stable as the default toolchain, but `cargo xtask fuzz-smoke` requires nightly plus the `cargo-fuzz` runner.\n",
    );

    if missing_nightly {
        message.push_str(
            "\nInstall nightly first:\n  rustup toolchain install nightly --profile minimal\n",
        );
    }

    if missing_cargo_fuzz {
        message.push_str(
            "\nInstall the fuzz runner on the maintained macOS path with:\n  CC=clang CXX=clang++ cargo install cargo-fuzz --locked\n",
        );
    }

    message
}

/// Builds the `cargo fuzz --help` probe used by fuzz-smoke preflight.
pub fn cargo_fuzz_probe_command() -> CommandSpec {
    CommandSpec::new("cargo", ["fuzz", "--help"], true, false)
}

/// Builds the non-mutating `cargo fuzz run` command used by the smoke workflow.
pub fn fuzz_smoke_command(target: &str, staged_corpus: &Path, runs: u32) -> DynResult<CommandSpec> {
    assert_known_fuzz_target(target)?;

    Ok(CommandSpec::new(
        "cargo",
        [
            "+nightly".to_owned(),
            "fuzz".to_owned(),
            "run".to_owned(),
            "--fuzz-dir".to_owned(),
            "fuzz".to_owned(),
            target.to_owned(),
            staged_corpus.to_string_lossy().to_string(),
            "--".to_owned(),
            format!("-runs={runs}"),
        ],
        false,
        true,
    ))
}

fn copy_dir_recursive(source: &Path, destination: &Path) -> DynResult<()> {
    fs::create_dir_all(destination)?;

    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());

        if entry.file_type()?.is_dir() {
            copy_dir_recursive(&source_path, &destination_path)?;
        } else {
            fs::copy(&source_path, &destination_path)?;
        }
    }

    Ok(())
}
