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
    child.stdin.take().unwrap().write_all(input).unwrap();
    child.wait_with_output().unwrap()
}
