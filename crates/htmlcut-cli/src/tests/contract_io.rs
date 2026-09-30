use super::*;
use htmlcut_core::{ErrorCode, ExtractionError};
use std::io::{self, Read, Write};

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
fn t03_strict_decoding_boms_and_expansion_boundaries() {
    assert_eq!(crate::input::decode(b"a\r\nb", None).unwrap(), "a\r\nb");
    assert_eq!(crate::input::decode(b"\xef\xbb\xbfa", None).unwrap(), "a");
    assert_eq!(
        crate::input::decode(b"\xff\xfeA\0", Some("utf-16le")).unwrap(),
        "A"
    );
    for (bytes, encoding) in [
        (&b"\xff"[..], None),
        (&b"\xff\xfeA\0"[..], None),
        (&b"\xff\xfeA\0"[..], Some("utf-16be")),
        (&b"\xef\xbb\xbfa"[..], Some("windows-1252")),
        (&b"a"[..], Some("unknown-encoding")),
    ] {
        assert_eq!(
            crate::input::decode(bytes, encoding).unwrap_err().code,
            ErrorCode::Decoding
        );
    }
    assert_eq!(
        crate::input::decode(&[0x80], Some("windows-1252")).unwrap(),
        "€"
    );
    for (maximum, valid) in [(2, false), (3, true), (4, true)] {
        assert_eq!(
            crate::input::decode_with_limit(&[0x80], Some("windows-1252"), maximum).is_ok(),
            valid
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
        ["htmlcut", "extract", "--stdin", "--css", "p"],
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
            ["htmlcut", "extract", "--stdin", "--css", "p"],
            &mut io::Cursor::new(b"<p>180</p>"),
            &mut writer,
            &mut error,
        );
        assert_eq!(code, 5);
        let error: ExtractionError = serde_json::from_slice(&error).unwrap();
        assert_eq!(error.code, ErrorCode::Publication);
        assert_eq!(error.selected_count, Some(1));
        assert!(error.source_sha256.is_some());
        assert!(!error.message.contains("secret"));
        assert_eq!(writer.bytes.is_empty(), !flush_only);
    }
    let code = app::run(
        ["htmlcut", "extract", "--stdin", "--css", "missing"],
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
            "htmlcut", "extract", "--stdin", "--css", "p", "--match", "all", "--raw",
        ],
        b"<p>A</p><p>B</p>",
    );
    assert_eq!(code, 2);
    assert!(output.is_empty());
    assert!(!error.is_empty());
    let (code, output, error) = invoke(
        &["htmlcut", "extract", "--stdin", "--css", "p", "--raw"],
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
