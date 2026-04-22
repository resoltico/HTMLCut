#![no_main]

use libfuzzer_sys::fuzz_target;

#[path = "support/request_common.rs"]
mod request_common;
#[path = "support/selector.rs"]
mod selector;

fuzz_target!(|input: selector::SelectorInput| {
    selector::drive(input);
});
