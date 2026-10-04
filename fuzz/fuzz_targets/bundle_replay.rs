// SPDX-License-Identifier: MPL-2.0
#![cfg_attr(not(test), cfg_attr(feature = "fuzzing", no_main))]
#[cfg(all(feature = "fuzzing", not(test)))]
use libfuzzer_sys::fuzz_target;
#[cfg(all(feature = "fuzzing", not(test)))]
#[path = "support/cli.rs"]
#[allow(dead_code)]
mod cli;
#[cfg(all(feature = "fuzzing", not(test)))]
use cli::{
    bundle, bundle_io, command, command_diagnostics, input, operation_metadata, publication,
};
#[cfg(all(feature = "fuzzing", not(test)))]
fuzz_target!(|data: &[u8]| {
    use htmlcut_core::*;
    let bytes = &data[..data.len().min(32768)];
    let _ = bundle::read_from(std::io::Cursor::new(bytes), bytes.len() as u64);
    let document = PreparedDocument::new(
        SourceSnapshot::new("<p>value</p>", Default::default()).unwrap(),
        Default::default(),
    )
    .unwrap();
    let plan = CompiledPlan::compile(&ExtractionPlan::css("p").unwrap()).unwrap();
    let expected = document.execute(&plan).unwrap();
    let mut valid = Vec::new();
    bundle::write(&mut valid, &document, &plan, &expected.receipt).unwrap();
    let replay = bundle::read_from(std::io::Cursor::new(&valid), valid.len() as u64).unwrap();
    assert_eq!(replay.document.execute(&replay.plan).unwrap(), expected);
    assert_eq!(replay.expected, expected.receipt);
    if let Some(first) = data.first() {
        let position = usize::from(*first) * 31 % valid.len();
        valid[position] ^= data.get(1).copied().unwrap_or(1);
        if let Ok(replay) = bundle::read_from(std::io::Cursor::new(&valid), valid.len() as u64) {
            let _ = replay
                .document
                .execute(&replay.plan)
                .map(|result| result.receipt == replay.expected);
        }
    }
});
#[cfg(any(test, not(feature = "fuzzing")))]
fn main() {}
