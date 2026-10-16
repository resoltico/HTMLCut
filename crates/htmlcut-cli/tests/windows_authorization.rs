// SPDX-License-Identifier: MPL-2.0
//! Independent native access controls against the production executable.
#![cfg(windows)]
use serde_json::Value;
use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    time::{Duration, Instant},
};
fn binary() -> OsString {
    std::env::var_os("HTMLCUT_TEST_BINARY").unwrap_or_else(|| env!("CARGO_BIN_EXE_htmlcut").into())
}
fn bounded(mut command: Command) -> Output {
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("native Windows capability is required");
    let deadline = Instant::now() + Duration::from_secs(20);
    while child.try_wait().unwrap().is_none() {
        if Instant::now() >= deadline {
            let _ = Command::new("taskkill.exe")
                .args(["/PID", &child.id().to_string(), "/T", "/F"])
                .output();
            let _ = child.kill();
            let output = child.wait_with_output().unwrap();
            panic!("native helper deadline exceeded: {output:?}");
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    child.wait_with_output().unwrap()
}
fn power(script: &Path, args: &[&OsStr]) -> Output {
    let mut command = Command::new("powershell.exe");
    command
        .args(["-NoLogo", "-NoProfile", "-NonInteractive", "-File"])
        .arg(script)
        .args(args);
    let output = bounded(command);
    assert!(output.status.success(), "{output:?}");
    output
}
fn publish(page: &Path, path: &Path) {
    let mut command = Command::new(binary());
    command.args(["get", "h1"]).arg(page).arg("-o").arg(path);
    let output = bounded(command);
    assert!(output.status.success(), "{output:?}");
    assert!(
        output.stdout.is_empty() && output.stderr.is_empty(),
        "{output:?}"
    );
}
fn fixture() -> (tempfile::TempDir, PathBuf) {
    let root = tempfile::tempdir().unwrap();
    let page = root.path().join("page.html");
    std::fs::write(&page, b"<h1>complete</h1>").unwrap();
    (root, page)
}
#[test]
fn publication_without_listing_retains_policy_and_rejects_missing_write_and_search() {
    let (root, page) = fixture();
    let script = root.path().join("parent-access.ps1");
    std::fs::write(&script, include_str!("support/parent-access.ps1")).unwrap();
    let parent = root.path().join("parent");
    let alias = root.path().join("alias");
    let backup = root.path().join("policy.bin");
    let result = std::panic::catch_unwind(|| {
        let output = power(
            &script,
            &[
                &binary(),
                page.as_os_str(),
                parent.as_os_str(),
                alias.as_os_str(),
                backup.as_os_str(),
            ],
        );
        let facts: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(facts["enumeration_denied"], true);
        assert_eq!(facts["native_create_write_open_replace"], true);
        assert_eq!(facts["generic_parent_read_error"], 5);
        assert_eq!(
            facts["zero_access_parent_identity"],
            facts["zero_access_alias_identity"]
        );
        for item in facts["positive"]
            .as_array()
            .unwrap()
            .iter()
            .chain(facts["negative"].as_array().unwrap())
        {
            for privilege in ["backup_enabled", "restore_enabled", "security_enabled"] {
                assert_eq!(item["privileges"][privilege], false);
            }
            if item["denied_parent_right"] == 32 {
                assert_eq!(item["privileges"]["traverse_bypass_enabled"], false);
            }
        }
        let positive = facts["positive"].as_array().unwrap();
        assert_eq!(positive.len(), 3);
        for item in positive {
            assert_eq!(item["exit"], 0);
            assert_eq!(item["stdout"], "");
            assert_eq!(item["stderr"], "");
        }
        assert_eq!(
            facts["existing_policy_after_setup"],
            facts["existing_policy_before_setup"]
        );
        assert_eq!(
            facts["existing_policy_before"],
            facts["existing_policy_before_setup"]
        );
        assert_eq!(
            facts["existing_policy_after"],
            facts["existing_policy_before"]
        );
        assert_eq!(facts["absent_policy"], facts["default_policy"]);
        for key in [
            "existing_bytes",
            "absent_bytes",
            "existing_bytes_after_negatives",
        ] {
            assert_eq!(facts[key], "\"complete\"\n");
        }
        let negative = facts["negative"].as_array().unwrap();
        assert_eq!(negative.len(), 4);
        for item in negative {
            assert_eq!(item["exit"], 5);
            assert_eq!(item["stdout"], "");
            assert_eq!(
                serde_json::from_str::<Value>(item["stderr"].as_str().unwrap()).unwrap()["error"],
                "output_unwritable"
            );
        }
        assert_eq!(
            facts["inventory"],
            serde_json::json!(["absent.json", "existing.json", "os-control.json"])
        );
    });
    if backup.exists() && parent.exists() {
        let restore = root.path().join("restore.ps1");
        std::fs::write(&restore,"param([string]$Parent,[string]$Backup);$ErrorActionPreference='Stop';$a=[System.Security.AccessControl.DirectorySecurity]::new();$a.SetSecurityDescriptorBinaryForm([System.IO.File]::ReadAllBytes($Backup),[System.Security.AccessControl.AccessControlSections]::Access);([System.IO.DirectoryInfo]::new($Parent)).SetAccessControl($a)").unwrap();
        power(&restore, &[parent.as_os_str(), backup.as_os_str()]);
    }
    if alias.exists() {
        std::fs::remove_dir(&alias).unwrap();
    }
    if parent.exists() {
        std::fs::remove_dir_all(parent).unwrap();
    }
    result.unwrap();
}
#[test]
fn parent_acl_changes_enforce_identically_on_control_and_published_files() {
    for protected in [false, true] {
        let (root, page) = fixture();
        let script = root.path().join("inheritance.ps1");
        std::fs::write(&script, include_str!("support/parent-inheritance.ps1")).unwrap();
        let parent = root.path().join("parent");
        std::fs::create_dir(&parent).unwrap();
        let control = parent.join("control.json");
        let published = parent.join("published.json");
        let flag = if protected { "1" } else { "0" };
        let operation = |name: &str| {
            power(
                &script,
                &[
                    OsStr::new(name),
                    parent.as_os_str(),
                    control.as_os_str(),
                    published.as_os_str(),
                    OsStr::new(flag),
                ],
            )
        };
        let facts = || serde_json::from_slice::<Value>(&operation("facts").stdout).unwrap();
        let result = std::panic::catch_unwind(|| {
            operation("setup");
            let before = facts();
            assert_eq!(before["control"], before["published"]);
            assert_eq!(before["control"]["protected"], protected);
            assert_eq!(before["control"]["dacl_present"], true);
            assert_eq!(before["control"]["dacl_null"], false);
            let aces = before["control"]["aces"].as_array().unwrap();
            assert!(!aces.is_empty());
            if !protected {
                assert!(
                    aces.iter()
                        .all(|a| a["flags"].as_u64().unwrap() & 0x10 != 0)
                );
            }
            assert!(std::fs::read(&control).is_ok() && std::fs::read(&published).is_ok());
            publish(&page, &published);
            assert_eq!(std::fs::read(&control).unwrap(), b"original");
            assert_eq!(std::fs::read(&published).unwrap(), b"\"complete\"\n");
            assert_eq!(facts(), before);
            operation("deny");
            let denied = facts();
            assert_eq!(denied["control"], denied["published"]);
            let read_deny = denied["control"]["aces"]
                .as_array()
                .unwrap()
                .iter()
                .any(|ace| {
                    ace["type"] == 1
                        && ace["flags"].as_u64().unwrap() & 0x10 != 0
                        && ace["mask"].as_u64().unwrap() & 1 != 0
                        && ace["sid"] == "S-1-1-0"
                });
            assert_eq!(read_deny, !protected);
            for path in [&control, &published] {
                if protected {
                    assert!(std::fs::read(path).is_ok());
                } else {
                    assert_eq!(
                        std::fs::read(path).unwrap_err().kind(),
                        std::io::ErrorKind::PermissionDenied
                    );
                }
            }
            operation("remove-deny");
            assert!(std::fs::read(&control).is_ok() && std::fs::read(&published).is_ok());
            assert_eq!(facts(), before);
        });
        operation("remove-deny");
        result.unwrap();
    }
}
#[test]
fn mandatory_integrity_keeps_actual_low_token_denial_and_normal_reads() {
    let (root, page) = fixture();
    let script = root.path().join("integrity.ps1");
    std::fs::write(&script, include_str!("support/mandatory-integrity.ps1")).unwrap();
    let target = root.path().join("integrity.json");
    std::fs::write(&target, b"original").unwrap();
    let probe = |operation: &str| {
        serde_json::from_slice::<Value>(
            &power(&script, &[OsStr::new(operation), target.as_os_str()]).stdout,
        )
        .unwrap()
    };
    let before = probe("setup");
    assert_eq!(before["initial_low"]["allowed"], true);
    assert_eq!(before["initial_low"]["text"], "original");
    assert_eq!(before["normal"]["allowed"], true);
    assert_eq!(before["normal"]["text"], "original");
    assert_eq!(before["initial_dacl"], before["dacl"]);
    assert_eq!(before["low"]["allowed"], false);
    assert_eq!(before["low"]["error"], 5);
    assert!(before["policy"].as_str().unwrap().contains("(ML;;NR;;;ME)"));
    publish(&page, &target);
    let after = probe("after");
    assert_eq!(after["normal"]["allowed"], true);
    assert_eq!(after["normal"]["text"], "\"complete\"\n");
    assert_eq!(after["low"]["allowed"], false);
    assert_eq!(after["low"]["error"], 5);
    assert_eq!(after["dacl"], before["dacl"]);
    assert_eq!(after["policy"], before["policy"]);
}
#[test]
fn protected_dacl_and_low_label_keep_actual_read_denial_after_replacement() {
    let (root, page) = fixture();
    let target = root.path().join("result.json");
    std::fs::write(&target, b"old-private").unwrap();
    let script = root.path().join("authorization.ps1");
    std::fs::write(&script,"param([string]$p);$ErrorActionPreference='Stop';$a=Get-Acl -LiteralPath $p;$a.SetAccessRuleProtection($true,$true);$u=[System.Security.Principal.WindowsIdentity]::GetCurrent().Name;$r=[System.Security.AccessControl.FileSystemAccessRule]::new($u,[System.Security.AccessControl.FileSystemRights]::ReadData,[System.Security.AccessControl.AccessControlType]::Deny);$a.AddAccessRule($r);Set-Acl -LiteralPath $p -AclObject $a").unwrap();
    power(&script, &[target.as_os_str()]);
    let result = std::panic::catch_unwind(|| {
        let mut command = Command::new("icacls.exe");
        command.arg(&target).args(["/setintegritylevel", "L"]);
        assert!(bounded(command).status.success());
        let label = || {
            let mut command = Command::new("icacls.exe");
            command.arg(&target);
            let output = bounded(command);
            assert!(output.status.success());
            String::from_utf8(output.stdout).unwrap()
        };
        let before = label();
        assert!(before.contains("Low Mandatory Level"));
        assert_eq!(
            std::fs::read(&target).unwrap_err().kind(),
            std::io::ErrorKind::PermissionDenied
        );
        publish(&page, &target);
        assert_eq!(
            std::fs::read(&target).unwrap_err().kind(),
            std::io::ErrorKind::PermissionDenied
        );
        assert!(label().contains("Low Mandatory Level"));
    });
    std::fs::write(&script,"param([string]$p);$ErrorActionPreference='Stop';$a=Get-Acl -LiteralPath $p;$u=[System.Security.Principal.WindowsIdentity]::GetCurrent().Name;foreach($r in @($a.Access)){if($r.IdentityReference.Value -eq $u -and $r.AccessControlType -eq 'Deny'){$a.RemoveAccessRuleSpecific($r)}};Set-Acl -LiteralPath $p -AclObject $a").unwrap();
    power(&script, &[target.as_os_str()]);
    result.unwrap();
}
#[test]
fn connected_named_pipe_producer_and_consumer_cannot_extend_executable_lifetime() {
    for reading in [true, false] {
        let (root, page) = fixture();
        let name = format!(
            "htmlcut-{}",
            root.path().file_name().unwrap().to_string_lossy()
        );
        let ready = root.path().join("pipe-ready");
        let script = root.path().join("server.ps1");
        let direction = if reading { "Out" } else { "In" };
        let write = if reading {
            "$b=[System.Text.Encoding]::UTF8.GetBytes('<h1>prefix');$p.Write($b,0,$b.Length);$p.Flush();"
        } else {
            ""
        };
        std::fs::write(&script,format!("param([string]$Name,[string]$Ready);$ErrorActionPreference='Stop';$p=[System.IO.Pipes.NamedPipeServerStream]::new($Name,[System.IO.Pipes.PipeDirection]::{direction});[System.IO.File]::WriteAllText($Ready,'ready');try{{$p.WaitForConnection();{write}Start-Sleep -Seconds 5}}finally{{$p.Dispose()}}" )).unwrap();
        let mut server = Command::new("powershell.exe")
            .args(["-NoProfile", "-File"])
            .arg(&script)
            .arg(&name)
            .arg(&ready)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let result = std::panic::catch_unwind(|| {
            let deadline = Instant::now() + Duration::from_secs(3);
            while !ready.exists() && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(5));
            }
            assert!(
                ready.exists(),
                "native named-pipe server readiness unavailable"
            );
            let pipe = format!(r"\\.\pipe\{name}");
            let mut command = Command::new(binary());
            if reading {
                command.args(["get", "h1", &pipe]);
            } else {
                std::fs::write(&page, format!("<p>{}</p>", "x".repeat(1024 * 1024))).unwrap();
                command.args(["get", "p"]).arg(&page).args(["-o", &pipe]);
            }
            command.args(["--timeout", ".1"]);
            let started = Instant::now();
            let output = bounded(command);
            assert!(started.elapsed() < Duration::from_secs(3));
            assert_eq!(output.status.code(), Some(4), "{output:?}");
            assert_eq!(
                serde_json::from_slice::<Value>(&output.stderr).unwrap()["limit"],
                "time"
            );
        });
        if server.try_wait().unwrap().is_none() {
            server.kill().unwrap();
        }
        let _ = server.wait_with_output().unwrap();
        result.unwrap();
    }
}
