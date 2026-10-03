#![cfg_attr(not(test), cfg_attr(feature = "fuzzing", no_main))]
#[cfg(all(feature = "fuzzing", not(test)))]
use libfuzzer_sys::fuzz_target;
#[cfg(all(feature = "fuzzing", not(test)))]
#[path = "support/inspection.rs"]
mod inspection;
#[cfg(all(feature = "fuzzing", not(test)))]
#[path = "support/snapshot.rs"]
mod snapshot;
#[cfg(all(feature = "fuzzing", not(test)))]
fuzz_target!(|input: inspection::InspectionInput| {
    inspection::drive(input);
});
#[cfg(any(test, not(feature = "fuzzing")))]
fn main() {}
