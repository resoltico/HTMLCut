// SPDX-License-Identifier: MPL-2.0
//! Execute the delivered binary's native policy and supervisor negative controls.
use std::{
    process::{Command, Stdio},
    time::{Duration, Instant},
};
fn binary() -> std::ffi::OsString {
    std::env::var_os("HTMLCUT_TEST_BINARY").unwrap_or_else(|| env!("CARGO_BIN_EXE_htmlcut").into())
}
#[test]
fn native_memory_policy_refuses_an_allocation_larger_than_allowance() {
    let output = Command::new(binary())
        .env("HTMLCUT_PRIVATE_WORKER", "verify-memory")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "policy did not refuse allocation: {output:?}"
    );
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
}
#[test]
fn supervision_rejects_floods_abnormal_exit_and_invalid_frames_without_data() {
    for mode in ["stdout-flood", "stderr-flood", "abnormal", "malformed"] {
        let output = Command::new(binary())
            .env("HTMLCUT_PRIVATE_WORKER", format!("verify-{mode}"))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "supervisor falsely accepted {mode}: {output:?}"
        );
        assert!(output.stdout.is_empty());
        assert!(output.stderr.is_empty());
    }
}
#[test]
fn deadline_kills_and_reaps_a_worker_that_never_finishes() {
    let start = Instant::now();
    let output = Command::new(binary())
        .env("HTMLCUT_PRIVATE_WORKER", "verify-timeout")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(output.status.success());
    assert!(output.stdout.is_empty());
    assert_eq!(output.stderr, b"HCPROBE8\n");
    assert!(start.elapsed() >= Duration::from_secs(9));
    assert!(start.elapsed() < Duration::from_secs(20));
}
#[test]
fn historical_parser_stress_cannot_replace_an_existing_destination() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("source.html");
    let target = directory.path().join("result.json");
    std::fs::write(
        &source,
        format!("{}{}", "<b>".repeat(70_000), "<p>X</b>Y".repeat(70_000)),
    )
    .unwrap();
    std::fs::write(&target, b"old result").unwrap();
    let output = Command::new(binary())
        .args(["extract", "--file"])
        .arg(&source)
        .args(["--select", "b", "--output"])
        .arg(&target)
        .arg("--overwrite")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert_eq!(std::fs::read(target).unwrap(), b"old result");
    assert!(!String::from_utf8_lossy(&output.stderr).contains("source.html"));
}

fn await_timeout_workload(supervisor: &mut std::process::Child) {
    use std::io::Read;
    let mut diagnostics = supervisor.stderr.take().unwrap();
    let (send, receive) = std::sync::mpsc::channel();
    let reader = std::thread::spawn(move || {
        let mut marker = [0; 9];
        let result = diagnostics.read_exact(&mut marker).map(|_| marker);
        let _ = send.send(result);
    });
    let ready = receive.recv_timeout(Duration::from_secs(2));
    if !matches!(&ready, Ok(Ok(marker)) if marker == b"HCPROBE8\n") {
        let _ = supervisor.kill();
        let _ = supervisor.wait();
        let _ = reader.join();
        panic!("Private timeout workload did not acknowledge readiness: {ready:?}");
    }
    reader.join().unwrap();
}

#[cfg(unix)]
#[test]
fn killing_the_supervisor_does_not_leave_a_running_worker() {
    let mut supervisor = Command::new(binary())
        .env("HTMLCUT_PRIVATE_WORKER", "verify-timeout")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    await_timeout_workload(&mut supervisor);
    let parent = supervisor.id();
    let deadline = Instant::now() + Duration::from_secs(3);
    let worker = loop {
        let output = Command::new("ps")
            .args(["-axo", "pid=,ppid="])
            .output()
            .unwrap();
        let found = String::from_utf8_lossy(&output.stdout)
            .lines()
            .find_map(|line| {
                let mut words = line.split_whitespace();
                let pid = words.next()?.parse::<u32>().ok()?;
                let ppid = words.next()?.parse::<u32>().ok()?;
                (ppid == parent).then_some(pid)
            });
        if let Some(pid) = found {
            break pid;
        }
        assert!(Instant::now() < deadline, "private worker did not start");
        std::thread::sleep(Duration::from_millis(20));
    };
    supervisor.kill().unwrap();
    supervisor.wait().unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        let output = Command::new("ps")
            .args(["-p", &worker.to_string(), "-o", "stat="])
            .output()
            .unwrap();
        let state = String::from_utf8_lossy(&output.stdout);
        // An exited zombie awaits OS adoption/reaping; it performs no untrusted work.
        if state.trim().is_empty() || state.trim().starts_with('Z') {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "orphan worker {worker} remains running: {state}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[cfg(windows)]
#[test]
fn killing_the_supervisor_does_not_leave_a_running_worker() {
    let started = Instant::now();
    let mut supervisor = Command::new(binary())
        .env("HTMLCUT_PRIVATE_WORKER", "verify-timeout")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    await_timeout_workload(&mut supervisor);
    // One PowerShell process avoids a second startup hiding cleanup behind the
    // worker's normal ten-second timeout. Its complete lifetime is bounded below.
    let script = format!(
        r#"
$ErrorActionPreference = 'Stop'
try {{
    $supervisorId = {parent}
    $discoveryDeadline = [DateTime]::UtcNow.AddSeconds(4)
    $workerId = $null
    do {{
        $worker = Get-CimInstance -ClassName Win32_Process -Filter "ParentProcessId = $supervisorId" -OperationTimeoutSec 2 | Select-Object -First 1
        if ($null -ne $worker) {{ $workerId = [uint32]$worker.ProcessId; break }}
        Start-Sleep -Milliseconds 25
    }} while ([DateTime]::UtcNow -lt $discoveryDeadline)
    if ($null -eq $workerId) {{ throw 'Private worker did not start' }}
    # The Rust test already consumed the timeout workload ACK. Its child has
    # consumed all request input and never reads it again, ruling out pipe EOF.
    $null = Get-Process -Id $supervisorId -ErrorAction Stop
    $null = Get-Process -Id $workerId -ErrorAction Stop
    Stop-Process -Id $supervisorId -Force -ErrorAction Stop
    $cleanupDeadline = [DateTime]::UtcNow.AddSeconds(1)
    while ($null -ne (Get-Process -Id $workerId -ErrorAction SilentlyContinue)) {{
        if ([DateTime]::UtcNow -ge $cleanupDeadline) {{ throw 'Worker survived supervisor termination' }}
        Start-Sleep -Milliseconds 25
    }}
    [Console]::WriteLine($workerId)
    exit 0
}} catch {{
    [Console]::Error.WriteLine($_.Exception.Message)
    if ($null -ne $workerId) {{ Stop-Process -Id $workerId -Force -ErrorAction SilentlyContinue }}
    exit 4
}}
"#,
        parent = supervisor.id()
    );
    let inspection = Command::new("powershell.exe")
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            &script,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    let mut inspection = match inspection {
        Ok(child) => child,
        Err(error) => {
            let _ = supervisor.kill();
            let _ = supervisor.wait();
            panic!("Windows process inspection could not start: {error}");
        }
    };
    // This includes PowerShell startup and CIM latency and finishes comfortably
    // before natural worker timeout; a surviving Job must fail this control.
    let deadline = started + Duration::from_secs(7);
    let inspected = loop {
        match inspection.try_wait() {
            Ok(Some(_)) => break true,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(20)),
            _ => {
                let _ = inspection.kill();
                break false;
            }
        }
    };
    let result = inspection.wait_with_output();
    // Reap the supervisor killed by Stop-Process; on failed inspection, kill it
    // ourselves so a failed assertion also cleans up the Job and its worker.
    let _ = supervisor.kill();
    let reaped = supervisor.wait();
    assert!(
        inspected,
        "Windows process inspection exceeded its seven-second bound"
    );
    let output = result.unwrap();
    assert!(
        output.status.success(),
        "Windows supervisor-death control failed: {output:?}"
    );
    assert!(
        output.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .trim()
            .parse::<u32>()
            .is_ok(),
        "No live worker PID was verified: {output:?}"
    );
    assert!(
        reaped.is_ok(),
        "Killed supervisor was not reaped: {reaped:?}"
    );
    assert!(
        started.elapsed() < Duration::from_secs(8),
        "Cleanup overlapped natural worker timeout"
    );
}

#[test]
fn maximum_size_snapshot_remains_usable_under_native_policy() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("large.html");
    let prefix = b"<h1>accepted</h1><script>";
    let suffix = b"</script>";
    let mut bytes = prefix.to_vec();
    bytes.resize(htmlcut_core::MAX_SOURCE_BYTES - suffix.len(), b'x');
    bytes.extend_from_slice(suffix);
    std::fs::write(&source, &bytes).unwrap();
    drop(bytes);
    let output = Command::new(binary())
        .args(["extract", "--file"])
        .arg(source)
        .args(["--select", "h1"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "maximum admitted snapshot refused: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"[\"accepted\"]\n");
}

#[test]
fn native_policy_refuses_touched_memory_pressure_while_supervisor_survives() {
    let output = Command::new(binary())
        .env("HTMLCUT_PRIVATE_WORKER", "verify-pressure")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "native memory pressure did not prove refusal: {output:?}"
    );
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
}

#[test]
fn deadline_breaks_a_blocked_snapshot_pipe_and_reaps_the_worker() {
    let start = Instant::now();
    let output = Command::new(binary())
        .env("HTMLCUT_PRIVATE_WORKER", "verify-blocked-input")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "blocked source pump was not recovered: {output:?}"
    );
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
    assert!(start.elapsed() >= Duration::from_secs(9));
    assert!(start.elapsed() < Duration::from_secs(20));
}

#[test]
fn output_aliases_are_compared_by_physical_identity_not_content() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("source.html");
    let alias = directory.path().join("alias.html");
    let distinct = directory.path().join("distinct.html");
    let html = b"<p>complete</p>";
    std::fs::write(&source, html).unwrap();
    std::fs::hard_link(&source, &alias).unwrap();
    std::fs::write(&distinct, html).unwrap();
    let refusal = Command::new(binary())
        .args(["extract", "--file"])
        .arg(&source)
        .args(["--select", "p", "--output"])
        .arg(&alias)
        .arg("--overwrite")
        .output()
        .unwrap();
    assert_eq!(refusal.status.code(), Some(2));
    assert!(refusal.stdout.is_empty());
    assert_eq!(std::fs::read(&source).unwrap(), html);
    assert_eq!(std::fs::read(&alias).unwrap(), html);
    let error: serde_json::Value = serde_json::from_slice(&refusal.stderr).unwrap();
    assert_eq!(error["code"], "invalid_options");
    assert!(!String::from_utf8_lossy(&refusal.stderr).contains("source.html"));
    let success = Command::new(binary())
        .args(["extract", "--file"])
        .arg(&source)
        .args(["--select", "p", "--output"])
        .arg(&distinct)
        .arg("--overwrite")
        .output()
        .unwrap();
    assert!(success.status.success());
    assert_eq!(std::fs::read(distinct).unwrap(), b"[\"complete\"]\n");
    assert_eq!(std::fs::read(source).unwrap(), html);
}

#[test]
fn regular_file_stdin_source_and_plan_refuse_direct_and_hardlink_output_aliases() {
    for plan_stdin in [false, true] {
        for hardlink in [false, true] {
            let directory = tempfile::tempdir().unwrap();
            let html = directory.path().join("source.html");
            let plan = directory.path().join("query.json");
            std::fs::write(&html, b"<p>owned</p>").unwrap();
            std::fs::write(&plan, br#"{"version":8,"select":"p"}"#).unwrap();
            let input = if plan_stdin { &plan } else { &html };
            let original = std::fs::read(input).unwrap();
            let alias = directory.path().join("alias");
            let target = if hardlink {
                std::fs::hard_link(input, &alias).unwrap();
                &alias
            } else {
                input
            };
            let mut command = Command::new(binary());
            command.arg("extract");
            if plan_stdin {
                command.arg("--file").arg(&html).args(["--plan", "-"]);
            } else {
                command.args(["--stdin", "--select", "p"]);
            }
            let output = command
                .arg("--output")
                .arg(target)
                .arg("--overwrite")
                .stdin(Stdio::from(std::fs::File::open(input).unwrap()))
                .output()
                .unwrap();
            assert_eq!(
                output.status.code(),
                Some(2),
                "plan_stdin={plan_stdin}, hardlink={hardlink}: {output:?}"
            );
            assert!(output.stdout.is_empty());
            assert_eq!(std::fs::read(input).unwrap(), original);
            assert_eq!(std::fs::read(target).unwrap(), original);
            let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
            assert_eq!(error["code"], "invalid_options");
            for name in ["source.html", "query.json", "alias"] {
                assert!(!String::from_utf8_lossy(&output.stderr).contains(name));
            }
        }
    }
}

#[test]
fn regular_file_stdin_supports_equal_content_distinct_targets() {
    for plan_stdin in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let html = directory.path().join("source.html");
        let plan = directory.path().join("query.json");
        let target = directory.path().join("output");
        std::fs::write(&html, b"<p>owned</p>").unwrap();
        std::fs::write(&plan, br#"{"version":8,"select":"p"}"#).unwrap();
        let input = if plan_stdin { &plan } else { &html };
        let original = std::fs::read(input).unwrap();
        std::fs::write(&target, &original).unwrap();
        let mut command = Command::new(binary());
        command.arg("extract");
        if plan_stdin {
            command.arg("--file").arg(&html).args(["--plan", "-"]);
        } else {
            command.args(["--stdin", "--select", "p"]);
        }
        let output = command
            .arg("--output")
            .arg(&target)
            .arg("--overwrite")
            .stdin(Stdio::from(std::fs::File::open(input).unwrap()))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "plan_stdin={plan_stdin}: {output:?}"
        );
        assert!(output.stdout.is_empty());
        assert!(output.stderr.is_empty());
        assert_eq!(std::fs::read(input).unwrap(), original);
        assert_eq!(std::fs::read(target).unwrap(), b"[\"owned\"]\n");
    }
}

#[test]
fn grammar_refusal_does_not_acquire_an_unused_native_stdin_handle() {
    let directory = tempfile::tempdir().unwrap();
    let sink = directory.path().join("write-only.txt");
    let output = Command::new(binary())
        .args(["extract", "--stdin", "--select", "["])
        .stdin(Stdio::from(std::fs::File::create(&sink).unwrap()))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["code"], "invalid_selector");
    for arguments in [vec!["--help"], vec!["schema", "htmlcut.extraction.plan"]] {
        let output = Command::new(binary())
            .args(arguments)
            .stdin(Stdio::from(std::fs::File::create(&sink).unwrap()))
            .output()
            .unwrap();
        assert!(output.status.success());
        assert!(!output.stdout.is_empty());
        assert!(output.stderr.is_empty());
    }
}

#[test]
fn stdin_plan_grammar_is_checked_before_alias_or_destination_admission() {
    for (plan_text, code) in [
        (r#"{"version":8,"select":"["}"#, "invalid_selector"),
        (
            r#"{"version":8,"select":"p","expect":[{"select":"p","pattern":"["}]}"#,
            "invalid_regex",
        ),
    ] {
        for alias in [false, true] {
            let directory = tempfile::tempdir().unwrap();
            let source = directory.path().join("source.html");
            let plan = directory.path().join("plan.json");
            std::fs::write(&source, b"<p>owned</p>").unwrap();
            std::fs::write(&plan, plan_text).unwrap();
            let target = if alias {
                plan.clone()
            } else {
                directory.path().join("missing-directory/output")
            };
            let output = Command::new(binary())
                .arg("extract")
                .arg("--file")
                .arg(&source)
                .args(["--plan", "-", "--output"])
                .arg(target)
                .arg("--overwrite")
                .stdin(Stdio::from(std::fs::File::open(&plan).unwrap()))
                .output()
                .unwrap();
            assert_eq!(output.status.code(), Some(2));
            assert!(output.stdout.is_empty());
            let error: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
            assert_eq!(error["code"], code);
            assert_eq!(std::fs::read(plan).unwrap(), plan_text.as_bytes());
            assert_eq!(std::fs::read(source).unwrap(), b"<p>owned</p>");
        }
    }
}
