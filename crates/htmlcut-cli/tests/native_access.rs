// SPDX-License-Identifier: MPL-2.0
//! Delivered-binary access enforcement, with independent OS positive/negative controls.
#![cfg(unix)]
use std::{
    fs::{self, OpenOptions},
    os::unix::fs::{MetadataExt, PermissionsExt, symlink},
    path::Path,
    process::{Command, Output, Stdio},
    time::{Duration, Instant},
};
fn binary() -> std::ffi::OsString {
    std::env::var_os("HTMLCUT_TEST_BINARY").expect("native acceptance requires HTMLCUT_TEST_BINARY")
}
fn completed(mut command: Command) -> Output {
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("required native helper unavailable");
    let deadline = Instant::now() + Duration::from_secs(5);
    while child.try_wait().unwrap().is_none() {
        if Instant::now() >= deadline {
            let _ = child.kill();
            let output = child.wait_with_output().unwrap();
            panic!("native command failed to finish itself: {output:?}");
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    child.wait_with_output().unwrap()
}
fn publish(page: &Path, path: &Path) -> Output {
    let mut c = Command::new(binary());
    c.args(["get", "h1"]).arg(page).arg("-o").arg(path);
    completed(c)
}
fn helper(name: &str, args: &[&std::ffi::OsStr]) -> Output {
    let mut c = Command::new(name);
    c.args(args);
    let output = completed(c);
    assert!(
        output.status.success(),
        "required native helper failed: {output:?}"
    );
    output
}
fn permissions(path: &Path, mode: u32) {
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
}
fn policy(path: &Path) -> (u32, u32, u32) {
    let m = fs::metadata(path).unwrap();
    (m.uid(), m.gid(), m.mode() & 0o777)
}
#[test]
#[ignore = "requires the delivered executable; native package acceptance runs this test"]
fn directory_write_and_search_without_listing_preserve_existing_and_new_policy() {
    let root = tempfile::tempdir().unwrap();
    let page = root.path().join("page.html");
    fs::write(&page, b"<h1>complete</h1>").unwrap();
    let parent = root.path().join("parent");
    fs::create_dir(&parent).unwrap();
    let existing = parent.join("existing.json");
    let absent = parent.join("absent.json");
    let control = parent.join("os-control.json");
    let alias = root.path().join("alias");
    symlink(&parent, &alias).unwrap();
    fs::write(&existing, b"old").unwrap();
    permissions(&existing, 0o640);
    let result = std::panic::catch_unwind(|| {
        let prior = policy(&existing);
        permissions(&parent, 0o311);
        assert_eq!(
            fs::read_dir(&parent).unwrap_err().kind(),
            std::io::ErrorKind::PermissionDenied,
            "native listing negative control requires an unprivileged identity"
        );
        drop(OpenOptions::new().write(true).open(&existing).unwrap());
        let stage = parent.join("os-stage.json");
        fs::write(&stage, b"os-control").unwrap();
        drop(
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&control)
                .unwrap(),
        );
        fs::rename(stage, &control).unwrap();
        assert_eq!(fs::read(&control).unwrap(), b"os-control");
        let default = policy(&control);
        for (path, expected) in [(&existing, prior), (&alias.join("absent.json"), default)] {
            let output = publish(&page, path);
            assert!(output.status.success(), "{output:?}");
            assert!(output.stdout.is_empty() && output.stderr.is_empty());
            assert_eq!(fs::read(path).unwrap(), b"\"complete\"\n");
            assert_eq!(policy(path), expected);
        }
        for mode in [0o511, 0o600] {
            permissions(&parent, mode);
            if mode == 0o511 {
                assert_eq!(
                    fs::write(parent.join("must-refuse.json"), b"control")
                        .unwrap_err()
                        .kind(),
                    std::io::ErrorKind::PermissionDenied
                );
            } else {
                assert_eq!(
                    fs::read(&existing).unwrap_err().kind(),
                    std::io::ErrorKind::PermissionDenied
                );
            }
            for path in [&existing, &parent.join("must-refuse.json")] {
                let output = publish(&page, path);
                assert_eq!(output.status.code(), Some(5), "{output:?}");
                assert!(output.stdout.is_empty());
                assert_eq!(
                    serde_json::from_slice::<serde_json::Value>(&output.stderr).unwrap()["error"],
                    "output_unwritable"
                );
            }
            permissions(&parent, 0o700);
            assert_eq!(fs::read(&existing).unwrap(), b"\"complete\"\n");
            assert!(!parent.join("must-refuse.json").exists());
        }
        let mut inventory = fs::read_dir(&parent)
            .unwrap()
            .map(|p| p.unwrap().file_name())
            .collect::<Vec<_>>();
        inventory.sort();
        assert_eq!(
            inventory,
            ["absent.json", "existing.json", "os-control.json"]
        );
    });
    permissions(&parent, 0o700);
    result.unwrap();
    assert!(absent.exists());
}
#[cfg(target_os = "macos")]
#[test]
#[ignore = "requires delivered executable and native ACL tools; package acceptance runs this test"]
fn native_acl_denial_and_inherited_directory_denial_remain_enforced() {
    use std::ffi::OsStr;
    let root = tempfile::tempdir().unwrap();
    let page = root.path().join("page.html");
    let output = root.path().join("result.json");
    fs::write(&page, b"<h1>complete</h1>").unwrap();
    fs::write(&output, b"old-private").unwrap();
    permissions(&output, 0o644);
    helper(
        "chmod",
        &[
            OsStr::new("+a"),
            OsStr::new("everyone allow read,file_inherit"),
            root.path().as_os_str(),
        ],
    );
    helper(
        "chmod",
        &[
            OsStr::new("+a"),
            OsStr::new("everyone deny read"),
            output.as_os_str(),
        ],
    );
    let result = std::panic::catch_unwind(|| {
        assert_eq!(
            fs::read(&output).unwrap_err().kind(),
            std::io::ErrorKind::PermissionDenied
        );
        assert!(publish(&page, &output).status.success());
        assert_eq!(
            fs::read(&output).unwrap_err().kind(),
            std::io::ErrorKind::PermissionDenied
        );
        let acl = helper("ls", &[OsStr::new("-le"), output.as_os_str()]);
        assert!(String::from_utf8_lossy(&acl.stdout).contains("deny read"));
        helper(
            "chmod",
            &[
                OsStr::new("+a"),
                OsStr::new("everyone deny read,directory_inherit,only_inherit"),
                root.path().as_os_str(),
            ],
        );
        let control = root.path().join("control");
        fs::create_dir(&control).unwrap();
        let denied = fs::read_dir(&control).unwrap_err().kind();
        helper("chmod", &[OsStr::new("-N"), control.as_os_str()]);
        fs::remove_dir(control).unwrap();
        assert_eq!(denied, std::io::ErrorKind::PermissionDenied);
        let before = fs::read_dir(root.path()).unwrap().count();
        assert!(publish(&page, &output).status.success());
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), before);
    });
    helper("chmod", &[OsStr::new("-N"), output.as_os_str()]);
    helper("chmod", &[OsStr::new("-N"), root.path().as_os_str()]);
    result.unwrap();
}
#[cfg(target_os = "linux")]
#[test]
#[ignore = "requires delivered executable, setfacl and sudo -n -u nobody; package acceptance runs this test"]
fn named_user_acl_denial_preserves_actual_enforcement() {
    use std::ffi::OsStr;
    let root = tempfile::tempdir().unwrap();
    permissions(root.path(), 0o755);
    let page = root.path().join("page.html");
    let path = root.path().join("result.json");
    fs::write(&page, b"<h1>complete</h1>").unwrap();
    fs::write(&path, b"old-private").unwrap();
    permissions(&path, 0o644);
    let read = || {
        let mut command = Command::new("sudo");
        command.args(["-n", "-u", "nobody", "cat"]).arg(&path);
        completed(command)
    };
    assert!(
        read().status.success(),
        "native authorized read control must succeed"
    );
    helper(
        "setfacl",
        &[
            OsStr::new("-m"),
            OsStr::new("d:u:nobody:r-x"),
            root.path().as_os_str(),
        ],
    );
    helper(
        "setfacl",
        &[
            OsStr::new("-m"),
            OsStr::new("u:nobody:---"),
            path.as_os_str(),
        ],
    );
    assert!(!read().status.success());
    assert!(publish(&page, &path).status.success());
    assert!(!read().status.success());
    helper(
        "setfacl",
        &[OsStr::new("-x"), OsStr::new("u:nobody"), path.as_os_str()],
    );
    assert_eq!(read().stdout, b"\"complete\"\n");
}
