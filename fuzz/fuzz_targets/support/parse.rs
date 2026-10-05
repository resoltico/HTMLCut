// SPDX-License-Identifier: MPL-2.0
use htmlcut_core::{CompiledPlan, ExtractionPlan};

pub fn drive(data: &[u8]) {
    let Ok(html) = std::str::from_utf8(data.get(..data.len().min(32768)).unwrap()) else {
        return;
    };
    let Some(document) = crate::snapshot::document(html) else {
        return;
    };
    let compiled = CompiledPlan::compile(&ExtractionPlan::css("body").unwrap()).unwrap();
    match (document.execute(&compiled), document.execute(&compiled)) {
        (Ok(first), Ok(second)) => {
            assert_eq!(first.payload(), second.payload());
            assert_eq!(first.receipt(), second.receipt());
        }
        (Err(first), Err(second)) => assert_eq!(first, second),
        _ => panic!("repeated parsing changed success state"),
    }
}
