// SPDX-License-Identifier: MPL-2.0
use crate::app;

#[path = "contract_io.rs"]
mod io;

fn invoke(args: &[&str], input: &[u8]) -> (i32, Vec<u8>, Vec<u8>) {
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = app::run(
        args.iter().copied(),
        &mut std::io::Cursor::new(input),
        &mut stdout,
        &mut stderr,
    );
    (code, stdout, stderr)
}

#[test]
fn minimal_cli_json_raw_errors_and_retired_slice_flags() {
    let html = b"<p id='amount'>180</p>";
    let (code, stdout, stderr) = invoke(
        &["htmlcut", "extract", "--stdin", "--select", "#amount"],
        html,
    );
    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    let result: serde_json::Value = serde_json::from_slice(&stdout).unwrap();
    assert_eq!(result, serde_json::json!(["180"]));
    assert_eq!(stdout.last(), Some(&b'\n'));
    let (code, stdout, stderr) = invoke(
        &[
            "htmlcut", "extract", "--stdin", "--select", "#amount", "--raw",
        ],
        html,
    );
    assert_eq!((code, stdout, stderr), (0, b"180".to_vec(), Vec::new()));
    let (code, stdout, stderr) = invoke(
        &["htmlcut", "extract", "--stdin", "--select", "aside"],
        html,
    );
    assert_eq!(code, 3);
    assert!(stdout.is_empty());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&stderr).unwrap()["code"],
        "no_match"
    );
    let (code, stdout, stderr) = invoke(
        &[
            "htmlcut", "extract", "--stdin", "--start", "START", "--end", "END", "--raw",
        ],
        "éSTART\r\n✓\r\nEND".as_bytes(),
    );
    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&stderr).unwrap()["code"],
        "invalid_options"
    );
}

#[path = "contract_options.rs"]
mod options;
