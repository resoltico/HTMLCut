// SPDX-License-Identifier: MPL-2.0
//! The same executable specification against the built or unpacked binary.
use serde_json::Value;
use std::{
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    time::{Duration, Instant},
};
fn binary() -> std::ffi::OsString {
    std::env::var_os("HTMLCUT_TEST_BINARY").unwrap_or_else(|| env!("CARGO_BIN_EXE_htmlcut").into())
}
fn corpus() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance")
}
fn exit_for(code: &str) -> i32 {
    match code {
        "invalid_contract" | "invalid_selector" | "invalid_pattern" | "invalid_encoding"
        | "invalid_arguments" | "invalid_base_url" => 2,
        "count_mismatch" | "value_mismatch" | "expectation_failed" | "missing_attribute"
        | "unresolvable_url" => 3,
        "limit_exceeded" => 4,
        "input_unreadable" | "output_unwritable" | "undecodable_input" => 5,
        "internal" => 6,
        _ => panic!("unknown specification code {code}"),
    }
}
#[test]
fn every_case_runs_through_the_delivered_binary() {
    let mut checked = 0;
    for kind in ["contract", "outline"] {
        let mut paths = std::fs::read_dir(corpus().join(kind))
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect::<Vec<_>>();
        paths.sort();
        for path in paths {
            let mut command = Command::new(binary());
            if kind == "contract" {
                command
                    .args(["run", "--from-file"])
                    .arg(path.join("contract.json"));
            } else {
                command.arg("outline");
                let config: Value =
                    serde_json::from_slice(&std::fs::read(path.join("outline.json")).unwrap())
                        .unwrap();
                for option in ["select", "depth", "max_lines"] {
                    if let Some(value) = config.get(option) {
                        command.arg(format!("--{}", option.replace('_', "-"))).arg(
                            value
                                .as_str()
                                .map(String::from)
                                .unwrap_or_else(|| value.to_string()),
                        );
                    }
                }
                if config["attrs"].as_bool() == Some(true) {
                    command.arg("--attrs");
                }
            }
            command.arg(path.join("page.html"));
            if let Ok(options) = std::fs::read(path.join("options.json")) {
                let options: Value = serde_json::from_slice(&options).unwrap();
                for option in ["base_url", "encoding", "max_nodes"] {
                    if let Some(value) = options.get(option) {
                        command.arg(format!("--{}", option.replace('_', "-"))).arg(
                            value
                                .as_str()
                                .map(String::from)
                                .unwrap_or_else(|| value.to_string()),
                        );
                    }
                }
            }
            let output = command.stdin(Stdio::null()).output().unwrap();
            if path.join("error.json").exists() {
                let mut expected: Value =
                    serde_json::from_slice(&std::fs::read(path.join("error.json")).unwrap())
                        .unwrap();
                assert_eq!(
                    output.status.code(),
                    Some(exit_for(expected["error"].as_str().unwrap())),
                    "{path:?}: {output:?}"
                );
                assert!(output.stdout.is_empty(), "{path:?}: failed output leaked");
                assert_eq!(
                    output.stderr.iter().filter(|c| **c == b'\n').count(),
                    1,
                    "{path:?}"
                );
                let mut actual: Value = serde_json::from_slice(&output.stderr).unwrap();
                assert!(actual["message"].is_string());
                actual.as_object_mut().unwrap().remove("message");
                expected.as_object_mut().unwrap().remove("message");
                assert_eq!(actual, expected, "{path:?}");
            } else {
                assert!(output.status.success(), "{path:?}: {output:?}");
                assert!(output.stderr.is_empty(), "{path:?}");
                if kind == "outline" {
                    assert_eq!(
                        output.stdout,
                        std::fs::read(path.join("expected.txt")).unwrap(),
                        "{path:?}"
                    );
                } else {
                    let expected: Value =
                        serde_json::from_slice(&std::fs::read(path.join("expected.json")).unwrap())
                            .unwrap();
                    assert_eq!(
                        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
                        expected,
                        "{path:?}"
                    );
                    let mut compact = serde_json::to_vec(&expected).unwrap();
                    compact.push(b'\n');
                    assert_eq!(
                        output.stdout, compact,
                        "{path:?}: encoding is not complete compact JSON"
                    );
                }
            }
            checked += 1;
        }
    }
    assert_eq!(checked, 62);
}
fn piped(args: &[&str], bytes: &[u8]) -> Output {
    let mut child = Command::new(binary())
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(bytes).unwrap();
    child.wait_with_output().unwrap()
}
#[test]
fn guide_and_workflow_commands_are_executed_verbatim() {
    let help = Command::new(binary()).arg("--help").output().unwrap();
    assert!(help.status.success());
    assert_eq!(help.stdout, include_bytes!("../src/guide.txt"));
    assert!(help.stderr.is_empty());
    let directory = tempfile::tempdir().unwrap();
    std::fs::copy(
        corpus().join("contract/04-records/page.html"),
        directory.path().join("page.html"),
    )
    .unwrap();
    for args in [
        vec!["outline", "page.html", "--select", "article.product"],
        vec!["get", "h1", "page.html"],
        vec![
            "run",
            r#"{"expect":[{"select":"h1","equals":"All products"}],"select":"article.product","match":"all","fields":{"title":{"select":"h3 a","read":"attr:title"},"price":{"select":".price","pattern":"^£(?<value>[0-9.]+)$"},"badge":{"select":".badge","match":"optional"}}}"#,
            "page.html",
        ],
    ] {
        let output = Command::new(binary())
            .args(args)
            .current_dir(directory.path())
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        assert!(output.stderr.is_empty());
    }
}
#[test]
fn raw_strings_lists_null_and_record_rejection_have_the_declared_shape() {
    for (args, expected) in [
        (vec!["get", "h1", "--raw"], b"hello\n".as_slice()),
        (vec!["get", "p", "--all", "--raw"], b"a\nb\n".as_slice()),
        (
            vec!["run", r#"{"select":"none","match":"optional"}"#, "--raw"],
            b"".as_slice(),
        ),
    ] {
        let output = piped(&args, b"<h1>hello</h1><p>a</p><p>b</p>");
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, expected);
        assert!(output.stderr.is_empty());
    }
    let output = Command::new(binary())
        .args(["run", r#"{"fields":{}}"#, "missing-input.html", "--raw"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stderr).unwrap()["error"],
        "invalid_arguments"
    );
}
#[test]
fn named_output_preserves_refusals_replaces_success_and_accepts_input_aliases() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("page.html");
    let target = directory.path().join("result.json");
    std::fs::write(&source, b"<h1>complete</h1>").unwrap();
    std::fs::write(&target, b"old").unwrap();
    let failed = Command::new(binary())
        .args(["get", "missing"])
        .arg(&source)
        .arg("-o")
        .arg(&target)
        .output()
        .unwrap();
    assert_eq!(failed.status.code(), Some(3));
    assert!(failed.stdout.is_empty());
    assert_eq!(std::fs::read(&target).unwrap(), b"old");
    let success = Command::new(binary())
        .args(["get", "h1"])
        .arg(&source)
        .arg("-o")
        .arg(&target)
        .output()
        .unwrap();
    assert!(success.status.success());
    assert!(success.stdout.is_empty());
    assert!(success.stderr.is_empty());
    assert_eq!(std::fs::read(&target).unwrap(), b"\"complete\"\n");
    let alias = Command::new(binary())
        .args(["get", "h1"])
        .arg(&source)
        .arg("-o")
        .arg(&source)
        .output()
        .unwrap();
    assert!(alias.status.success());
    assert_eq!(std::fs::read(&source).unwrap(), b"\"complete\"\n");
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 2);
}
#[test]
fn watchdog_ends_real_upstream_selector_work_without_a_test_mode() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("deep.html");
    std::fs::write(
        &path,
        format!(
            "{}<span>x</span>{}",
            "<div>".repeat(1000),
            "</div>".repeat(1000)
        ),
    )
    .unwrap();
    let start = Instant::now();
    let selector = (0..256)
        .map(|i| format!("div:has(.missing{i})"))
        .collect::<Vec<_>>()
        .join(",");
    let output = Command::new(binary())
        .args(["get", &selector])
        .arg(&path)
        .args(["--timeout", "0.05"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(4), "{output:?}");
    assert!(output.stdout.is_empty());
    let error: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["error"], "limit_exceeded");
    assert_eq!(error["limit"], "time");
    assert!(start.elapsed() < Duration::from_secs(3));
    assert_eq!(output.stderr.iter().filter(|c| **c == b'\n').count(), 1);
}
#[test]
fn nested_has_is_refused_as_invalid_css_before_input_access() {
    let output = Command::new(binary())
        .args(["get", "body:has(div:has(.missing))", "missing-input.html"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stderr).unwrap()["error"],
        "invalid_selector"
    );
}
#[test]
fn real_input_and_output_errors_keep_paths_and_operating_system_details() {
    let directory = tempfile::tempdir().unwrap();
    let missing = directory.path().join("missing.html");
    let output = Command::new(binary())
        .args(["get", "h1"])
        .arg(&missing)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(5));
    assert!(output.stdout.is_empty());
    let error: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["error"], "input_unreadable");
    assert_eq!(error["file"], missing.to_string_lossy().as_ref());
    assert!(error["detail"].as_str().is_some_and(|s| !s.is_empty()));
    let source = directory.path().join("source.html");
    std::fs::write(&source, b"<h1>x</h1>").unwrap();
    let target = directory.path().join("directory");
    std::fs::create_dir(&target).unwrap();
    let output = Command::new(binary())
        .args(["get", "h1"])
        .arg(&source)
        .arg("-o")
        .arg(&target)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(5));
    assert!(output.stdout.is_empty());
    let error: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["error"], "output_unwritable");
    assert!(error["detail"].is_string());
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 2);
}
#[cfg(unix)]
#[test]
fn process_substitution_and_fifo_inputs_are_readable_and_input_wait_is_not_timed() {
    let output = Command::new("bash")
        .args([
            "-c",
            "\"$1\" get h1 <(printf '<h1>substitution</h1>')",
            "htmlcut-input",
        ])
        .arg(binary())
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(output.stdout, b"\"substitution\"\n");
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("snapshot.pipe");
    assert!(
        Command::new("mkfifo")
            .arg(&path)
            .status()
            .unwrap()
            .success()
    );
    let writer_path = path.clone();
    let writer = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(200));
        std::fs::write(writer_path, b"<h1>fifo</h1>").unwrap();
    });
    let output = Command::new(binary())
        .args(["get", "h1"])
        .arg(&path)
        .args(["--timeout", "0.05"])
        .output()
        .unwrap();
    writer.join().unwrap();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(output.stdout, b"\"fifo\"\n");
}
#[cfg(unix)]
#[test]
fn an_implicit_terminal_is_refused_before_reading() {
    let script = "import os,pty,subprocess,sys,json\nm,s=pty.openpty()\np=subprocess.run([sys.argv[1],'get','h1'],stdin=s,stdout=subprocess.PIPE,stderr=subprocess.PIPE,timeout=3)\nos.close(m);os.close(s)\nassert p.returncode==2 and not p.stdout\nassert json.loads(p.stderr)['error']=='invalid_arguments'\n";
    let output = Command::new("python3")
        .arg("-c")
        .arg(script)
        .arg(binary())
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
}
