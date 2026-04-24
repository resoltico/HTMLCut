#![cfg_attr(not(test), cfg_attr(feature = "fuzzing", no_main))]

#[cfg(all(feature = "fuzzing", not(test)))]
use libfuzzer_sys::fuzz_target;

#[cfg(all(feature = "fuzzing", not(test)))]
#[path = "support/request_common.rs"]
mod request_common;
#[cfg(all(feature = "fuzzing", not(test)))]
#[path = "support/slice.rs"]
mod slice;

#[cfg(all(feature = "fuzzing", not(test)))]
fuzz_target!(|input: slice::SliceInput| {
    slice::drive(input);
});

#[cfg(any(test, not(feature = "fuzzing")))]
fn main() {}
