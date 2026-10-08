// SPDX-License-Identifier: MPL-2.0
use super::*;
use htmlcut_core::{ErrorCode, ExtractionError};
use std::io::{self, Read, Write};

#[cfg(unix)]
#[test]
fn native_path_errors_are_typed_without_persisting_path_text() {
    use std::os::unix::ffi::OsStringExt;
    let mut args: Vec<std::ffi::OsString> = ["htmlcut", "extract", "--file"]
        .into_iter()
        .map(Into::into)
        .collect();
    args.push(std::ffi::OsString::from_vec(vec![0xff]));
    args.extend(["--select", "p"].into_iter().map(Into::into));
    let mut output = Vec::new();
    let mut errors = Vec::new();
    assert_eq!(
        app::run(args, &mut io::empty(), &mut output, &mut errors),
        5
    );
    assert!(output.is_empty());
    assert_eq!(
        invoke(
            &[
                "htmlcut",
                "extract",
                "--file",
                "/htmlcut-missing-source",
                "--select",
                "p"
            ],
            b""
        )
        .0,
        5
    );
}

#[test]
fn destination_changed_during_acquisition_never_publishes_a_result() {
    struct ChangeDestination {
        path: std::path::PathBuf,
        once: bool,
    }
    impl Read for ChangeDestination {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            if self.once {
                return Ok(0);
            }
            self.once = true;
            std::fs::create_dir(&self.path)?;
            let bytes = b"<p>value</p>";
            buffer[..bytes.len()].copy_from_slice(bytes);
            Ok(bytes.len())
        }
    }
    let root = htmlcut_tempdir::tempdir().unwrap();
    let target = root.path().join("output.json");
    let mut source = ChangeDestination {
        path: target.clone(),
        once: false,
    };
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = app::run(
        [
            "htmlcut",
            "extract",
            "--stdin",
            "--select",
            "p",
            "--output",
            target.to_str().unwrap(),
        ],
        &mut source,
        &mut out,
        &mut err,
    );
    assert_eq!(code, 5);
    assert!(out.is_empty());
}

struct ReadFailure;
impl Read for ReadFailure {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::other("secret transport detail"))
    }
}

#[derive(Default)]
struct WriteFailure {
    bytes: Vec<u8>,
    flush_only: bool,
}
impl Write for WriteFailure {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        if self.flush_only {
            self.bytes.extend_from_slice(data);
            Ok(data.len())
        } else {
            Err(io::Error::other("secret publication detail"))
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::other("secret flush detail"))
    }
}

#[test]
fn utf8_snapshots_preserve_bom_crlf_nul_and_reject_other_encodings() {
    for bytes in [b"a\r\nb".as_slice(), b"\xef\xbb\xbfa\0".as_slice()] {
        let snapshot = crate::input::snapshot(None, &mut io::Cursor::new(bytes), None).unwrap();
        assert_eq!(snapshot.html().as_bytes(), bytes);
    }
    for bytes in [b"\xff".as_slice(), b"\xff\xfeA\0".as_slice()] {
        assert_eq!(
            crate::input::snapshot(None, &mut io::Cursor::new(bytes), None)
                .unwrap_err()
                .code,
            ErrorCode::Decoding
        );
    }
    for (maximum, valid) in [(2, false), (3, true), (4, true)] {
        assert_eq!(
            crate::input::read_bounded(&mut io::Cursor::new(b"abc"), maximum).is_ok(),
            valid
        );
    }
}

#[test]
fn t31_read_write_flush_and_diagnostic_failures_are_truthful_and_redacted() {
    let mut output = Vec::new();
    let mut error = Vec::new();
    let code = app::run(
        ["htmlcut", "extract", "--stdin", "--select", "p"],
        &mut ReadFailure,
        &mut output,
        &mut error,
    );
    assert_eq!(code, 5);
    assert!(output.is_empty());
    assert!(
        !String::from_utf8(error)
            .unwrap()
            .contains("secret transport")
    );
    for flush_only in [false, true] {
        let mut writer = WriteFailure {
            flush_only,
            ..Default::default()
        };
        let mut error = Vec::new();
        let code = app::run(
            ["htmlcut", "extract", "--stdin", "--select", "p"],
            &mut io::Cursor::new(b"<p>180</p>"),
            &mut writer,
            &mut error,
        );
        assert_eq!(code, 5);
        let error: ExtractionError = serde_json::from_slice(&error).unwrap();
        assert_eq!(error.code, ErrorCode::Publication);
        assert_eq!(error.selected_count, None);
        assert!(!error.message.contains("secret"));
        assert_eq!(writer.bytes.is_empty(), !flush_only);
    }
    let code = app::run(
        ["htmlcut", "extract", "--stdin", "--select", "missing"],
        &mut io::Cursor::new(b"<p>180</p>"),
        &mut Vec::new(),
        &mut WriteFailure::default(),
    );
    assert_eq!(code, 5); // reporting a semantic error itself failed
}

#[test]
fn t20_t22_raw_cardinality_and_json_escaping_are_staged() {
    let (code, output, error) = invoke(
        &[
            "htmlcut", "extract", "--stdin", "--select", "p", "--all", "--raw",
        ],
        b"<p>A</p><p>B</p>",
    );
    assert_eq!(code, 2);
    assert!(output.is_empty());
    assert!(!error.is_empty());
    let (code, output, error) = invoke(
        &["htmlcut", "extract", "--stdin", "--select", "p", "--raw"],
        b"<p></p>",
    );
    assert_eq!(code, 0);
    assert!(output.is_empty());
    assert!(error.is_empty());
    let text = "\"\n\\";
    let mut outputs = Vec::new();
    for maximum in [5, 6, 7] {
        outputs.push(crate::publication::json(&text, maximum).is_ok());
    }
    assert_eq!(outputs, [false, false, false]); // escaped JSON plus framing costs nine bytes
    assert_eq!(crate::publication::json(&text, 9).unwrap().len(), 9);
    assert!(crate::publication::json(&text, 8).is_err());
    assert!(crate::publication::json(&text, 10).is_ok());
}

#[test]
fn t31_atomic_create_race_overwrite_collision_and_cleanup() {
    let root = htmlcut_tempdir::tempdir().unwrap();
    let target = root.path().join("result.json");
    let staged = crate::publication::Staged::prepare(&target, b"replacement", false).unwrap();
    std::fs::write(&target, b"winner").unwrap();
    assert!(staged.commit().is_err());
    assert_eq!(std::fs::read(&target).unwrap(), b"winner");
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
    crate::publication::Staged::prepare(&target, b"replacement", true)
        .unwrap()
        .commit()
        .unwrap();
    assert_eq!(std::fs::read(&target).unwrap(), b"replacement");
    assert!(
        crate::publication::validate_destinations(
            std::slice::from_ref(&target),
            std::slice::from_ref(&target),
            true
        )
        .is_err()
    );
    assert!(
        crate::publication::validate_destinations(&[], &[target.clone(), target.clone()], true)
            .is_err()
    );
    assert!(
        crate::publication::validate_destinations(&[], std::slice::from_ref(&target), false)
            .is_err()
    );
    let blocked = root.path().join("directory");
    std::fs::create_dir(&blocked).unwrap();
    assert!(crate::publication::Staged::prepare(&blocked, b"x", true).is_err());
    assert!(
        crate::publication::Staged::prepare(&root.path().join("missing/target"), b"x", false)
            .is_err()
    );
}

#[cfg(unix)]
#[test]
fn t31_symlink_destinations_cannot_follow_or_clobber_the_source() {
    let root = htmlcut_tempdir::tempdir().unwrap();
    let source = root.path().join("source.html");
    let alias = root.path().join("alias");
    std::fs::write(&source, "<p>180</p>").unwrap();
    std::os::unix::fs::symlink(&source, &alias).unwrap();
    assert!(crate::publication::Staged::prepare(&alias, b"changed", true).is_err());
    assert_eq!(std::fs::read_to_string(&source).unwrap(), "<p>180</p>");
}

#[test]
fn t31_help_and_version_write_or_flush_failures_do_not_report_success() {
    for option in ["--help", "--version"] {
        for flush_only in [false, true] {
            let mut writer = WriteFailure {
                flush_only,
                ..Default::default()
            };
            let mut error = Vec::new();
            let code = app::run(
                ["htmlcut", option],
                &mut io::Cursor::new(b""),
                &mut writer,
                &mut error,
            );
            assert_eq!(code, 5);
            assert_eq!(
                serde_json::from_slice::<serde_json::Value>(&error).unwrap()["code"],
                "publication"
            );
            assert!(!String::from_utf8_lossy(&error).contains("secret"));
        }
    }
}

#[test]
fn t20_t29_default_json_accepts_a_complete_large_value_with_escaping() {
    let value = "\"".repeat(1024 * 1024);
    let html = format!("<p>{value}</p>");
    let (code, stdout, stderr) = invoke(
        &["htmlcut", "extract", "--stdin", "--select", "p"],
        html.as_bytes(),
    );
    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    let result: serde_json::Value = serde_json::from_slice(&stdout).unwrap();
    assert_eq!(result, serde_json::json!([value]));
    assert!(stdout.len() > 2 * 1024 * 1024);
}
