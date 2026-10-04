// SPDX-License-Identifier: MPL-2.0
#![cfg_attr(not(test), cfg_attr(feature = "fuzzing", no_main))]
#[cfg(all(feature = "fuzzing", not(test)))]
use libfuzzer_sys::fuzz_target;
#[cfg(all(feature = "fuzzing", not(test)))]
#[path = "support/cli.rs"]
#[allow(dead_code)]
mod cli;
#[cfg(all(feature = "fuzzing", not(test)))]
use cli::{
    app, bundle, bundle_io, command, command_diagnostics, input, operation_metadata, publication,
};
#[cfg(all(feature = "fuzzing", not(test)))]
fuzz_target!(|data: &[u8]| {
    let suffix = String::from_utf8_lossy(&data[..data.len().min(4096)]);
    let unknown = format!("--unrecognized-{suffix}");
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = app::run(
        ["htmlcut", "extract", "--stdin", "--css", "p", &unknown],
        &mut std::io::Cursor::new(b"<p>value</p>"),
        &mut stdout,
        &mut stderr,
    );
    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    let error: serde_json::Value = serde_json::from_slice(&stderr).unwrap();
    assert_eq!(error["schema"], "htmlcut.extraction.error");
});
#[cfg(any(test, not(feature = "fuzzing")))]
fn main() {}
