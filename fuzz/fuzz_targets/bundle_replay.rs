// SPDX-License-Identifier: MPL-2.0
#![cfg_attr(not(test), cfg_attr(feature = "fuzzing", no_main))]
#[cfg(all(feature = "fuzzing", not(test)))]
use libfuzzer_sys::fuzz_target;
#[cfg(all(feature = "fuzzing", not(test)))]
#[path = "support/cli.rs"]
#[allow(dead_code)]
mod cli;
#[cfg(all(feature = "fuzzing", not(test)))]
use cli::{bundle, bundle_io, command, command_diagnostics, input, publication};
#[cfg(all(feature = "fuzzing", not(test)))]
fuzz_target!(|data: &[u8]| {
    use htmlcut_core::*;
    let bytes = &data[..data.len().min(32768)];
    let _ = bundle::read_from(std::io::Cursor::new(bytes), bytes.len() as u64);
    let document = PreparedDocument::new(
        SourceSnapshot::new(
            format!(
                "<p>value</p><!--{}-->",
                "x".repeat(usize::from(data.first().copied().unwrap_or(0)) * 65)
            ),
            Default::default(),
        )
        .unwrap(),
        Default::default(),
    )
    .unwrap();
    let mut query = ExtractionPlan::css("p").unwrap();
    query.limits.max_work = u32::from(data.get(1).copied().unwrap_or(255)) + 1;
    let plan = CompiledPlan::compile(&query).unwrap();
    // Warm explicit identities must not buy logical evidence work.
    let _ = document.snapshot().source_sha256();
    let _ = plan.plan_sha256();
    let cold_document = PreparedDocument::new(
        SourceSnapshot::new(document.snapshot().html(), Default::default()).unwrap(),
        Default::default(),
    )
    .unwrap();
    let cold_plan = CompiledPlan::compile(&query).unwrap();
    let first = document.execute(&plan);
    let second = cold_document.execute(&cold_plan);
    let (expected, cold) = match (first, second) {
        (Ok(first), Ok(second)) => (first, second),
        (Err(first), Err(second)) => {
            assert_eq!(first, second);
            return;
        }
        _ => panic!("fresh execution changed success state"),
    };
    assert_eq!(expected.payload(), cold.payload());
    let evidence = expected.receipt();
    assert_eq!(evidence, cold.receipt());
    assert_eq!(evidence, expected.receipt());
    if evidence.is_err() {
        return;
    }
    let mut valid = Vec::new();
    bundle::write(&mut valid, &expected).unwrap();
    let replay = bundle::read_from(std::io::Cursor::new(&valid), valid.len() as u64).unwrap();
    let actual = replay.document.execute(&replay.plan).unwrap();
    assert_eq!(actual.payload(), expected.payload());
    assert_eq!(actual.receipt().unwrap(), expected.receipt().unwrap());
    assert_eq!(&replay.expected, expected.receipt().unwrap());
    if let Some(first) = data.first() {
        let position = usize::from(*first) * 31 % valid.len();
        valid[position] ^= data.get(1).copied().unwrap_or(1);
        if let Ok(replay) = bundle::read_from(std::io::Cursor::new(&valid), valid.len() as u64) {
            let _ = replay.document.execute(&replay.plan).map(|result| {
                result
                    .receipt()
                    .is_ok_and(|receipt| *receipt == replay.expected)
            });
        }
    }
});
#[cfg(any(test, not(feature = "fuzzing")))]
fn main() {}
