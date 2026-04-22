#![no_main]

use libfuzzer_sys::fuzz_target;

#[path = "support/request_common.rs"]
mod request_common;
#[path = "support/slice.rs"]
mod slice;

fuzz_target!(|input: slice::SliceInput| {
    slice::drive(input);
});
