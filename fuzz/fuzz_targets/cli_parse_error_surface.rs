// SPDX-License-Identifier: MPL-2.0
#![cfg_attr(not(test), cfg_attr(feature = "fuzzing", no_main))]
#[cfg(all(feature = "fuzzing", not(test)))]
use libfuzzer_sys::fuzz_target;
#[cfg(all(feature = "fuzzing", not(test)))]
#[path = "support/cli.rs"]
#[allow(dead_code)]
mod cli;
#[cfg(all(feature = "fuzzing", not(test)))]
use cli::{command, command_diagnostics, input};
#[cfg(all(feature = "fuzzing", not(test)))]
fuzz_target!(|data: &[u8]| {
    let suffix = String::from_utf8_lossy(&data[..data.len().min(4096)]);
    let unknown = format!("--unrecognized-{suffix}");
    use clap::Parser;
    let parse_error =
        command::Cli::try_parse_from(["htmlcut", "extract", "--stdin", "--select", "p", &unknown])
            .err()
            .unwrap();
    let failure = command_diagnostics::failure(&parse_error);
    let error = serde_json::to_value(&failure).unwrap();
    assert_eq!(error["schema"], "htmlcut.extraction.error");
});
#[cfg(any(test, not(feature = "fuzzing")))]
fn main() {}
