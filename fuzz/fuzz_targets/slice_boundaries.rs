#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;

#[path = "support.rs"]
mod support;

#[derive(Arbitrary, Debug)]
struct SliceInput {
    html: String,
    start: String,
    end: String,
    regex_mode: bool,
    flags: support::FuzzRegexFlags,
    include_start: bool,
    include_end: bool,
    value_kind: support::FuzzValueKind,
    selection: support::FuzzSelection,
    normalization: support::FuzzNormalization,
}

fuzz_target!(|input: SliceInput| {
    support::drive_slice_request(
        &input.html,
        &input.start,
        &input.end,
        input.regex_mode,
        input.flags,
        input.include_start,
        input.include_end,
        input.value_kind,
        input.selection,
        input.normalization,
    );
});
