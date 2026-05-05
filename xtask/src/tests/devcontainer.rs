use std::fs;
use std::path::Path;

#[test]
fn devcontainer_check_routes_cargo_target_into_the_writable_cache_mount() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");
    let script = fs::read_to_string(repo_root.join("scripts").join("devcontainer-check.sh"))
        .expect("read devcontainer-check.sh");

    assert!(script.contains("export CARGO_TARGET_DIR=/home/vscode/.cache/htmlcut-target"));
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
