// SPDX-License-Identifier: MPL-2.0
//! Actual binary input admission and contract-file repair boundaries.
use serde_json::Value;
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
            "invalid_arguments"
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
        assert_eq!(error["error"], "invalid_arguments");
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
    assert_eq!(error["error"], "invalid_arguments");
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
