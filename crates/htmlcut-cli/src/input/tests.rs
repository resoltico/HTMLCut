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
