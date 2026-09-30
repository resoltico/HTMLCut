use crate::app;

#[path = "contract_http.rs"]
mod http;
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
fn minimal_cli_raw_json_error_and_slice_paths() {
    let html = b"<p id='amount'>180</p>";
    let (code, stdout, stderr) =
        invoke(&["htmlcut", "extract", "--stdin", "--css", "#amount"], html);
    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    assert!(stderr.is_empty());
    let result: serde_json::Value = serde_json::from_slice(&stdout).unwrap();
    assert_eq!(result["values"], serde_json::json!(["180"]));
    assert_eq!(stdout.last(), Some(&b'\n'));
    let (code, stdout, stderr) = invoke(
        &["htmlcut", "extract", "--stdin", "--css", "#amount", "--raw"],
        html,
    );
    assert_eq!((code, stdout, stderr), (0, b"180".to_vec(), Vec::new()));
    let (code, stdout, stderr) = invoke(&["htmlcut", "extract", "--stdin", "--css", "aside"], html);
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
    assert_eq!(
        (code, stdout, stderr),
        (0, "\r\n✓\r\n".as_bytes().to_vec(), Vec::new())
    );
}

#[path = "contract_options.rs"]
mod options;
