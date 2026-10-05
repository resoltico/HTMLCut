// SPDX-License-Identifier: MPL-2.0
use super::*;

#[test]
fn bounded_diagnostics_and_metadata_do_not_publish_prefixes() {
    let mut stderr = Vec::new();
    let mut error = options("bounded diagnostic");
    error.message = "synthetic private diagnostic".repeat(500);
    assert_eq!(report(error, &mut stderr), 5);
    assert!(stderr.is_empty());
    let metadata = crate::operation_metadata::describe(None).unwrap();
    let complete = crate::publication::json(&metadata, 16 * 1024).unwrap();
    for maximum in [0, complete.len() - 1, complete.len(), complete.len() + 1] {
        let mut stdout = Vec::new();
        let result = emit_json(&metadata, maximum, &mut stdout);
        if maximum < complete.len() {
            assert_eq!(
                result.unwrap_err().code,
                htmlcut_core::ErrorCode::ResourceLimit
            );
            assert!(stdout.is_empty());
        } else {
            result.unwrap();
            assert_eq!(stdout, complete);
        }
    }
}

#[cfg(unix)]
#[test]
fn resolved_replay_path_conversion_rejects_invalid_utf8() {
    use std::os::unix::ffi::OsStringExt;
    let path = std::path::PathBuf::from(std::ffi::OsString::from_vec(vec![0xff]));
    assert_eq!(
        saved_path_utf8(&path).unwrap_err().code,
        htmlcut_core::ErrorCode::InvalidOptions
    );
    let root = htmlcut_tempdir::tempdir().unwrap();
    let source = root.path().join("source.html");
    std::fs::write(&source, "<p>value</p>").unwrap();
    let saved = root.path().join("run.json");
    CANONICAL_PATH_RESPONSE.with_borrow_mut(|slot| *slot = Some(path));
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = run(
        [
            "htmlcut",
            "extract",
            "--file",
            source.to_str().unwrap(),
            "--select",
            "p",
            "--save-run",
            saved.to_str().unwrap(),
        ],
        &mut std::io::empty(),
        &mut out,
        &mut err,
    );
    assert_eq!(code, 2);
    assert!(out.is_empty());
    assert!(!saved.exists());
}
