#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;

#[path = "support.rs"]
mod support;

#[derive(Arbitrary, Debug)]
struct SelectorInput {
    html: String,
    selector: String,
    value_kind: support::FuzzValueKind,
    selection: support::FuzzSelection,
    normalization: support::FuzzNormalization,
}

fuzz_target!(|input: SelectorInput| {
    support::drive_selector_request(
        &input.html,
        &input.selector,
        input.value_kind,
        input.selection,
        input.normalization,
    );
});
