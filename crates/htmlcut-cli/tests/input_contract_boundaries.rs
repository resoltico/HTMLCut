// SPDX-License-Identifier: MPL-2.0
//! Actual binary input admission and contract-file repair boundaries.
use serde_json::Value;
#[path = "support/capture.rs"]
mod capture;
use std::{
    io::Write,
    process::{Command, Output, Stdio},
    time::{Duration, Instant},
};

fn command() -> Command {
    Command::new(
        std::env::var_os("HTMLCUT_TEST_BINARY")
            .unwrap_or_else(|| env!("CARGO_BIN_EXE_htmlcut").into()),
    )
}
fn piped(args: &[&str], bytes: &[u8]) -> Output {
    let mut child = command()
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(bytes).unwrap();
    child.wait_with_output().unwrap()
}
fn held_stdin(mut invocation: Command) -> Output {
    let mut child = invocation
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let producer = child.stdin.take().unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        if child.try_wait().unwrap().is_some() {
            // Do not send EOF until the command has already refused the conflict.
            drop(producer);
            return child.wait_with_output().unwrap();
        }
        if Instant::now() >= deadline {
            child.kill().unwrap();
            drop(producer);
            let output = child.wait_with_output().unwrap();
            panic!("input admission waited for an open producer: {output:?}");
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn contextual_help_distinguishes_schema_output_from_snapshot_input() {
    for name in ["schema", "get", "outline", "run"] {
        let output = command().args([name, "--help"]).output().unwrap();
        assert!(output.status.success(), "{output:?}");
        assert!(output.stderr.is_empty());
        let text = String::from_utf8(output.stdout).unwrap();
        assert!(text.contains("Usage:"));
        if name == "schema" {
            assert!(text.contains("--output"));
            assert!(text.contains("--timeout"));
            assert!(text.contains("-o - selects stdout"));
            assert!(!text.contains("FILE is a local snapshot"));
            for unsupported in ["--raw", "--max-nodes", "--encoding", "--max-input-bytes"] {
                assert!(!text.contains(unsupported), "{text}");
            }
        } else {
            assert!(text.contains("FILE is a local snapshot; '-' is standard input."));
            assert!(!text.contains("Schema is written to stdout by default"));
        }
    }
}

#[test]
fn schema_routes_complete_output_without_accepting_snapshot_options() {
    let directory = tempfile::tempdir().unwrap();
    let destination = directory.path().join("schema.json");
    let output = command()
        .args(["schema", "-o"])
        .arg(&destination)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
    let schema: Value = serde_json::from_slice(&std::fs::read(&destination).unwrap()).unwrap();
    assert!(schema["$defs"].is_object());
    let output = command().args(["schema", "-o", "-"]).output().unwrap();
    assert!(output.status.success(), "{output:?}");
    assert!(output.stderr.is_empty());
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        schema
    );
    for options in [
        vec!["page.html"],
        vec!["--raw"],
        vec!["--max-nodes", "1"],
        vec!["--encoding", "utf-8"],
    ] {
        let output = command().arg("schema").args(options).output().unwrap();
        assert_eq!(output.status.code(), Some(2), "{output:?}");
        assert!(output.stdout.is_empty());
        assert_eq!(
            serde_json::from_slice::<Value>(&output.stderr).unwrap()["error"],
            "invalid_request"
        );
    }
}

#[test]
fn contract_and_html_stdin_are_valid_when_the_other_source_is_a_file() {
    let directory = tempfile::tempdir().unwrap();
    let page = directory.path().join("page.html");
    let contract = directory.path().join("contract.json");
    std::fs::write(&page, b"<p>complete</p>").unwrap();
    std::fs::write(&contract, b"\"p\"").unwrap();
    for (args, bytes) in [
        (
            vec!["run", "--from-file", "-", page.to_str().unwrap()],
            b"\"p\"".as_slice(),
        ),
        (
            vec!["run", "--from-file", contract.to_str().unwrap(), "-"],
            b"<p>complete</p>".as_slice(),
        ),
    ] {
        let output = piped(&args, bytes);
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, b"\"complete\"\n");
        assert!(output.stderr.is_empty());
    }
}

#[test]
fn competing_stdin_claims_refuse_before_eof_or_invocation_timeout() {
    #[cfg(unix)]
    let directory = tempfile::tempdir().unwrap();
    let sources = vec![None, Some(std::path::PathBuf::from("-"))];
    #[cfg(unix)]
    let sources = {
        let mut sources = sources;
        let alias = directory.path().join("contract-and-html-stdin");
        std::os::unix::fs::symlink("/dev/stdin", &alias).unwrap();
        sources.extend([
            Some("/dev/stdin".into()),
            Some("/dev/fd/0".into()),
            Some(alias),
        ]);
        sources
    };
    for source in sources {
        let mut invocation = command();
        invocation.args(["run", "--from-file", "-", "--timeout", "1"]);
        if let Some(source) = source {
            invocation.arg(source);
        }
        let output = held_stdin(invocation);
        assert_eq!(output.status.code(), Some(2), "{output:?}");
        assert!(output.stdout.is_empty());
        let error: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(error["error"], "invalid_request");
        assert!(error["message"].as_str().unwrap().contains("distinct"));
    }
}

#[test]
fn contract_file_and_stdin_enforce_the_complete_262144_byte_grammar_bound() {
    let directory = tempfile::tempdir().unwrap();
    let page = directory.path().join("page.html");
    let contract = directory.path().join("contract.json");
    std::fs::write(&page, b"<p>complete</p>").unwrap();
    for size in [262_144, 262_145] {
        let mut bytes = b"\"p\"".to_vec();
        bytes.resize(size, b' ');
        std::fs::write(&contract, &bytes).unwrap();
        for stdin in [false, true] {
            let output = if stdin {
                piped(&["run", "--from-file", "-", page.to_str().unwrap()], &bytes)
            } else {
                command()
                    .args(["run", "--from-file"])
                    .arg(&contract)
                    .arg(&page)
                    .output()
                    .unwrap()
            };
            if size == 262_144 {
                assert!(output.status.success(), "{output:?}");
                assert_eq!(output.stdout, b"\"complete\"\n");
                assert!(output.stderr.is_empty());
            } else {
                assert_eq!(output.status.code(), Some(2), "{output:?}");
                assert!(output.stdout.is_empty());
                let error: Value = serde_json::from_slice(&output.stderr).unwrap();
                assert_eq!(error["error"], "invalid_contract");
                assert!(error["message"].as_str().unwrap().contains("262144"));
            }
        }
    }
}

#[test]
fn url_shaped_filenames_refuse_locally_with_the_acquisition_repair_hint() {
    let directory = tempfile::tempdir().unwrap();
    for source in [
        "http://snapshot.local.invalid/page",
        "https://snapshot.local.invalid/page",
    ] {
        let output = command()
            .current_dir(directory.path())
            .args(["get", "p", source])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(5), "{output:?}");
        assert!(output.stdout.is_empty());
        let error: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(error["error"], "input_unreadable");
        let message = error["message"].as_str().unwrap();
        assert!(message.contains("FILE is a local snapshot path"));
        assert!(message.contains("Fetch successfully into owned storage first"));
        assert!(message.contains("--base-url") && message.contains("--encoding"));
    }
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
}
#[test]
fn explicit_input_dash_and_byte_limits_preserve_complete_results() {
    let bytes = b"<p>complete</p>";
    for excess in [0, 1, 2] {
        let maximum = (bytes.len() - excess).to_string();
        let output = piped(&["get", "p", "-", "--max-input-bytes", &maximum], bytes);
        if excess == 0 {
            assert!(output.status.success(), "{output:?}");
            assert_eq!(output.stdout, b"\"complete\"\n");
            assert!(output.stderr.is_empty());
        } else {
            assert_eq!(output.status.code(), Some(4), "{output:?}");
            assert!(output.stdout.is_empty());
            let error: Value = serde_json::from_slice(&output.stderr).unwrap();
            assert_eq!(error["limit"], "input_bytes");
            assert_eq!(error["bound"], bytes.len() - excess);
        }
    }
}
#[test]
fn contract_directory_spellings_are_not_stdin_markers() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::create_dir(directory.path().join("-")).unwrap();
    std::fs::write(directory.path().join("page.html"), b"<p>complete</p>").unwrap();
    for spelling in ["-/", "-/.", "./-"] {
        for explicit_source in [false, true] {
            let mut invocation = command();
            invocation
                .current_dir(directory.path())
                .arg("run")
                .arg(format!("--from-file={spelling}"))
                .stdin(Stdio::null());
            if explicit_source {
                invocation.arg("page.html");
            }
            let output = invocation.output().unwrap();
            assert_eq!(output.status.code(), Some(5), "{spelling}: {output:?}");
            assert!(output.stdout.is_empty());
            let error: Value = serde_json::from_slice(&output.stderr).unwrap();
            assert_eq!(error["error"], "input_unreadable");
        }
    }
}
#[test]
fn contract_file_accepts_one_input_operand_and_refuses_more() {
    let directory = tempfile::tempdir().unwrap();
    let contract = directory.path().join("contract.json");
    let input = directory.path().join("page.html");
    std::fs::write(&contract, b"\"p\"").unwrap();
    std::fs::write(&input, b"<p>complete</p>").unwrap();
    let output = command()
        .args(["run", "--from-file"])
        .arg(&contract)
        .arg(&input)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(output.stdout, b"\"complete\"\n");
    let output = command()
        .args(["run", "--from-file"])
        .arg(&contract)
        .arg(&input)
        .arg(&input)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(output.stdout.is_empty());
    let error: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["error"], "invalid_request");
    assert!(
        error["message"]
            .as_str()
            .unwrap()
            .contains("at most one FILE")
    );
}

#[test]
fn relative_output_name_uses_the_current_directory_atomically() {
    let directory = tempfile::tempdir().unwrap();
    let input = directory.path().join("page.html");
    std::fs::write(&input, b"<p>complete</p>").unwrap();
    for existing in [false, true] {
        if existing {
            std::fs::write(
                directory.path().join("result.json"),
                b"a longer old result stays intact until replacement",
            )
            .unwrap();
        }
        let output = command()
            .current_dir(directory.path())
            .args(["get", "p"])
            .arg(&input)
            .args(["-o", "result.json"])
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        assert_eq!(
            std::fs::read(directory.path().join("result.json")).unwrap(),
            b"\"complete\"\n"
        );
        assert!(output.stdout.is_empty());
        assert!(output.stderr.is_empty());
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 2);
    }
}
#[test]
fn contract_file_hint_is_specific_to_a_mistaken_positional_file() {
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("invalid.json");
    std::fs::write(&file, b"{").unwrap();
    std::fs::write(directory.path().join("{"), b"unrelated file").unwrap();
    std::fs::write(directory.path().join("true"), b"unrelated file").unwrap();
    for from_file in [false, true] {
        let mut command = command();
        command.current_dir(directory.path());
        command.arg("run");
        if from_file {
            command.arg("--from-file");
        }
        let output = command.arg(&file).output().unwrap();
        let error: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(error["error"], "invalid_contract");
        assert_eq!(
            error["message"]
                .as_str()
                .unwrap()
                .contains("Read a contract file with --from-file."),
            !from_file
        );
    }
    let output = command()
        .args(["run", "{\"bogus\":true}"])
        .output()
        .unwrap();
    let error: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["error"], "invalid_contract");
    assert!(
        !error["message"]
            .as_str()
            .unwrap()
            .contains("Read a contract file with --from-file.")
    );
    let output = command()
        .current_dir(directory.path())
        .args(["run", "true"])
        .output()
        .unwrap();
    let error: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["error"], "invalid_contract");
    assert!(
        !error["message"]
            .as_str()
            .unwrap()
            .contains("Read a contract file with --from-file.")
    );
}

#[cfg(unix)]
#[test]
fn contract_descriptor_aliases_conflict_with_html_stdin_before_consumption() {
    let root = tempfile::tempdir().unwrap();
    let alias = root.path().join("contract-stdin");
    std::os::unix::fs::symlink("/dev/stdin", &alias).unwrap();
    let mut aliases = vec![
        std::path::PathBuf::from("/dev/stdin"),
        "/dev/fd/0".into(),
        alias,
    ];
    if cfg!(target_os = "linux") {
        aliases.extend(["/proc/self/fd/0".into(), "/proc/thread-self/fd/0".into()]);
    }
    for source in aliases {
        let mut invocation = command();
        invocation
            .args(["run", "--from-file"])
            .arg(source)
            .args(["-", "--timeout", "1"]);
        let output = held_stdin(invocation);
        assert_eq!(output.status.code(), Some(2), "{output:?}");
        assert!(output.stdout.is_empty());
        assert_eq!(
            serde_json::from_slice::<Value>(&output.stderr).unwrap()["error"],
            "invalid_request"
        );
    }
}

#[test]
fn declared_command_and_input_carriers_have_independent_unicode_oracles() {
    let directory = tempfile::tempdir().unwrap();
    let page = directory.path().join("page.html");
    let contract = directory.path().join("contract.json");
    std::fs::write(&page, "<p>Rīga 雪</p>").unwrap();
    std::fs::write(&contract, b"\"p\"").unwrap();
    let mut checked = 0;
    for operation in 0..5 {
        for input in 0..3 {
            let mut invocation = command();
            match operation {
                0 => {
                    invocation.args(["get", "p"]);
                }
                1 => {
                    invocation.args(["run", "\"p\""]);
                }
                2 => {
                    invocation.args(["run", "--from-file"]).arg(&contract);
                }
                3 => {
                    invocation.args(["run", "--from-file", "-"]);
                }
                4 => {
                    invocation.args(["outline", "--select", "p"]);
                }
                _ => unreachable!(),
            }
            if input == 0 {
                invocation.arg(&page);
            }
            if input == 2 {
                invocation.arg("-");
            }
            if operation == 3 && input != 0 {
                let output = held_stdin(invocation);
                assert_eq!(output.status.code(), Some(2));
                assert!(output.stdout.is_empty());
                let error: Value = serde_json::from_slice(&output.stderr).unwrap();
                assert_eq!(error["error"], "invalid_request");
            } else {
                let bytes = if operation == 3 {
                    b"\"p\"".as_slice()
                } else if input != 0 {
                    "<p>Rīga 雪</p>".as_bytes()
                } else {
                    b""
                };
                let output = capture::finite_capture(
                    &mut invocation,
                    Duration::from_secs(3),
                    1024 * 1024,
                    capture::Stdin::Bytes(bytes),
                )
                .unwrap();
                assert!(output.status.success(), "{operation}/{input}: {output:?}");
                assert!(output.stderr.is_empty());
                let expected = if operation == 4 {
                    "1 match for p\np \"Rīga 雪\"\n"
                } else {
                    "\"Rīga 雪\"\n"
                };
                assert_eq!(output.stdout, expected.as_bytes(), "{operation}/{input}");
            }
            checked += 1;
        }
    }
    assert_eq!(checked, 15);
}

#[test]
fn limit_forwarding_has_exact_and_below_controls_without_publication() {
    let directory = tempfile::tempdir().unwrap();
    let page = directory.path().join("page.html");
    let destination = directory.path().join("result");
    // Document + html/head/body/p + text = 6 nodes; input and readings count UTF-8 bytes.
    std::fs::write(&page, "<p>雪</p>").unwrap();
    for (flag, exact, limit) in [
        ("--max-input-bytes", 10, "input_bytes"),
        ("--max-nodes", 6, "nodes"),
        ("--max-reading-bytes", 3, "reading_bytes"),
        ("--max-output-bytes", 6, "output_bytes"),
    ] {
        let mut invocation = command();
        invocation
            .args(["get", "p"])
            .arg(&page)
            .args([flag, &exact.to_string()]);
        let output = capture::finite_capture(
            &mut invocation,
            Duration::from_secs(3),
            1024 * 1024,
            capture::Stdin::Closed,
        )
        .unwrap();
        assert!(output.status.success(), "{flag}: {output:?}");
        assert_eq!(output.stdout, "\"雪\"\n".as_bytes());
        assert!(output.stderr.is_empty());
        std::fs::write(&destination, b"old").unwrap();
        let mut invocation = command();
        invocation
            .args(["get", "p"])
            .arg(&page)
            .args([flag, &(exact - 1).to_string(), "-o"])
            .arg(&destination);
        let output = capture::finite_capture(
            &mut invocation,
            Duration::from_secs(3),
            1024 * 1024,
            capture::Stdin::Closed,
        )
        .unwrap();
        assert_eq!(output.status.code(), Some(4), "{flag}: {output:?}");
        assert!(output.stdout.is_empty());
        assert_eq!(output.stderr.iter().filter(|b| **b == b'\n').count(), 1);
        let error: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(error["error"], "limit_exceeded");
        assert_eq!(error["limit"], limit);
        assert_eq!(error["bound"], exact - 1);
        assert_eq!(std::fs::read(&destination).unwrap(), b"old");
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 2);
    }
}

#[test]
fn invalid_preflight_and_numeric_syntax_refuse_before_held_html() {
    let directory = tempfile::tempdir().unwrap();
    let destination = directory.path().join("result");
    for (args, code) in [
        (
            vec!["run", "--from-file", "absent-contract.json"],
            "input_unreadable",
        ),
        (vec!["run", "{\"unknown\":1}"], "invalid_contract"),
        (vec!["run", "{\"select\":\"p:hover\"}"], "invalid_selector"),
        (
            vec!["run", "{\"fields\":{\"x\":\"p\"}}", "--raw"],
            "unrepresentable_output",
        ),
        (vec!["get", "p", "--timeout", "NaN"], "invalid_request"),
        (vec!["get", "p", "--max-nodes", "1.0"], "invalid_request"),
    ] {
        std::fs::write(&destination, b"old").unwrap();
        let mut invocation = command();
        invocation.args(args).arg("-o").arg(&destination);
        let output = capture::finite_capture(
            &mut invocation,
            Duration::from_secs(3),
            1024 * 1024,
            capture::Stdin::Held,
        )
        .unwrap();
        assert_eq!(
            output.status.code(),
            Some(if code == "input_unreadable" {
                5
            } else if code == "unrepresentable_output" {
                3
            } else {
                2
            }),
            "{output:?}"
        );
        assert!(output.stdout.is_empty());
        let error: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(error["error"], code);
        assert_eq!(std::fs::read(&destination).unwrap(), b"old");
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
    }
}

#[test]
fn carrier_child() {
    if std::env::var_os("HTMLCUT_CARRIER_CHILD").is_some() {
        std::thread::sleep(Duration::from_secs(5));
    }
}
#[test]
fn carrier_owner_reaps_a_child_that_never_reads_or_finishes() {
    let mut child = Command::new(std::env::current_exe().unwrap());
    child
        .args(["--exact", "carrier_child", "--nocapture"])
        .env("HTMLCUT_CARRIER_CHILD", "hang");
    let started = Instant::now();
    let error = capture::finite_capture(
        &mut child,
        Duration::from_millis(100),
        1024,
        capture::Stdin::Bytes(b"held input"),
    )
    .unwrap_err();
    assert_eq!(
        error, "capture deadline exceeded",
        "cleanup must be confirmed before assertions"
    );
    assert!(started.elapsed() < Duration::from_secs(3));
}
#[test]
fn decoding_and_semantic_refusals_precede_an_unpublishable_destination() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/contract");
    for (case, expected, exit) in [
        ("authored-offers-bad-decoding", "undecodable_input", 5),
        ("authored-offers-wrong-page", "expectation_failed", 3),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let destination = directory.path().join("destination");
        std::fs::create_dir(&destination).unwrap();
        std::fs::write(destination.join("marker"), b"old marker").unwrap();
        let case = root.join(case);
        let mut invocation = command();
        invocation
            .args(["run", "--from-file"])
            .arg(case.join("contract.json"))
            .arg(case.join("page.html"));
        invocation
            .args([
                "--encoding",
                if expected == "undecodable_input" {
                    "utf-8"
                } else {
                    "windows-1252"
                },
                "--base-url",
                "https://origin.test/shop/page",
                "-o",
            ])
            .arg(&destination);
        let output = capture::finite_capture(
            &mut invocation,
            Duration::from_secs(3),
            1024,
            capture::Stdin::Closed,
        )
        .unwrap();
        assert_eq!(output.status.code(), Some(exit), "{output:?}");
        assert!(output.stdout.is_empty());
        assert_eq!(output.stderr.iter().filter(|b| **b == b'\n').count(), 1);
        let facts: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(facts["error"], expected);
        assert_eq!(
            std::fs::read(destination.join("marker")).unwrap(),
            b"old marker"
        );
        assert_eq!(std::fs::read_dir(&destination).unwrap().count(), 1);
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
        // This exact destination independently refuses a valid operation.
        let mut publish = command();
        publish
            .args(["get", "h1"])
            .arg(root.join("authored-offers-bom/page.html"))
            .arg("-o")
            .arg(&destination);
        let output = capture::finite_capture(
            &mut publish,
            Duration::from_secs(3),
            1024,
            capture::Stdin::Closed,
        )
        .unwrap();
        assert_eq!(output.status.code(), Some(5), "{output:?}");
        assert!(output.stdout.is_empty());
        assert_eq!(output.stderr.iter().filter(|b| **b == b'\n').count(), 1);
        assert_eq!(
            serde_json::from_slice::<Value>(&output.stderr).unwrap()["error"],
            "output_unwritable"
        );
        assert_eq!(
            std::fs::read(destination.join("marker")).unwrap(),
            b"old marker"
        );
        assert_eq!(std::fs::read_dir(&destination).unwrap().count(), 1);
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
    }
}
