use super::*;
use htmlcut_tempdir::tempdir;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

fn write_repo_scaffold(repo_root: &Path) {
    fs::write(
        repo_root.join("Cargo.toml"),
        "[workspace.package]\nversion = \"3.0.0\"\n",
    )
    .expect("write Cargo.toml");
    fs::write(repo_root.join("changelog.md"), "## [Unreleased]\n").expect("write changelog.md");
    let baseline_dir = repo_root.join("semver-baseline").join("htmlcut-core");
    fs::create_dir_all(&baseline_dir).expect("create semver baseline dir");
    fs::write(
        baseline_dir.join("Cargo.toml"),
        "[package]\nname = \"htmlcut-core\"\nversion = \"2.0.0\"\n",
    )
    .expect("write baseline Cargo.toml");
    write_empty_release_targets_script(repo_root);
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

mod command_exec;
mod coverage;
mod docs;
mod fuzz;
mod host_tools;
mod plan;
mod policy;
mod preflight;
mod release;
mod toolchain;
mod versions;

fn seed_tracked_files(repo_root: &Path) -> BTreeMap<PathBuf, String> {
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
        fs::create_dir_all(file_path.parent().expect("parent")).expect("create dir");
        fs::write(&file_path, "// tracked\n").expect("write tracked file");
    }

    tracked_files(repo_root).expect("tracked files")
}

fn tracked_subset(repo_root: &Path, relative_paths: &[&str]) -> BTreeMap<PathBuf, String> {
    for relative_path in relative_paths {
        let file_path = repo_root.join(relative_path);
        fs::create_dir_all(file_path.parent().expect("parent")).expect("create dir");
        fs::write(&file_path, "// tracked\n").expect("write tracked file");
    }

    relative_paths
        .iter()
        .map(|relative_path| {
            (
                normalize_path(repo_root, &repo_root.join(relative_path)).expect("path"),
                (*relative_path).to_owned(),
            )
        })
        .collect()
}
