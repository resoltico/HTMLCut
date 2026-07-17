use super::*;
use std::fs;
use std::path::Path;
use std::process::Command;

use serde_json::Value;

fn bash_command() -> Command {
    Command::new(crate::release::bash_program_for_tests())
}

fn release_script_argument(repo_root: &Path, script_name: &str) -> String {
    crate::release::bash_source_argument_for_tests(&repo_root.join("scripts").join(script_name))
}

fn init_git_repo(repo_root: &Path) {
    git(repo_root, &["init", "--initial-branch=main"]);
    git(repo_root, &["config", "user.name", "HTMLCut Tests"]);
    git(
        repo_root,
        &["config", "user.email", "htmlcut-tests@example.invalid"],
    );
}

fn assert_visible_as_untracked(repo_root: &Path, paths: &[&str]) {
    let output = git_output(repo_root, &["ls-files", "--others", "--exclude-standard"]);
    let listed_paths = String::from_utf8(output).expect("utf8 git ls-files output");
    for path in paths {
        assert!(
            listed_paths.lines().any(|listed| listed == *path),
            "expected {path} to stay untracked-and-visible, got:\n{listed_paths}",
        );
    }
}

fn git(repo_root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo_root)
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "git {:?} failed:\nstdout:\n{}\nstderr:\n{}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn git_output(repo_root: &Path, args: &[&str]) -> Vec<u8> {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo_root)
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "git {:?} failed:\nstdout:\n{}\nstderr:\n{}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

fn tar_entry_names(archive: &[u8]) -> Vec<String> {
    let mut entries = Vec::new();
    let mut offset = 0usize;

    while offset + 512 <= archive.len() {
        let header = &archive[offset..offset + 512];
        if header.iter().all(|byte| *byte == 0) {
            break;
        }

        let name = tar_header_field(&header[..100]);
        let prefix = tar_header_field(&header[345..500]);
        let entry = if prefix.is_empty() {
            name
        } else {
            format!("{prefix}/{name}")
        };
        entries.push(entry);

        let size = tar_octal_field(&header[124..136]);
        let payload_blocks = size.div_ceil(512);
        offset += 512 + (payload_blocks * 512);
    }

    entries
}

fn tar_header_field(field: &[u8]) -> String {
    let end = field
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(field.len());
    String::from_utf8_lossy(&field[..end]).trim().to_owned()
}

fn tar_octal_field(field: &[u8]) -> usize {
    let value = tar_header_field(field);
    usize::from_str_radix(value.trim(), 8).expect("tar size is valid octal")
}

mod launcher_and_protocol;
mod scripts_and_archives;
mod shell_helpers;
