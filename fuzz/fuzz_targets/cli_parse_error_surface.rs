#![cfg_attr(not(test), cfg_attr(feature = "fuzzing", no_main))]

#[cfg(all(feature = "fuzzing", not(test)))]
use libfuzzer_sys::fuzz_target;

#[cfg(all(feature = "fuzzing", not(test)))]
#[path = "support/cli.rs"]
mod cli;

#[cfg(all(feature = "fuzzing", not(test)))]
fuzz_target!(|input: &[u8]| {
    cli::drive(input);
});

#[cfg(any(test, not(feature = "fuzzing")))]
fn main() {}
