#![cfg_attr(not(test), cfg_attr(feature = "fuzzing", no_main))]

#[cfg(all(feature = "fuzzing", not(test)))]
use libfuzzer_sys::fuzz_target;

#[cfg(all(feature = "fuzzing", not(test)))]
#[path = "support/interop.rs"]
mod interop;
#[cfg(all(feature = "fuzzing", not(test)))]
#[path = "support/interop_common.rs"]
mod interop_common;

#[cfg(all(feature = "fuzzing", not(test)))]
fuzz_target!(|input: interop::InteropInput| {
    interop::drive(input);
});

#[cfg(any(test, not(feature = "fuzzing")))]
fn main() {}
