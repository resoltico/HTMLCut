// SPDX-License-Identifier: MPL-2.0
use arbitrary::Arbitrary;
use htmlcut_core::{CompiledPlan, ExtractionPlan, Selection};

#[derive(Arbitrary, Debug)]
pub struct RelationalInput {
    width: u8,
    work: u8,
}

pub fn drive(input: RelationalInput) {
    let html = format!(
        "<main>{}</main>",
        "<section><span>candidate</span></section>".repeat(usize::from(input.width % 64 + 1))
    );
    let Some(document) = crate::snapshot::document(&html) else {
        return;
    };
    let mut plan = ExtractionPlan::css("section:has(span)").unwrap();
    plan.selection = Selection::All {
        min: 0,
        max: Some(128),
    };
    plan.limits.max_work = u32::from(input.work) + 1;
    if let Ok(compiled) = CompiledPlan::compile(&plan) {
        let _ = document.execute(&compiled);
    }
}
