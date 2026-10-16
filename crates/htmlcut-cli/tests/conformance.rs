// SPDX-License-Identifier: MPL-2.0
//! The same executable specification against the built or unpacked binary.
use serde_json::Value;
use std::{
    io::Write,
    path::{Path, PathBuf},
    process::{Child, Command, Output, Stdio},
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
        | "invalid_request" | "invalid_base_url" => 2,
        "count_mismatch"
        | "value_mismatch"
        | "expectation_failed"
        | "missing_attribute"
        | "unresolvable_url"
        | "unrepresentable_output" => 3,
        "limit_exceeded" => 4,
        "input_unreadable" | "output_unwritable" | "undecodable_input" => 5,
        "internal" => 6,
        _ => panic!("unknown specification code {code}"),
    }
}
#[test]
fn every_case_runs_through_the_delivered_binary() {
    let checked = run_cases(&corpus());
    println!("HTMLCUT_CONFORMANCE_CASES={checked}");
}
fn run_cases(root: &Path) -> usize {
    let mut checked = 0;
    let mut discovered = 0;
    for kind in ["contract", "outline"] {
        let mut paths = std::fs::read_dir(root.join(kind))
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect::<Vec<_>>();
        paths.retain(|path| path.is_dir());
        discovered += paths.len();
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
            let options: Value = std::fs::read(path.join("options.json"))
                .ok()
                .map(|bytes| serde_json::from_slice(&bytes).unwrap())
                .unwrap_or_default();
            if options["raw"] == true {
                command.arg("--raw");
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
                if options["partial_error_facts"] == true {
                    for (key, value) in expected.as_object().unwrap() {
                        assert_eq!(actual.get(key), Some(value), "{path:?}: {key}");
                    }
                } else {
                    assert_eq!(actual, expected, "{path:?}");
                }
            } else {
                assert!(output.status.success(), "{path:?}: {output:?}");
                assert!(output.stderr.is_empty(), "{path:?}");
                if kind == "outline" {
                    assert_eq!(
                        output.stdout,
                        std::fs::read(path.join("expected.txt")).unwrap(),
                        "{path:?}"
                    );
                } else if path.join("expected.raw").exists() {
                    assert_eq!(
                        output.stdout,
                        std::fs::read(path.join("expected.raw")).unwrap(),
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
    assert!(discovered > 0, "the conformance corpus is empty");
    assert_eq!(checked, discovered);
    checked
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
fn bounded_output(mut child: Child, timeout: Duration) -> Output {
    let deadline = Instant::now() + timeout;
    loop {
        if child.try_wait().unwrap().is_some() {
            return child.wait_with_output().unwrap();
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let output = child.wait_with_output().unwrap();
            panic!("pipe test process exceeded its deadline: {output:?}");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
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
        .args([
            "run",
            r#"{"fields":{"title":"h1"}}"#,
            "missing-input.html",
            "--raw",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3));
    assert!(output.stdout.is_empty());
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stderr).unwrap()["error"],
        "unrepresentable_output"
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
fn process_substitution_and_fifo_inputs_are_readable_within_the_deadline() {
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
        .args(["--timeout", "1"])
        .output()
        .unwrap();
    writer.join().unwrap();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(output.stdout, b"\"fifo\"\n");
}
#[cfg(unix)]
#[test]
#[allow(unsafe_code)]
fn an_implicit_terminal_is_refused_before_reading() {
    use std::os::fd::{FromRawFd, OwnedFd};
    let mut master = 0;
    let mut slave = 0;
    // openpty returns two independently owned descriptors on success.
    assert_eq!(
        unsafe {
            libc::openpty(
                &mut master,
                &mut slave,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        },
        0
    );
    let _master = unsafe { OwnedFd::from_raw_fd(master) };
    let slave = unsafe { OwnedFd::from_raw_fd(slave) };
    let child = Command::new(binary())
        .args(["get", "h1"])
        .stdin(Stdio::from(slave))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let output = bounded_output(child, Duration::from_secs(3));
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stderr).unwrap()["error"],
        "invalid_request"
    );
}

#[test]
fn first_contact_reaches_the_guide_and_contract_files_get_a_hint() {
    for argument in ["-h", "help"] {
        let output = Command::new(binary()).arg(argument).output().unwrap();
        assert!(output.status.success());
        assert_eq!(output.stdout, include_bytes!("../src/guide.txt"));
        assert!(output.stderr.is_empty());
    }
    let output = Command::new(binary()).arg("-V").output().unwrap();
    assert!(output.status.success());
    assert_eq!(
        output.stdout,
        concat!("htmlcut ", env!("CARGO_PKG_VERSION"), "\n").as_bytes()
    );
    for arguments in [vec![], vec!["--unknown-option"]] {
        let output = Command::new(binary()).args(arguments).output().unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        let error: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(error["error"], "invalid_request");
        assert!(
            error["message"]
                .as_str()
                .unwrap()
                .ends_with("Run htmlcut --help for the guide.")
        );
        assert_eq!(output.stderr.iter().filter(|c| **c == b'\n').count(), 1);
    }
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("contract.json");
    std::fs::write(&file, b"\"h1\"").unwrap();
    let output = Command::new(binary())
        .arg("run")
        .arg(file)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let error: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["error"], "invalid_contract");
    assert!(error["message"].as_str().unwrap().contains("--from-file"));
}

#[test]
fn a_consumer_closing_stdout_is_silent_success() {
    use std::io::Read;
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("large.html");
    std::fs::write(&file, format!("<body>{}</body>", "x".repeat(2_000_000))).unwrap();
    let destinations = [
        None,
        Some("-"),
        #[cfg(unix)]
        Some("/dev/stdout"),
        #[cfg(unix)]
        Some("/dev/fd/1"),
        #[cfg(target_os = "linux")]
        Some("/proc/self/fd/1"),
    ];
    for destination in destinations {
        let mut command = Command::new(binary());
        command.args(["get", "body", "--raw"]).arg(&file);
        if let Some(path) = destination {
            command.args(["-o", path]);
        }
        let mut child = command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut pipe = child.stdout.take().unwrap();
        let mut byte = [0];
        pipe.read_exact(&mut byte).unwrap();
        assert_eq!(byte, [b'x']);
        drop(pipe);
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success(), "{destination:?}: {output:?}");
        assert!(output.stderr.is_empty(), "{output:?}");
    }
}

#[cfg(unix)]
#[test]
fn named_output_obeys_umask_and_preserves_existing_permission_bits() {
    use std::os::unix::fs::PermissionsExt;
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("page.html");
    std::fs::write(&source, b"<h1>complete</h1>").unwrap();
    for mask in ["022", "077"] {
        let target = directory.path().join(mask);
        let output = Command::new("bash")
            .args([
                "-c",
                "umask \"$1\"; exec \"$2\" get h1 \"$3\" -o \"$4\"",
                "htmlcut-permissions",
                mask,
            ])
            .arg(binary())
            .arg(&source)
            .arg(&target)
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        let expected = 0o666 & !u32::from_str_radix(mask, 8).unwrap();
        assert_eq!(
            target.metadata().unwrap().permissions().mode() & 0o7777,
            expected
        );
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o640)).unwrap();
        let output = Command::new(binary())
            .args(["get", "h1"])
            .arg(&source)
            .arg("-o")
            .arg(&target)
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        assert_eq!(
            target.metadata().unwrap().permissions().mode() & 0o7777,
            0o640
        );
        let bytes = std::fs::read(&target).unwrap();
        let output = Command::new(binary())
            .args(["get", "missing"])
            .arg(&source)
            .arg("-o")
            .arg(&target)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(3));
        assert_eq!(std::fs::read(&target).unwrap(), bytes);
        assert_eq!(
            target.metadata().unwrap().permissions().mode() & 0o7777,
            0o640
        );
    }
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 3);
}

#[test]
#[ignore = "needs the shipped binary; run by the package smoke"]
fn positional_predicates_reuse_sibling_indexes_within_the_deadline() {
    let performance_binary = std::env::var_os("HTMLCUT_TEST_BINARY").expect(
        "HTMLCUT_TEST_BINARY must name the shipped binary for the package performance test",
    );
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("siblings.html");
    let mut html = String::from("<main>");
    for n in 0..60_000 {
        html.push_str(&format!("<p>{n}</p>"));
    }
    html.push_str("</main>");
    std::fs::write(&file, html).unwrap();
    let main = Command::new(&performance_binary)
        .args(["get", "p:nth-last-child(3n)", "--all", "--timeout", "5"])
        .arg(&file)
        .output()
        .unwrap();
    assert!(main.status.success(), "{main:?}");
    let start = Instant::now();
    let predicate = Command::new(&performance_binary)
        .args([
            "run",
            r#"{"select":"p","match":"all","where":[{"select":":scope:nth-last-child(3n)"}]}"#,
            "--timeout",
            "5",
        ])
        .arg(&file)
        .output()
        .unwrap();
    assert!(predicate.status.success(), "{predicate:?}");
    assert!(start.elapsed() < Duration::from_secs(5));
    assert_eq!(predicate.stdout, main.stdout);
    let values: Vec<String> = serde_json::from_slice(&predicate.stdout).unwrap();
    assert_eq!(
        values,
        (0..60_000)
            .filter(|n| (60_000 - n) % 3 == 0)
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
    );
}

#[test]
fn corpus_runner_refuses_empty_or_incomplete_corpora() {
    let directory = tempfile::tempdir().unwrap();
    for kind in ["contract", "outline"] {
        std::fs::create_dir(directory.path().join(kind)).unwrap();
    }
    assert!(std::panic::catch_unwind(|| run_cases(directory.path())).is_err());
    let case = directory.path().join("contract/incomplete");
    std::fs::create_dir(&case).unwrap();
    std::fs::write(case.join("page.html"), b"<h1>x</h1>").unwrap();
    std::fs::write(case.join("contract.json"), b"\"h1\"").unwrap();
    assert!(std::panic::catch_unwind(|| run_cases(directory.path())).is_err());
}

#[test]
fn content_type_meta_with_incomplete_charset_parses_successfully() {
    let directory = tempfile::tempdir().unwrap();
    let page = directory.path().join("page.html");
    let destination = directory.path().join("result.json");
    std::fs::write(
        &page,
        b"<meta http-equiv=content-type content='text/html; charset'><p>ok</p>",
    )
    .unwrap();
    std::fs::write(&destination, b"previous complete result").unwrap();
    let output = Command::new(binary())
        .args(["get", "p"])
        .arg(&page)
        .arg("-o")
        .arg(&destination)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
    assert_eq!(std::fs::read(&destination).unwrap(), b"\"ok\"\n");
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 2);
}

#[test]
fn output_dash_is_stdout_and_directories_refuse() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("page.html");
    std::fs::write(&source, b"<h1>complete</h1>").unwrap();
    let output = Command::new(binary())
        .args(["get", "h1"])
        .arg(&source)
        .args(["-o", "-"])
        .current_dir(directory.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(output.stdout, b"\"complete\"\n");
    assert!(!directory.path().join("-").exists());
    let output = Command::new(binary())
        .args(["get", "h1"])
        .arg(&source)
        .arg("-o")
        .arg(directory.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(5));
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stderr).unwrap()["error"],
        "output_unwritable"
    );
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
}

#[cfg(unix)]
#[test]
fn output_links_follow_targets_and_permission_checks_preserve_objects() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("page.html");
    let target = directory.path().join("target");
    let link = directory.path().join("link");
    std::fs::write(&source, b"<h1>complete</h1>").unwrap();
    std::fs::write(&target, b"old").unwrap();
    std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o6754)).unwrap();
    symlink("target", &link).unwrap();
    let publish = |selector: &str, path: &Path| {
        Command::new(binary())
            .args(["get", selector])
            .arg(&source)
            .arg("-o")
            .arg(path)
            .output()
            .unwrap()
    };
    let output = publish("h1", &link);
    assert!(output.status.success(), "{output:?}");
    assert!(link.symlink_metadata().unwrap().file_type().is_symlink());
    assert_eq!(std::fs::read(&target).unwrap(), b"\"complete\"\n");
    assert_eq!(
        target.metadata().unwrap().permissions().mode() & 0o7777,
        0o754
    );
    let absent = directory.path().join("absent");
    let dangling = directory.path().join("dangling");
    let chain = directory.path().join("chain");
    symlink("absent", &dangling).unwrap();
    symlink("dangling", &chain).unwrap();
    assert!(publish("h1", &chain).status.success());
    assert!(
        dangling
            .symlink_metadata()
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(std::fs::read(&absent).unwrap(), b"\"complete\"\n");
    std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o444)).unwrap();
    let bytes = std::fs::read(&target).unwrap();
    let output = publish("h1", &link);
    assert_eq!(output.status.code(), Some(5));
    assert_eq!(std::fs::read(&target).unwrap(), bytes);
    assert!(link.symlink_metadata().unwrap().file_type().is_symlink());
    for path in [&link, &dangling, &chain] {
        assert_eq!(publish("missing", path).status.code(), Some(3));
        assert!(path.symlink_metadata().unwrap().file_type().is_symlink());
    }
    let cycle_a = directory.path().join("cycle-a");
    let cycle_b = directory.path().join("cycle-b");
    symlink("cycle-b", &cycle_a).unwrap();
    symlink("cycle-a", &cycle_b).unwrap();
    assert_eq!(publish("h1", &cycle_a).status.code(), Some(5));
    let missing_directory = directory.path().join("missing-directory/");
    assert_eq!(publish("h1", &missing_directory).status.code(), Some(5));
    assert!(!missing_directory.exists());
    let trailing_link = directory.path().join("trailing-link");
    symlink("missing-directory/", &trailing_link).unwrap();
    assert_eq!(publish("h1", &trailing_link).status.code(), Some(5));
    assert!(!missing_directory.exists());
    let real = directory.path().join("real");
    std::fs::create_dir(&real).unwrap();
    std::fs::create_dir(real.join("child")).unwrap();
    let parent_link = directory.path().join("parent-link");
    symlink("real/child", &parent_link).unwrap();
    let parent_target = directory.path().join("parent-target");
    symlink("parent-link/../new", &parent_target).unwrap();
    assert!(publish("h1", &parent_target).status.success());
    assert!(real.join("new").exists());
    assert!(!directory.path().join("new").exists());
    let hard_link = directory.path().join("hard-link");
    std::fs::hard_link(&absent, &hard_link).unwrap();
    std::fs::write(&source, b"<h1>changed</h1>").unwrap();
    assert!(publish("h1", &absent).status.success());
    assert_eq!(std::fs::read(&hard_link).unwrap(), b"\"complete\"\n");
    assert_eq!(std::fs::read(&absent).unwrap(), b"\"changed\"\n");
    assert!(
        !std::fs::read_dir(directory.path()).unwrap().any(|e| e
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".tmp"))
    );
}

#[cfg(unix)]
#[test]
fn output_pipes_devices_and_descriptor_aliases_are_written_directly() {
    use std::io::Read;
    use std::os::unix::fs::{FileTypeExt, symlink};
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("page.html");
    let pipe = directory.path().join("output.pipe");
    std::fs::write(&source, b"<h1>complete</h1>").unwrap();
    assert!(
        Command::new("mkfifo")
            .arg(&pipe)
            .status()
            .unwrap()
            .success()
    );
    // Refusal must not even open the FIFO: there is no reader to unblock it.
    let mut refusal = Command::new(binary())
        .args(["get", "missing"])
        .arg(&source)
        .arg("-o")
        .arg(&pipe)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        if let Some(status) = refusal.try_wait().unwrap() {
            assert_eq!(status.code(), Some(3));
            break;
        }
        if Instant::now() >= deadline {
            refusal.kill().unwrap();
            refusal.wait().unwrap();
            panic!("refusal opened its output FIFO");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let mut reader = Command::new("cat")
        .arg(&pipe)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let publication = std::panic::catch_unwind(|| {
        let producer = Command::new(binary())
            .args(["get", "h1"])
            .arg(&source)
            .arg("-o")
            .arg(&pipe)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let output = bounded_output(producer, Duration::from_secs(10));
        assert!(output.status.success(), "{output:?}");
    });
    if publication.is_err() {
        let _ = reader.kill();
    }
    let received = bounded_output(reader, Duration::from_secs(10));
    publication.unwrap();
    assert!(received.status.success(), "{received:?}");
    assert_eq!(received.stdout, b"\"complete\"\n");
    assert!(pipe.symlink_metadata().unwrap().file_type().is_fifo());
    let output = Command::new(binary())
        .args(["get", "h1"])
        .arg(&source)
        .args(["-o", "/dev/null"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert!(
        Path::new("/dev/null")
            .metadata()
            .unwrap()
            .file_type()
            .is_char_device()
    );
    let alias = directory.path().join("stdout-link");
    symlink("/dev/stdout", &alias).unwrap();
    for path in [Path::new("/dev/stdout"), alias.as_path()] {
        let target = directory.path().join("redirected");
        let writer = std::fs::File::create(&target).unwrap();
        let mut observer = writer.try_clone().unwrap();
        let output = Command::new(binary())
            .args(["get", "h1"])
            .arg(&source)
            .arg("-o")
            .arg(path)
            .stdout(Stdio::from(writer))
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        // The original descriptor must see the data, not just a replaced name.
        use std::io::{Seek, SeekFrom};
        observer.seek(SeekFrom::Start(0)).unwrap();
        let mut data = Vec::new();
        // The write-only observer has no read permission; compare its size and the name.
        assert_eq!(observer.metadata().unwrap().len(), 11);
        std::fs::File::open(&target)
            .unwrap()
            .read_to_end(&mut data)
            .unwrap();
        assert_eq!(data, b"\"complete\"\n");
    }
    assert!(alias.symlink_metadata().unwrap().file_type().is_symlink());
}

#[cfg(unix)]
#[test]
fn descriptor_aliases_preserve_inherited_cursors_append_and_access_modes() {
    use std::io::{Seek, SeekFrom};
    use std::os::unix::fs::symlink;
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("page.html");
    std::fs::write(&source, b"<h1>complete</h1>").unwrap();
    let paths = [
        "/dev/stdout",
        "/dev/fd/1",
        #[cfg(target_os = "linux")]
        "/proc/self/fd/1",
        #[cfg(target_os = "linux")]
        "/proc/thread-self/fd/1",
    ];
    let mut aliases = paths.iter().map(PathBuf::from).collect::<Vec<_>>();
    for (index, target) in paths.iter().enumerate() {
        let alias = directory.path().join(format!("stream-link-{index}"));
        symlink(target, &alias).unwrap();
        aliases.push(alias);
    }
    let parent_alias = directory.path().join("descriptor-parent");
    symlink("/dev/fd", &parent_alias).unwrap();
    aliases.push(parent_alias.join("1"));
    // Both controls use the same inherited descriptor as the named aliases.
    let destinations = std::iter::once(None)
        .chain(std::iter::once(Some(PathBuf::from("-"))))
        .chain(aliases.into_iter().map(Some));
    for destination in destinations {
        for append in [false, true] {
            let target = directory.path().join("redirected");
            std::fs::write(&target, b"prefix|XXXXXXXXXXXXXXXX").unwrap();
            let mut writer = std::fs::OpenOptions::new()
                .write(true)
                .append(append)
                .open(&target)
                .unwrap();
            writer.seek(SeekFrom::Start(7)).unwrap();
            let mut observer = writer.try_clone().unwrap();
            let mut command = Command::new(binary());
            command.args(["get", "h1"]).arg(&source);
            if let Some(path) = &destination {
                command.arg("-o").arg(path);
            }
            let output = command.stdout(Stdio::from(writer)).output().unwrap();
            assert!(
                output.status.success(),
                "{destination:?} {append}: {output:?}"
            );
            assert!(output.stderr.is_empty(), "{output:?}");
            let expected: &[u8] = if append {
                b"prefix|XXXXXXXXXXXXXXXX\"complete\"\n"
            } else {
                b"prefix|\"complete\"\nXXXXX"
            };
            assert_eq!(
                std::fs::read(&target).unwrap(),
                expected,
                "{destination:?} {append}"
            );
            assert_eq!(
                observer.stream_position().unwrap(),
                if append { 34 } else { 18 }
            );
        }
    }
    for (path, stream) in [
        ("/dev/stdin", 0),
        ("/dev/fd/0", 0),
        ("/dev/stderr", 2),
        ("/dev/fd/2", 2),
    ] {
        let target = directory.path().join("other-stream");
        std::fs::write(&target, b"prefix|").unwrap();
        let mut writer = std::fs::OpenOptions::new()
            .write(true)
            .open(&target)
            .unwrap();
        writer.seek(SeekFrom::Start(7)).unwrap();
        let mut command = Command::new(binary());
        command.args(["get", "h1"]).arg(&source).args(["-o", path]);
        if stream == 0 {
            command.stdin(Stdio::from(writer));
        } else {
            command.stderr(Stdio::from(writer));
        }
        assert!(command.output().unwrap().status.success());
        assert_eq!(std::fs::read(&target).unwrap(), b"prefix|\"complete\"\n");
    }
    // A writable pathname must not grant extra access to a read-only inherited FD.
    for path in ["/dev/stdin", "/dev/fd/0"] {
        let target = directory.path().join("readonly-stream");
        std::fs::write(&target, b"preserve").unwrap();
        let output = Command::new(binary())
            .args(["get", "h1"])
            .arg(&source)
            .args(["-o", path])
            .stdin(Stdio::from(std::fs::File::open(&target).unwrap()))
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(5), "{output:?}");
        let error: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(error["error"], "output_unwritable");
        assert_eq!(error["file"], path);
        assert!(
            error["detail"]
                .as_str()
                .is_some_and(|detail| !detail.is_empty())
        );
        assert!(output.stdout.is_empty());
        assert_eq!(std::fs::read(&target).unwrap(), b"preserve");
    }
    let invalid_paths = [
        "/dev/stdout/",
        "/dev/stdout/.",
        "/dev/fd/1/",
        "/dev/fd/1/.",
        "/dev/stdout/./.",
        "/dev/fd/1/.///",
        #[cfg(target_os = "linux")]
        "/proc/self/fd/1/",
        #[cfg(target_os = "linux")]
        "/proc/self/fd/1/.",
    ];
    for (index, spelling) in invalid_paths.iter().enumerate() {
        let link = directory
            .path()
            .join(format!("directory-stream-link-{index}"));
        symlink(spelling, &link).unwrap();
        for path in [Path::new(spelling), link.as_path()] {
            let target = directory.path().join("directory-stream");
            std::fs::write(&target, b"prefix|preserve").unwrap();
            let mut writer = std::fs::OpenOptions::new()
                .write(true)
                .open(&target)
                .unwrap();
            writer.seek(SeekFrom::Start(7)).unwrap();
            let mut observer = writer.try_clone().unwrap();
            let output = Command::new(binary())
                .args(["get", "h1"])
                .arg(&source)
                .arg("-o")
                .arg(path)
                .stdout(Stdio::from(writer))
                .output()
                .unwrap();
            assert_eq!(output.status.code(), Some(5), "{path:?}: {output:?}");
            let error: Value = serde_json::from_slice(&output.stderr).unwrap();
            assert_eq!(error["error"], "output_unwritable");
            assert_eq!(error["file"], path.to_string_lossy().as_ref());
            assert!(output.stdout.is_empty());
            assert_eq!(std::fs::read(&target).unwrap(), b"prefix|preserve");
            assert_eq!(observer.stream_position().unwrap(), 7);
        }
        assert!(link.symlink_metadata().unwrap().file_type().is_symlink());
    }
}

#[test]
fn dash_is_a_stream_marker_only_when_it_is_the_entire_path() {
    use std::io::Seek;
    let directory = tempfile::tempdir().unwrap();
    std::fs::create_dir(directory.path().join("-")).unwrap();
    let source = directory.path().join("page.html");
    std::fs::write(&source, b"<h1>complete</h1>").unwrap();
    for spelling in ["-/", "-/.", "-/./.", "./-"] {
        let reader = std::fs::File::open(&source).unwrap();
        let mut observer = reader.try_clone().unwrap();
        let output = Command::new(binary())
            .args(["get", "h1", "--", spelling])
            .current_dir(directory.path())
            .stdin(Stdio::from(reader))
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(5), "{output:?}");
        let error: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(error["error"], "input_unreadable");
        assert_eq!(error["file"], spelling);
        assert!(output.stdout.is_empty());
        assert_eq!(observer.stream_position().unwrap(), 0);
        let output = Command::new(binary())
            .args(["get", "h1"])
            .arg(&source)
            .arg(format!("--output={spelling}"))
            .current_dir(directory.path())
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(5), "{output:?}");
        let error: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(error["error"], "output_unwritable");
        assert_eq!(error["file"], spelling);
        assert!(output.stdout.is_empty());
        assert!(directory.path().join("-").is_dir());
    }
}

#[cfg(unix)]
#[test]
#[allow(unsafe_code)]
fn numeric_descriptor_aliases_use_the_inherited_descriptor_and_refuse_closed_handles() {
    use std::{
        fs::OpenOptions,
        io::{Seek, SeekFrom},
        os::{fd::AsRawFd, unix::process::CommandExt},
    };
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("page.html");
    std::fs::write(&source, b"<h1>complete</h1>").unwrap();
    let target = root.path().join("output");
    for append in [false, true] {
        std::fs::write(&target, b"prefix|XXXXXXXXXXXXXXXX").unwrap();
        let mut stream = OpenOptions::new()
            .read(true)
            .write(true)
            .append(append)
            .open(&target)
            .unwrap();
        let fd = stream.as_raw_fd();
        assert!(fd > 2);
        let mut aliases = vec![format!("/dev/fd/{fd}")];
        if cfg!(target_os = "linux") {
            aliases.extend([
                format!("/proc/self/fd/{fd}"),
                format!("/proc/thread-self/fd/{fd}"),
            ]);
        }
        for alias in aliases {
            std::fs::write(&target, b"prefix|XXXXXXXXXXXXXXXX").unwrap();
            stream.seek(SeekFrom::Start(7)).unwrap();
            let mut invocation = Command::new(binary());
            invocation
                .args(["get", "h1"])
                .arg(&source)
                .args(["-o", &alias])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
            // Preserve just this owned descriptor across exec. fcntl is async-signal-safe.
            unsafe {
                invocation.pre_exec(move || {
                    if libc::fcntl(fd, libc::F_SETFD, 0) < 0 {
                        return Err(std::io::Error::last_os_error());
                    }
                    Ok(())
                });
            }
            let output = bounded_output(invocation.spawn().unwrap(), Duration::from_secs(10));
            assert!(output.status.success(), "{output:?}");
            assert!(output.stdout.is_empty() && output.stderr.is_empty());
            assert_eq!(
                std::fs::read(&target).unwrap(),
                if append {
                    &b"prefix|XXXXXXXXXXXXXXXX\"complete\"\n"[..]
                } else {
                    &b"prefix|\"complete\"\nXXXXX"[..]
                }
            );
            assert_eq!(
                stream.stream_position().unwrap(),
                if append { 34 } else { 18 }
            );
        }
        drop(stream);
        let output = Command::new(binary())
            .args(["get", "h1"])
            .arg(&source)
            .args(["-o", &format!("/dev/fd/{fd}")])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(5), "{output:?}");
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("Bad file descriptor"));
    }
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::MetadataExt;
        for alias in [
            "/proc/self/fd/01",
            "/dev/fd/0001",
            "/dev/fd/99999999999999999999",
        ] {
            let output = Command::new(binary())
                .args(["get", "h1"])
                .arg(&source)
                .args(["-o", alias])
                .output()
                .unwrap();
            assert_eq!(output.status.code(), Some(5));
            assert!(output.stdout.is_empty());
        }
        std::fs::write(&target, b"prefix|XXXXXXXXXXXXXXXX").unwrap();
        let mut stream = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&target)
            .unwrap();
        stream.seek(SeekFrom::Start(7)).unwrap();
        let inode = stream.metadata().unwrap().ino();
        let output = Command::new(binary())
            .args(["get", "h1"])
            .arg(&source)
            .args([
                "-o",
                &format!("/proc/{}/fd/{}", std::process::id(), stream.as_raw_fd()),
            ])
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        assert_eq!(std::fs::metadata(&target).unwrap().ino(), inode);
        assert_eq!(
            std::fs::read(&target).unwrap(),
            b"\"complete\"\nXXXXXXXXXXXX"
        );
        assert_eq!(stream.stream_position().unwrap(), 7);
    }
}

#[cfg(windows)]
#[test]
fn windows_null_device_is_written_without_creating_a_file() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("page.html");
    std::fs::write(&source, b"<h1>complete</h1>").unwrap();
    let output = Command::new(binary())
        .args(["get", "h1"])
        .arg(&source)
        .args(["-o", "NUL"])
        .current_dir(directory.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert!(output.stdout.is_empty());
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
}

#[cfg(windows)]
#[test]
fn windows_named_pipe_output_is_delivered_and_reader_disconnect_is_silent() {
    use std::io::{BufRead, BufReader, Read};
    for disconnect in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("page.html");
        if disconnect {
            // This exceeds the configured pipe buffer, so closing after one byte
            // interrupts publication rather than merely racing its completion.
            std::fs::write(&source, format!("<h1>{}</h1>", "x".repeat(1024 * 1024))).unwrap();
        } else {
            std::fs::write(&source, b"<h1>pipe</h1>").unwrap();
        }
        let name = format!(
            "htmlcut-{}",
            directory.path().file_name().unwrap().to_string_lossy()
        );
        let script = r#"$ErrorActionPreference='Stop'; $p=[System.IO.Pipes.NamedPipeServerStream]::new($env:HTMLCUT_PIPE_NAME,[System.IO.Pipes.PipeDirection]::In,1,[System.IO.Pipes.PipeTransmissionMode]::Byte,[System.IO.Pipes.PipeOptions]::Asynchronous,4096,4096); try { $connection=$p.WaitForConnectionAsync(); [Console]::Out.WriteLine('READY'); [Console]::Out.Flush(); [void]$connection.GetAwaiter().GetResult(); if ($env:HTMLCUT_PIPE_DISCONNECT -eq '1') { [Console]::Out.Write([char]$p.ReadByte()) } else { $r=[System.IO.StreamReader]::new($p); [Console]::Out.Write($r.ReadToEnd()) }; [Console]::Out.Flush() } finally { $p.Dispose() }"#;
        let mut server = Command::new("powershell.exe")
            .args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                script,
            ])
            .env("HTMLCUT_PIPE_NAME", &name)
            .env(
                "HTMLCUT_PIPE_DISCONNECT",
                if disconnect { "1" } else { "0" },
            )
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut stream = BufReader::new(server.stdout.take().unwrap());
        let (ready_sender, ready_receiver) = std::sync::mpsc::channel();
        let reader = std::thread::spawn(move || -> std::io::Result<Vec<u8>> {
            let mut ready = String::new();
            let handshake = stream.read_line(&mut ready).map(|_| ready);
            let _ = ready_sender.send(handshake);
            let mut delivered = Vec::new();
            stream.read_to_end(&mut delivered)?;
            Ok(delivered)
        });
        let publication = std::panic::catch_unwind(|| {
            let ready = ready_receiver
                .recv_timeout(Duration::from_secs(10))
                .expect("named-pipe server did not announce readiness")
                .unwrap();
            assert_eq!(ready.trim(), "READY");
            let path = format!(r"\\.\pipe\{name}");
            let producer = Command::new(binary())
                .args(["get", "h1"])
                .arg(&source)
                .arg("-o")
                .arg(path)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            let output = bounded_output(producer, Duration::from_secs(10));
            assert!(output.status.success(), "{output:?}");
            assert!(output.stdout.is_empty());
            assert!(output.stderr.is_empty(), "{output:?}");
        });
        if publication.is_err() {
            let _ = server.kill();
        }
        let completion = bounded_output(server, Duration::from_secs(10));
        let delivered = reader.join().unwrap();
        publication.unwrap();
        assert!(completion.status.success(), "{completion:?}");
        let delivered = delivered.unwrap();
        assert_eq!(
            delivered,
            if disconnect {
                &b"\""[..]
            } else {
                &b"\"pipe\"\n"[..]
            }
        );
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
    }
}

#[test]
fn opening_a_regular_file_for_write_does_not_change_it() {
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("probe");
    std::fs::write(&file, b"preserved").unwrap();
    let before = file.metadata().unwrap();
    drop(std::fs::OpenOptions::new().write(true).open(&file).unwrap());
    let after = file.metadata().unwrap();
    assert_eq!(std::fs::read(&file).unwrap(), b"preserved");
    assert_eq!(before.len(), after.len());
    assert_eq!(before.modified().unwrap(), after.modified().unwrap());
    assert_eq!(
        before.permissions().readonly(),
        after.permissions().readonly()
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        assert_eq!(
            (before.mode(), before.ctime(), before.ctime_nsec()),
            (after.mode(), after.ctime(), after.ctime_nsec())
        );
    }
}

#[cfg(windows)]
#[test]
fn windows_dangling_link_resolution_creates_the_target() {
    use std::os::windows::fs::symlink_file;
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("page.html");
    let link = directory.path().join("link");
    let target = directory.path().join("absent.json");
    std::fs::write(&source, b"<h1>complete</h1>").unwrap();
    symlink_file("absent.json", &link)
        .expect("native Windows symlink QA requires symlink creation privileges");
    assert_eq!(std::fs::read_link(&link).unwrap(), Path::new("absent.json"));
    assert_eq!(
        std::fs::canonicalize(&link).unwrap_err().kind(),
        std::io::ErrorKind::NotFound
    );
    let output = Command::new(binary())
        .args(["get", "h1"])
        .arg(&source)
        .arg("-o")
        .arg(&link)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert!(link.symlink_metadata().unwrap().file_type().is_symlink());
    assert_eq!(std::fs::read(&target).unwrap(), b"\"complete\"\n");
}

#[test]
fn documented_unavailable_selectors_refuse_as_selectors() {
    for selector in [
        "p::before",
        "input:checked",
        "input:disabled",
        "a:link",
        "p:lang(en)",
        "p:nth-child(2 of .x)",
        "p:contains(x)",
        "div:has(p:has(a))",
    ] {
        let output = Command::new(binary())
            .args(["get", selector, "missing-input.html"])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2), "{selector}: {output:?}");
        assert_eq!(
            serde_json::from_slice::<Value>(&output.stderr).unwrap()["error"],
            "invalid_selector"
        );
        assert!(output.stdout.is_empty());
    }
}
