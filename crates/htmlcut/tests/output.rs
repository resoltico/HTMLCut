// SPDX-License-Identifier: MPL-2.0
mod support;
use std::{fs, path::Path};
fn page(root: &Path) -> std::path::PathBuf {
    let page = root.join("page.html");
    fs::write(&page, b"<p>x</p>").unwrap();
    page
}
fn stages(root: &Path) {
    assert!(fs::read_dir(root).unwrap().all(|e| {
        !e.unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".htmlcut-")
    }));
}
#[test]
fn refusal_preserves_existing_bytes_and_mtime_then_success_replaces_content() {
    let root = tempfile::tempdir().unwrap();
    let page = page(root.path());
    let destination = root.path().join("result");
    fs::write(&destination, b"old").unwrap();
    let before = fs::metadata(&destination).unwrap().modified().unwrap();
    let o = support::command()
        .args(["get", "missing"])
        .arg(&page)
        .arg("-o")
        .arg(&destination)
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(3));
    assert_eq!(fs::read(&destination).unwrap(), b"old");
    assert_eq!(
        fs::metadata(&destination).unwrap().modified().unwrap(),
        before
    );
    stages(root.path());
    let o = support::command()
        .args(["get", "p"])
        .arg(&page)
        .arg("-o")
        .arg(&destination)
        .output()
        .unwrap();
    assert!(o.status.success(), "{o:?}");
    assert!(o.stdout.is_empty());
    assert_eq!(fs::read(&destination).unwrap(), b"\"x\"\n");
    stages(root.path());
}
#[test]
fn directories_are_refused_without_staging() {
    let root = tempfile::tempdir().unwrap();
    let page = page(root.path());
    let o = support::command()
        .args(["get", "p"])
        .arg(page)
        .arg("-o")
        .arg(root.path())
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(5));
    let e: serde_json::Value = serde_json::from_slice(&o.stderr).unwrap();
    assert_eq!(e["error"], "output_unwritable");
    stages(root.path());
}
#[cfg(unix)]
fn symlink(target: &Path, link: &Path) {
    std::os::unix::fs::symlink(target, link).unwrap();
}
#[cfg(windows)]
fn symlink(target: &Path, link: &Path) {
    std::os::windows::fs::symlink_file(target, link).unwrap();
}
#[cfg(any(unix, windows))]
#[test]
fn links_keep_the_link_replace_the_target_and_refuse_dangling_links() {
    let root = tempfile::tempdir().unwrap();
    let page = page(root.path());
    let target = root.path().join("target");
    let link = root.path().join("link");
    fs::write(&target, b"old").unwrap();
    symlink(&target, &link);
    let o = support::command()
        .args(["get", "p"])
        .arg(&page)
        .arg("-o")
        .arg(&link)
        .output()
        .unwrap();
    assert!(o.status.success(), "{o:?}");
    assert!(
        fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(fs::read(&target).unwrap(), b"\"x\"\n");
    stages(root.path());
    fs::remove_file(&target).unwrap();
    let o = support::command()
        .args(["get", "p"])
        .arg(&page)
        .arg("-o")
        .arg(&link)
        .output()
        .unwrap();
    assert_eq!(o.status.code(), Some(5));
    assert!(
        fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert!(String::from_utf8_lossy(&o.stderr).contains("dangling symbolic link"));
    stages(root.path());
}
#[cfg(unix)]
#[test]
fn replacement_keeps_mode_and_creation_obeys_umask() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let page = page(root.path());
    let existing = root.path().join("existing");
    fs::write(&existing, b"old").unwrap();
    fs::set_permissions(&existing, fs::Permissions::from_mode(0o640)).unwrap();
    let o = support::command()
        .args(["get", "p"])
        .arg(&page)
        .arg("-o")
        .arg(&existing)
        .output()
        .unwrap();
    assert!(o.status.success());
    assert_eq!(
        fs::metadata(existing).unwrap().permissions().mode() & 0o777,
        0o640
    );
    let new = root.path().join("created");
    let o = std::process::Command::new("sh")
        .args([
            "-c",
            "umask 022; exec \"$1\" get p \"$2\" -o \"$3\"",
            "htmlcut-output",
        ])
        .arg(support::binary())
        .arg(page)
        .arg(&new)
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();
    assert!(o.status.success(), "{o:?}");
    assert_eq!(
        fs::metadata(new).unwrap().permissions().mode() & 0o777,
        0o644
    );
    stages(root.path());
}
#[cfg(unix)]
#[test]
fn fifo_and_dev_null_receive_direct_writes() {
    use std::process::{Command, Stdio};
    let root = tempfile::tempdir().unwrap();
    let page = page(root.path());
    let fifo = root.path().join("pipe");
    assert!(
        Command::new("mkfifo")
            .arg(&fifo)
            .stdin(Stdio::null())
            .status()
            .unwrap()
            .success()
    );
    let mut reader = Command::new("cat")
        .arg(&fifo)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let o = support::command()
        .args(["get", "p"])
        .arg(&page)
        .arg("-o")
        .arg(&fifo)
        .output()
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while reader.try_wait().unwrap().is_none() && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    if reader.try_wait().unwrap().is_none() {
        reader.kill().unwrap();
    }
    let read = reader.wait_with_output().unwrap();
    assert!(o.status.success(), "{o:?}");
    assert!(read.status.success());
    assert_eq!(read.stdout, b"\"x\"\n");
    assert!(!fs::metadata(&fifo).unwrap().is_file());
    let o = support::command()
        .args(["get", "p"])
        .arg(page)
        .args(["-o", "/dev/null"])
        .output()
        .unwrap();
    assert!(o.status.success(), "{o:?}");
    stages(root.path());
}
#[cfg(windows)]
#[test]
fn nul_is_a_direct_destination() {
    let root = tempfile::tempdir().unwrap();
    let page = page(root.path());
    let o = support::command()
        .args(["get", "p"])
        .arg(page)
        .args(["-o", "NUL"])
        .current_dir(root.path())
        .output()
        .unwrap();
    assert!(o.status.success(), "{o:?}");
    stages(root.path());
}
#[test]
fn closed_stdout_is_silent_success() {
    use std::process::Stdio;
    let root = tempfile::tempdir().unwrap();
    let page = root.path().join("large.html");
    fs::write(&page, format!("<p>{}</p>", "x".repeat(1_000_000))).unwrap();
    let mut child = support::command()
        .args(["get", "p"])
        .arg(page)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    drop(child.stdout.take());
    let o = child.wait_with_output().unwrap();
    assert!(o.status.success(), "{o:?}");
    assert!(o.stderr.is_empty());
}
