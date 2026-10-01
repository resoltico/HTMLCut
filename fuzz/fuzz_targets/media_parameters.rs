#![cfg_attr(not(test), cfg_attr(feature = "fuzzing", no_main))]
#[cfg(all(feature = "fuzzing", not(test)))]
use libfuzzer_sys::fuzz_target;
#[cfg(all(feature = "fuzzing", not(test)))]
#[path = "../../crates/htmlcut-cli/src/input.rs"]
// This parser target reuses only the maintained error constructors, not acquisition I/O.
#[allow(dead_code)]
mod input;
#[cfg(all(feature = "fuzzing", not(test)))]
#[path = "../../crates/htmlcut-cli/src/input/http/media_type.rs"]
mod media_type;
#[cfg(all(feature = "fuzzing", not(test)))]
fuzz_target!(|data: &[u8]| {
    let bytes = &data[..data.len().min(65537)];
    if let Err(error) = media_type::charset(Some(bytes)) {
        assert!(matches!(error.code.exit_class(), 4 | 5));
    }
    let note: String = bytes
        .iter()
        .take(512)
        .map(|byte| char::from(b'a' + byte % 26))
        .collect();
    for header in [
        format!("text/html; note=\"{note};charset=windows-1252\"; charset=utf-8"),
        format!("text/html; charset=\"u\\tf-8\"; note=\"{note};charset=unknown\""),
    ] {
        assert_eq!(
            media_type::charset(Some(header.as_bytes())).unwrap(),
            Some("utf-8".into())
        );
    }
    let header = format!("text/html; note=\"{note};charset=unknown\"");
    assert_eq!(media_type::charset(Some(header.as_bytes())).unwrap(), None);
    assert!(media_type::charset(Some(b"text/html; charset=utf-8; charset=windows-1252")).is_err());
});
#[cfg(any(test, not(feature = "fuzzing")))]
fn main() {}
