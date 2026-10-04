// SPDX-License-Identifier: MPL-2.0
//! Black-box helpers for the binary product; no CLI Rust API.
use std::io::Write;
use std::process::{Command, Output, Stdio};

pub(crate) fn invoke(args: &[&str], input: &[u8]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_htmlcut"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let written = child.stdin.take().unwrap().write_all(input);
    let output = child.wait_with_output().unwrap();
    if let Err(error) = written {
        // Early configuration rejection can close stdin before the parent supplies its payload.
        assert!(
            error.kind() == std::io::ErrorKind::BrokenPipe && !output.status.success(),
            "input handoff failed: {error}; child status: {}",
            output.status
        );
    }
    output
}
