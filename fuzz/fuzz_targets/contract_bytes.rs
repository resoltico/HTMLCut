// SPDX-License-Identifier: MPL-2.0
#![no_main]
use libfuzzer_sys::fuzz_target;
fuzz_target!(|bytes: &[u8]| {
    if let Ok(text) = std::str::from_utf8(bytes) {
        let _ = htmlcut_core::Contract::from_json(text);
    }
});
