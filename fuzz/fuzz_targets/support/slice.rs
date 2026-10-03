use arbitrary::Arbitrary;
use htmlcut_core::{Boundary, CompiledPlan, ExtractionPlan, Selection, Strategy};

#[derive(Arbitrary, Debug)]
pub struct SliceInput {
    html: String,
    start: String,
    end: String,
    regex: bool,
    include_start: bool,
    include_end: bool,
}

pub fn drive(input: SliceInput) {
    let Some(document) = crate::snapshot::document(&input.html) else {
        return;
    };
    let boundary = |text: &str| {
        if input.regex {
            Boundary::Regex {
                pattern: crate::snapshot::text(text, 128).into(),
                flags: String::new(),
            }
        } else {
            Boundary::Literal {
                value: crate::snapshot::text(text, 128).into(),
            }
        }
    };
    let Ok(mut plan) = ExtractionPlan::slice(boundary(&input.start), boundary(&input.end)) else {
        return;
    };
    if let Strategy::Slice {
        include_start,
        include_end,
        ..
    } = &mut plan.strategy
    {
        *include_start = input.include_start;
        *include_end = input.include_end;
    }
    plan.selection = Selection::All {
        min: 0,
        max: Some(128),
    };
    plan.limits.max_candidates = 512;
    plan.limits.max_selected = 128;
    plan.limits.max_work = 20_000;
    plan.limits.max_value_bytes = 8192;
    if let Ok(compiled) = CompiledPlan::compile(&plan)
        && let Ok(result) = document.execute(&compiled)
    {
        for (value, range) in result
            .data
            .as_values()
            .unwrap()
            .iter()
            .zip(result.receipt.ranges.as_ref().unwrap())
        {
            assert_eq!(value, &document.snapshot().html()[range.start..range.end]);
        }
    }
}
