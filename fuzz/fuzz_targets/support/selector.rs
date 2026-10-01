use arbitrary::Arbitrary;
use htmlcut_core::{CompiledPlan, ExtractionPlan, Selection};

#[derive(Arbitrary, Debug)]
pub struct SelectorInput {
    html: String,
    selector: String,
    all: bool,
}

pub fn drive(input: SelectorInput) {
    let Some(document) = crate::snapshot::document(&input.html) else {
        return;
    };
    let Ok(mut plan) = ExtractionPlan::css(crate::snapshot::text(&input.selector, 1024)) else {
        return;
    };
    if input.all {
        plan.selection = Selection::All {
            min: 0,
            max: Some(128),
        };
    }
    plan.limits.max_candidates = 512;
    plan.limits.max_selected = 128;
    plan.limits.max_work = 20_000;
    plan.limits.max_value_bytes = 8192;
    plan.limits.max_total_value_bytes = 32768;
    if let Ok(compiled) = CompiledPlan::compile(&plan) {
        let first = document.execute(&compiled);
        let second = document.execute(&compiled);
        assert_eq!(first, second);
    }
}
