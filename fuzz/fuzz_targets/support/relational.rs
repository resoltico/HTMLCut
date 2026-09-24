use std::num::NonZeroU32;

use arbitrary::Arbitrary;
use htmlcut_core::interop::v2::{
    CssSelectorText, ExecutionBudget, HtmlInput, Output, Plan, PlanStrategy, PreparationLimits,
    Rendering, Selection, TextWhitespace, compile_plan, execute, prepare_document,
};

#[derive(Arbitrary, Debug)]
pub struct RelationalInput {
    width: u8,
    selector_work_units: u8,
}

pub fn drive(input: RelationalInput) {
    let width = usize::from(input.width % 64 + 1);
    let html = format!(
        "<main>{}</main>",
        "<section><span>candidate</span></section>".repeat(width)
    );
    let Ok(html_input) = HtmlInput::new("relational-selector-fuzz", html) else {
        return;
    };
    let Ok(document) = prepare_document(html_input, PreparationLimits::default()) else {
        return;
    };
    let selector = CssSelectorText::new("section:has(span)").expect("fixed selector is valid");
    let budget = ExecutionBudget {
        max_selector_work_units: non_zero(u32::from(input.selector_work_units % 32) + 1),
        max_candidates: non_zero(128),
        max_selected_matches: non_zero(128),
        max_match_output_bytes: non_zero(8_192),
        max_total_output_bytes: non_zero(1_048_576),
    };
    let plan = Plan::new(
        PlanStrategy::css_selector(selector),
        Selection::all(),
        Output::plain_text(),
        Rendering::new(TextWhitespace::Normalize, false),
    )
    .with_execution_budget(budget);
    let Ok(compiled) = compile_plan(&plan) else {
        return;
    };
    let _ = execute(&document, &compiled);
}

fn non_zero(value: u32) -> NonZeroU32 {
    NonZeroU32::new(value).expect("fixed fuzz budget is non-zero")
}
