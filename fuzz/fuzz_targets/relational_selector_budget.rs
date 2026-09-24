#![cfg_attr(not(test), cfg_attr(feature = "fuzzing", no_main))]

#[cfg(all(feature = "fuzzing", not(test)))]
use libfuzzer_sys::fuzz_target;

#[cfg(all(feature = "fuzzing", not(test)))]
#[path = "support/relational.rs"]
mod relational;

#[cfg(all(feature = "fuzzing", not(test)))]
fuzz_target!(|input: relational::RelationalInput| {
    relational::drive(input);
});

#[cfg(any(test, not(feature = "fuzzing")))]
fn main() {}
