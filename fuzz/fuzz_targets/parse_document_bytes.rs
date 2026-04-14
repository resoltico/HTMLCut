#![no_main]

use libfuzzer_sys::fuzz_target;

#[path = "support.rs"]
mod support;

fuzz_target!(|data: &[u8]| {
    let html = String::from_utf8_lossy(data);
    support::drive_parse_surfaces(html.as_ref());
});
