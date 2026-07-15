use std::fs;
use std::path::Path;
use std::process::Command;

#[test]
fn devcontainer_check_routes_cargo_target_into_the_writable_cache_mount() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");
    let script = fs::read_to_string(repo_root.join("scripts").join("devcontainer-check.sh"))
        .expect("read devcontainer-check.sh");

    assert!(
        script.contains("export CARGO_TARGET_DIR=/home/vscode/.cache/htmlcut-artifacts/target")
    );
    assert!(
        script.contains("export CARGO_BUILD_BUILD_DIR=/home/vscode/.cache/htmlcut-artifacts/build")
    );
    assert!(script.contains("git config --global --add safe.directory /workspaces/htmlcut"));
    assert!(script.contains("./scripts/devcontainer-prepare-user-home.sh"));
    assert!(script.contains("./check.sh"));
}

#[test]
fn devcontainer_check_mounts_worktree_git_metadata_for_release_worktrees() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");
    let script = fs::read_to_string(repo_root.join("scripts").join("devcontainer-check.sh"))
        .expect("read devcontainer-check.sh");

    assert!(script.contains("append_worktree_git_metadata_mounts"));
    assert!(script.contains("gitdir: "));
    assert!(script.contains("--volume \"${repo_root}:${repo_root}:ro\""));
    assert!(script.contains("--volume \"${common_git_dir}:${common_git_dir}:ro\""));
}

#[test]
fn contributor_rust_tool_inventory_pins_nightly_miri_components() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");
    let script = fs::read_to_string(repo_root.join("scripts").join("contributor-rust-tools.sh"))
        .expect("read contributor-rust-tools.sh");

    assert!(script.contains("HTMLCUT_CONTRIBUTOR_RUST_STABLE_COMPONENTS=("));
    assert!(script.contains("HTMLCUT_CONTRIBUTOR_RUST_NIGHTLY_COMPONENTS=("));
    assert!(script.contains("\"llvm-tools-preview\""));
    assert!(script.contains("\"miri\""));
    assert!(script.contains("\"rust-src\""));
    assert!(script.contains("htmlcut_contributor_rustup_toolchain_install()"));
}

#[test]
fn contributor_rust_tools_script_is_safe_to_source_under_readonly_shell_locals() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");
    let output = Command::new("bash")
        .current_dir(repo_root)
        .arg("-lc")
        .arg(
            r#"
                set -euo pipefail
                script_dir=/tmp/htmlcut-readonly-script-dir
                readonly script_dir
                repo_root=/tmp/htmlcut-readonly-repo-root
                readonly repo_root
                source ./scripts/contributor-rust-tools.sh
                [[ -n "${HTMLCUT_CONTRIBUTOR_RUST_STABLE_TOOLCHAIN}" ]]
                [[ -n "${HTMLCUT_CONTRIBUTOR_RUST_NIGHTLY_TOOLCHAIN}" ]]
            "#,
        )
        .output()
        .expect("run contributor-rust-tools source-safety probe");

    assert!(
        output.status.success(),
        "expected contributor-rust-tools.sh to source cleanly under readonly shell locals\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn devcontainer_bootstrap_and_validation_cover_nightly_miri() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");
    let bootstrap = fs::read_to_string(repo_root.join("scripts").join("devcontainer-bootstrap.sh"))
        .expect("read devcontainer-bootstrap.sh");
    let validator = fs::read_to_string(repo_root.join("scripts").join("validate-devcontainer.sh"))
        .expect("read validate-devcontainer.sh");

    assert!(bootstrap.contains("htmlcut_contributor_install_nightly_toolchain"));
    assert!(bootstrap.contains("htmlcut_contributor_install_stable_toolchain_components"));
    assert!(bootstrap.contains("retry_command()"));
    assert!(bootstrap.contains("retry_command 3 5 install_rustup_once"));
    assert!(bootstrap.contains("retry_command 3 5 rustup toolchain install"));
    assert!(bootstrap.contains("retry_command 3 5 htmlcut_contributor_install_nightly_toolchain"));
    assert!(
        bootstrap
            .contains("retry_command 3 5 htmlcut_contributor_install_stable_toolchain_components")
    );
    assert!(validator.contains("./scripts/xtask.sh --help >/dev/null"));
    assert!(!validator.contains("cargo xtask --help >/dev/null"));
    assert!(validator.contains("HTMLCUT_STABLE_TOOLCHAIN"));
    assert!(!validator.contains("rustc 1\\.95\\.0"));
    assert!(bootstrap.contains("cargo +nightly miri --version >/dev/null"));
    assert_eq!(
        validator
            .matches("cargo +nightly miri --version >/dev/null")
            .count(),
        2
    );
}
