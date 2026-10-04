// SPDX-License-Identifier: MPL-2.0
//! Acquisition and charset handling belong to callers; retired inputs cannot be used.
use std::process::{Command, Stdio};
#[test]
fn environment_source_and_charset_flags_are_rejected_without_leaking_values() {
    for args in [
        vec!["extract", "--url-env", "SYNTHETIC_URL", "--css", "p"],
        vec![
            "extract",
            "--url",
            "https://invalid.test/?secret=SYNTHETIC_SECRET",
            "--css",
            "p",
        ],
        vec!["extract", "--stdin", "--css", "p", "--encoding", "utf-16le"],
    ] {
        let result = Command::new(env!("CARGO_BIN_EXE_htmlcut"))
            .args(args)
            .env(
                "SYNTHETIC_URL",
                "https://invalid.test/?secret=SYNTHETIC_SECRET",
            )
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert_eq!(result.status.code(), Some(2));
        assert!(result.stdout.is_empty());
        assert!(!String::from_utf8_lossy(&result.stderr).contains("SYNTHETIC_SECRET"));
    }
}
