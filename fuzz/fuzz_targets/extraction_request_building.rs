#![no_main]

use libfuzzer_sys::fuzz_target;

#[path = "support/interop.rs"]
mod interop;
#[path = "support/interop_common.rs"]
mod interop_common;

fuzz_target!(|input: interop::InteropInput| {
    interop::drive(input);
});
