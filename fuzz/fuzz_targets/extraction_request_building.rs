#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;

#[path = "support.rs"]
mod support;

#[derive(Arbitrary, Debug)]
struct InteropInput {
    html: String,
    strategy: support::FuzzInteropStrategy,
    value_kind: support::FuzzValueKind,
    selection: support::FuzzSelection,
    normalization: support::FuzzNormalization,
}

fuzz_target!(|input: InteropInput| {
    support::drive_interop_request(
        &input.html,
        input.strategy,
        input.value_kind,
        input.selection,
        input.normalization,
    );
});
