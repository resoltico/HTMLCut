use super::*;
use htmlcut_tempdir::tempdir;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const SOURCE_SHAPE_POLICY: &str = r#"version = 1

[[rules]]
path = "crates/htmlcut-cli/src/"
match = "prefix"
role = "test CLI source"
owner = "xtask test fixture"
rationale = "The synthetic repository needs explicit source ownership when it runs maintainer gates."
split_trigger = "Fixture source exceeds its deliberately broad test budget."
max_physical_lines = 100
max_items = 100
max_public_items = 100
max_imports = 100
max_functions = 100
max_decision_points = 100
max_match_arms = 100
allowed_internal_dependencies = []

[[rules]]
path = "crates/htmlcut-cli/tests/"
match = "prefix"
role = "test CLI integration scenario"
owner = "xtask test fixture"
rationale = "The synthetic repository needs explicit test ownership when it runs maintainer gates."
split_trigger = "Fixture test source exceeds its deliberately broad test budget."
max_physical_lines = 100
max_items = 100
max_public_items = 100
max_imports = 100
max_functions = 100
max_decision_points = 100
max_match_arms = 100
allowed_internal_dependencies = []

"#;

const XTASK_SOURCE_RULE: &str = r#"
[[rules]]
path = "xtask/src/"
match = "prefix"
role = "test xtask source"
owner = "xtask test fixture"
rationale = "The synthetic repository needs explicit maintenance-tool ownership when it runs maintainer gates."
split_trigger = "Fixture maintenance source exceeds its deliberately broad test budget."
max_physical_lines = 100
max_items = 100
max_public_items = 100
max_imports = 100
max_functions = 100
max_decision_points = 100
max_match_arms = 100
allowed_internal_dependencies = []
"#;

fn test_command_spec<I, S>(
    program: impl Into<PathBuf>,
    args: I,
    quiet_stdout: bool,
    force_clang: bool,
) -> CommandSpec
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let spec = CommandSpec::new(
        program,
        args,
        if quiet_stdout {
            CommandStdout::Quiet
        } else {
            CommandStdout::Inherit
        },
        if force_clang {
            CommandToolchainEnv::ForceClang
        } else {
            CommandToolchainEnv::Inherit
        },
    );

    if spec.program == Path::new("cargo") {
        spec.with_artifact_layout(CommandArtifactLayout::ManagedWorkspace)
    } else {
        spec
    }
}

fn command_is_quiet(command: &CommandSpec) -> bool {
    matches!(command.stdout, CommandStdout::Quiet)
}

fn command_quiets_stderr(command: &CommandSpec) -> bool {
    matches!(command.stderr, CommandStderr::Quiet)
}

fn command_forces_clang(command: &CommandSpec) -> bool {
    matches!(command.toolchain_env, CommandToolchainEnv::ForceClang)
}

fn command_uses_managed_workspace_artifacts(command: &CommandSpec) -> bool {
    matches!(
        command.artifact_layout,
        CommandArtifactLayout::ManagedWorkspace
    )
}

fn command_uses_managed_coverage_artifacts(command: &CommandSpec) -> bool {
    matches!(
        command.artifact_layout,
        CommandArtifactLayout::ManagedCoverage
    )
}

fn with_isolated_managed_workspace_artifacts<T>(
    operation: impl FnOnce(&Path, PathBuf, PathBuf) -> T,
) -> T {
    let repo_root = tempdir().expect("tempdir");
    let target_dir = repo_root.path().join(".managed-artifacts").join("target");
    let build_dir = repo_root.path().join(".managed-artifacts").join("build");

    crate::plan::with_cargo_artifact_dir_overrides_for_tests(
        target_dir.clone(),
        build_dir.clone(),
        || operation(repo_root.path(), target_dir, build_dir),
    )
}

fn write_repo_scaffold(repo_root: &Path) {
    fs::write(
        repo_root.join("Cargo.toml"),
        "[workspace.package]\nversion = \"3.0.0\"\n",
    )
    .expect("write Cargo.toml");
    fs::write(repo_root.join("changelog.md"), "## [Unreleased]\n").expect("write changelog.md");
    fs::write(
        repo_root.join("deny.toml"),
        "[graph]\ntargets = [\n    \"aarch64-apple-darwin\",\n    \"x86_64-apple-darwin\",\n    \"x86_64-unknown-linux-musl\",\n    \"x86_64-pc-windows-msvc\",\n]\n",
    )
    .expect("write deny.toml");
    let baseline_dir = repo_root.join("semver-baseline").join("htmlcut-core");
    fs::create_dir_all(&baseline_dir).expect("create semver baseline dir");
    fs::write(
        baseline_dir.join("Cargo.toml"),
        "[package]\nname = \"htmlcut-core\"\nversion = \"2.0.0\"\n",
    )
    .expect("write baseline Cargo.toml");
    let cli_src_dir = repo_root.join("crates").join("htmlcut-cli").join("src");
    let cli_tests_dir = repo_root.join("crates").join("htmlcut-cli").join("tests");
    fs::create_dir_all(&cli_src_dir).expect("create htmlcut-cli src dir");
    fs::create_dir_all(&cli_tests_dir).expect("create htmlcut-cli tests dir");
    fs::write(cli_src_dir.join("main.rs"), "fn main() {}\n").expect("write htmlcut-cli main.rs");
    for test_target in [
        "discovery",
        "help",
        "inspect",
        "parity",
        "select",
        "transport",
    ] {
        fs::write(
            cli_tests_dir.join(format!("{test_target}.rs")),
            "#[test]\nfn placeholder() {}\n",
        )
        .expect("write htmlcut-cli test target");
    }
    let tooling_dir = repo_root.join("tooling");
    fs::create_dir_all(&tooling_dir).expect("create tooling dir");
    fs::write(
        tooling_dir.join("rust-source-shape-policy.toml"),
        SOURCE_SHAPE_POLICY,
    )
    .expect("write Rust source-shape policy");
    write_empty_release_targets_script(repo_root);
}

fn add_xtask_source_rule_to_repo_scaffold(repo_root: &Path) {
    let policy_path = repo_root
        .join("tooling")
        .join("rust-source-shape-policy.toml");
    let policy = fs::read_to_string(&policy_path).expect("read Rust source-shape policy");
    fs::write(policy_path, format!("{policy}{XTASK_SOURCE_RULE}"))
        .expect("add xtask source-shape rule");
}

fn write_empty_release_targets_script(repo_root: &Path) {
    let scripts_dir = repo_root.join("scripts");
    fs::create_dir_all(&scripts_dir).expect("create scripts dir");
    fs::write(
        scripts_dir.join("release-targets.sh"),
        r#"#!/usr/bin/env bash
release_target_triples() {
    printf '%s\n' \
        'aarch64-apple-darwin' \
        'x86_64-apple-darwin' \
        'x86_64-unknown-linux-musl' \
        'x86_64-pc-windows-msvc'
}

release_matrix_json() {
    printf '{"include":[]}\n'
}

release_asset_names_for_version() {
    :
}

macos_deployment_target_for_target() {
    :
}

case "${1:-}" in
    triples)
        release_target_triples
        ;;
    matrix-json)
        release_matrix_json
        ;;
    assets)
        [[ "${2:-}" == "--version" ]] || exit 64
        release_asset_names_for_version "${3:-}"
        ;;
    macos-deployment-target)
        [[ "${2:-}" == "--target" ]] || exit 64
        macos_deployment_target_for_target "${3:-}"
        ;;
esac
"#,
    )
    .expect("write empty release-targets.sh");
}

mod app;
mod command_exec;
mod coverage;
mod devcontainer;
mod docs;
mod fuzz;
mod host_tools;
mod hygiene;
mod miri;
mod outdated;
mod plan;
mod policy;
mod preflight;
mod release;
mod semver_baseline;
mod toolchain;
mod versions;

fn write_executable_tracked_source(file_path: &Path) {
    fs::create_dir_all(file_path.parent().expect("parent")).expect("create dir");
    fs::write(file_path, "pub(crate) fn tracked() -> usize {\n    1\n}\n")
        .expect("write tracked file");
}

fn seed_tracked_files(repo_root: &Path) -> BTreeMap<PathBuf, TrackedCoverageFile> {
    for relative_path in [
        "crates/htmlcut-core/src/catalog.rs",
        "crates/htmlcut-core/src/contracts/mod.rs",
        "crates/htmlcut-cli/src/execute.rs",
        "crates/htmlcut-cli/src/execute/commands.rs",
        "xtask/src/plan.rs",
    ]
    .into_iter()
    .chain(COVERAGE_EXCLUDED_RELATIVE_PATHS.iter().copied())
    {
        let file_path = repo_root.join(relative_path);
        write_executable_tracked_source(&file_path);
    }

    tracked_files(repo_root).expect("tracked files")
}

fn tracked_subset(
    repo_root: &Path,
    relative_paths: &[&str],
) -> BTreeMap<PathBuf, TrackedCoverageFile> {
    for relative_path in relative_paths {
        let file_path = repo_root.join(relative_path);
        write_executable_tracked_source(&file_path);
    }

    relative_paths
        .iter()
        .map(|relative_path| {
            (
                normalize_path(repo_root, &repo_root.join(relative_path)).expect("path"),
                TrackedCoverageFile::executable(*relative_path),
            )
        })
        .collect()
}
