#![cfg_attr(not(test), cfg_attr(feature = "fuzzing", no_main))]

#[cfg(all(feature = "fuzzing", not(test)))]
use libfuzzer_sys::fuzz_target;

#[cfg(all(feature = "fuzzing", not(test)))]
#[path = "support/parse.rs"]
mod parse_support;

#[cfg(all(feature = "fuzzing", not(test)))]
fuzz_target!(|data: &[u8]| {
    parse_support::drive(data);
});

#[cfg(any(test, not(feature = "fuzzing")))]
fn main() {}
