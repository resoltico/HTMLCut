use super::*;
use htmlcut_core::{ErrorCode, ExtractionError};
use std::io::{self, Read, Write};

#[cfg(unix)]
#[test]
fn non_utf8_file_options_and_resolved_replay_paths_are_typed_failures() {
    use std::os::unix::ffi::OsStringExt;
    #[cfg(not(target_os = "macos"))]
    use std::os::unix::fs::symlink;
    let mut args: Vec<std::ffi::OsString> = ["htmlcut", "extract", "--file"]
        .into_iter()
        .map(Into::into)
        .collect();
    args.push(std::ffi::OsString::from_vec(vec![0xff]));
    args.extend(["--css", "p"].into_iter().map(Into::into));
    let mut output = Vec::new();
    let mut errors = Vec::new();
    assert_eq!(
        app::run(args, &mut io::empty(), &mut output, &mut errors),
        2
    );
    assert!(output.is_empty());
    assert_eq!(
        invoke(
            &[
                "htmlcut",
                "extract",
                "--file",
                "/htmlcut-missing-replay-source",
                "--css",
                "p",
                "--save-run",
                "/htmlcut-missing-run"
            ],
            b""
        )
        .0,
        5
    );
    // APFS rejects invalid UTF-8 names; exercise the actual filesystem route on Unix filesystems
    // that support such names, while the conversion decision is tested on every Unix host above.
    #[cfg(not(target_os = "macos"))]
    {
        let root = htmlcut_tempdir::tempdir().unwrap();
        let invalid = root.path().join(std::ffi::OsString::from_vec(vec![0xff]));
        std::fs::create_dir(&invalid).unwrap();
        std::fs::write(invalid.join("page.html"), "<p>value</p>").unwrap();
        let alias = root.path().join("page.html");
        symlink(invalid.join("page.html"), &alias).unwrap();
        let run = root.path().join("run.json");
        let args = [
            "htmlcut",
            "extract",
            "--file",
            alias.to_str().unwrap(),
            "--css",
            "p",
            "--save-run",
            run.to_str().unwrap(),
        ];
        assert_eq!(invoke(&args, b"").0, 2);
        assert!(!run.exists());
        std::fs::remove_file(&alias).unwrap();
        assert_eq!(invoke(&args, b"").0, 5);
    }
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
            "--css",
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

#[test]
fn t31_acquisition_time_destination_races_cannot_publish_saved_runs_or_audits() {
    struct RacingInput<'a> {
        target: &'a std::path::Path,
        body: io::Cursor<&'static [u8]>,
        raced: bool,
    }
    impl Read for RacingInput<'_> {
        fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
            if !self.raced {
                std::fs::create_dir(self.target)?;
                self.raced = true;
            }
            self.body.read(output)
        }
    }
    for flag in ["--save-run", "--audit"] {
        let root = tempfile::tempdir().unwrap();
        let target = root.path().join("evidence.json");
        let mut arguments = vec![
            "htmlcut",
            "extract",
            "--stdin",
            "--css",
            "p",
            flag,
            target.to_str().unwrap(),
        ];
        if flag == "--audit" {
            arguments.extend(["--audit-field", "counts"]);
        }
        let mut input = RacingInput {
            target: &target,
            body: io::Cursor::new(b"<p>value</p>"),
            raced: false,
        };
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        assert_eq!(app::run(arguments, &mut input, &mut stdout, &mut stderr), 5);
        assert!(stdout.is_empty());
        assert!(target.is_dir());
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&stderr).unwrap()["code"],
            "publication"
        );
    }
}

#[test]
fn t20_t29_default_json_accepts_a_complete_large_value_with_escaping() {
    let value = "\"".repeat(1024 * 1024);
    let html = format!("<p>{value}</p>");
    let (code, stdout, stderr) = invoke(
        &["htmlcut", "extract", "--stdin", "--css", "p"],
        html.as_bytes(),
    );
    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&stderr));
    let result: serde_json::Value = serde_json::from_slice(&stdout).unwrap();
    assert_eq!(result["values"], serde_json::json!([value]));
    assert!(stdout.len() > 2 * 1024 * 1024);
}
