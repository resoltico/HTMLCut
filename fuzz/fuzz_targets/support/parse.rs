use htmlcut_core::{CompiledPlan, ExtractionPlan};

pub fn drive(data: &[u8]) {
    let Ok(html) = std::str::from_utf8(data.get(..data.len().min(32768)).unwrap()) else {
        return;
    };
    let Some(document) = crate::snapshot::document(html) else {
        return;
    };
    let compiled = CompiledPlan::compile(&ExtractionPlan::css("body").unwrap()).unwrap();
    assert_eq!(document.execute(&compiled), document.execute(&compiled));
}
