#![no_main]

use libfuzzer_sys::fuzz_target;

#[path = "support/parse.rs"]
mod parse_support;

fuzz_target!(|data: &[u8]| {
    parse_support::drive(data);
});
