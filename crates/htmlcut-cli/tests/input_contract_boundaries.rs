// SPDX-License-Identifier: MPL-2.0
//! Actual binary input admission and contract-file repair boundaries.
use serde_json::Value;
use std::{
    io::Write,
    process::{Command, Output, Stdio},
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
