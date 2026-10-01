#![cfg_attr(not(test), cfg_attr(feature = "fuzzing", no_main))]
#[cfg(all(feature = "fuzzing", not(test)))]
use libfuzzer_sys::fuzz_target;
#[cfg(all(feature = "fuzzing", not(test)))]
#[path = "../../crates/htmlcut-cli/src/app.rs"]
mod app;
#[cfg(all(feature = "fuzzing", not(test)))]
#[path = "../../crates/htmlcut-cli/src/command.rs"]
mod command;
#[cfg(all(feature = "fuzzing", not(test)))]
#[path = "../../crates/htmlcut-cli/src/command_diagnostics.rs"]
mod command_diagnostics;
#[cfg(all(feature = "fuzzing", not(test)))]
#[path = "../../crates/htmlcut-cli/src/evidence.rs"]
mod evidence;
#[cfg(all(feature = "fuzzing", not(test)))]
#[path = "../../crates/htmlcut-cli/src/input.rs"]
mod input;
#[cfg(all(feature = "fuzzing", not(test)))]
#[path = "../../crates/htmlcut-cli/src/input/http/media_type.rs"]
mod media_type;
#[cfg(all(feature = "fuzzing", not(test)))]
#[path = "../../crates/htmlcut-cli/src/operation_metadata.rs"]
mod operation_metadata;
#[cfg(all(feature = "fuzzing", not(test)))]
#[path = "../../crates/htmlcut-cli/src/publication.rs"]
mod publication;
#[cfg(all(feature = "fuzzing", not(test)))]
fuzz_target!(|data: &[u8]| {
    let suffix = String::from_utf8_lossy(&data[..data.len().min(4096)]);
    let unknown = format!("--unrecognized-{suffix}");
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let code = app::run(
        ["htmlcut", "extract", "--stdin", "--css", "p", &unknown],
        &mut std::io::Cursor::new(b"<p>value</p>"),
        &mut stdout,
        &mut stderr,
    );
    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    let error: serde_json::Value = serde_json::from_slice(&stderr).unwrap();
    assert_eq!(error["schema"], "htmlcut.extraction.error");
});
#[cfg(any(test, not(feature = "fuzzing")))]
fn main() {}
