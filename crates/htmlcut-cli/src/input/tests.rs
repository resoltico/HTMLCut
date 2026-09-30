use super::*;
use std::io::Cursor;

#[test]
fn decoding_expansion_chunking_and_expired_deadline_never_publish_partial_unicode() {
    let bytes = vec![0x80; 9000];
    assert_eq!(
        decode_with_limit(&bytes, Some("windows-1252"), 27000)
            .unwrap()
            .len(),
        27000
    );
    assert_eq!(
        decode_with_limit(&bytes, Some("windows-1252"), 26999)
            .unwrap_err()
            .code,
        ErrorCode::ResourceLimit
    );
    let deadline = std::time::Instant::now() - std::time::Duration::from_secs(1);
    assert_eq!(
        decode_until(b"valid", None, 1024, Some(deadline))
            .unwrap_err()
            .code,
        ErrorCode::ResourceLimit
    );
    assert!(
        SourceSpec::Stdin
            .acquire(
                &mut Cursor::new(b"body"),
                Some("unknown"),
                None,
                Path::new(".")
            )
            .is_err()
    );
    assert!(
        SourceSpec::Stdin
            .acquire(
                &mut Cursor::new(b"body"),
                None,
                Some("invalid"),
                Path::new(".")
            )
            .is_err()
    );
}

#[test]
fn missing_runtime_environment_and_closed_source_validation_are_typed() {
    let source = SourceSpec::Http {
        url: None,
        url_env: Some("HTMLCUT_TEST_UNSET_987654321_PRIVATE".into()),
    };
    assert_eq!(
        source
            .acquire(&mut Cursor::new(b""), None, None, Path::new("."))
            .unwrap_err()
            .code,
        ErrorCode::InvalidOptions
    );
    assert!(valid_env_name("_USER1"));
    assert!(!valid_env_name(""));
    assert!(!valid_env_name("1USER"));
    assert!(!valid_env_name(&"U".repeat(257)));
    let malformed = SourceSpec::Http {
        url: Some("https://example.test".into()),
        url_env: Some("USER".into()),
    };
    assert!(
        malformed
            .acquire(&mut Cursor::new(b""), None, None, Path::new("."))
            .is_err()
    );
}

#[test]
fn deadlines_cover_post_decode_and_post_snapshot_without_accepting_partial_input() {
    use std::cell::Cell;
    use std::time::{Duration, Instant};
    let source = SourceSpec::Http {
        url: Some("https://example.test/source".into()),
        url_env: None,
    };
    let base = Instant::now();
    let deadline = base + Duration::from_secs(10);
    for expires_at in [1, 2, 3] {
        let calls = Cell::new(0);
        let mut adapter = Acquisition {
            fetch: |url: &str, explicit: bool| {
                assert_eq!(url, "https://example.test/source");
                assert!(!explicit);
                Ok(http::Acquired {
                    bytes: b"<p>complete</p>".to_vec(),
                    charset: None,
                    final_url: "https://example.test/final".into(),
                    deadline,
                })
            },
            now: || {
                calls.set(calls.get() + 1);
                if calls.get() >= expires_at {
                    deadline
                } else {
                    base
                }
            },
        };
        let result = source.acquire_with(
            &mut Cursor::new(b""),
            None,
            None,
            Path::new("."),
            &mut adapter,
        );
        if expires_at <= 2 {
            let error = result.unwrap_err();
            assert_eq!(error.code, ErrorCode::ResourceLimit);
            assert_eq!(error.stage, "acquisition");
        } else {
            let snapshot = result.unwrap();
            assert_eq!(snapshot.html(), "<p>complete</p>");
            assert_eq!(
                snapshot.metadata().base_url.as_deref(),
                Some("https://example.test/final")
            );
        }
        assert_eq!(calls.get(), expires_at.min(2));
    }
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("invalid.html"), [0xff]).unwrap();
    assert_eq!(
        SourceSpec::File {
            path: "invalid.html".into()
        }
        .acquire(&mut Cursor::new(b""), None, None, directory.path())
        .unwrap_err()
        .code,
        ErrorCode::Decoding
    );
}

#[test]
fn malformed_provider_metadata_cannot_become_an_accepted_snapshot() {
    let source = SourceSpec::Http {
        url: Some("https://example.test/source".into()),
        url_env: None,
    };
    let now = std::time::Instant::now();
    let mut adapter = Acquisition {
        fetch: |_: &str, _: bool| {
            Ok(http::Acquired {
                bytes: b"valid".to_vec(),
                charset: None,
                final_url: "http://[".into(),
                deadline: now + std::time::Duration::from_secs(10),
            })
        },
        now: || now,
    };
    assert_eq!(
        source
            .acquire_with(
                &mut Cursor::new(b""),
                None,
                None,
                Path::new("."),
                &mut adapter
            )
            .unwrap_err()
            .code,
        ErrorCode::InvalidBaseUrl
    );
}
