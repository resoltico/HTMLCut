//! Records the prepared-engine side of the v13-to-v14 operational benchmark.
//!
//! The companion `scripts/benchmark-prepared-engine.sh` program measures this example against the
//! immutable `v13.2.0` CLI workflow. This executable deliberately reports model facts only; its
//! wall-clock duration is not a correctness assertion.

#![forbid(unsafe_code)]

use std::fmt::Write as _;

use htmlcut_core::interop::v2::{
    CssSelectorText, HtmlInput, Output, Plan, PlanStrategy, PreparationLimits, Rendering,
    Selection, TextWhitespace, compile_plan, execute, prepare_document,
};

const PLAN_COUNT: usize = 50;
const DOCUMENT_ARTICLE_COUNT: usize = 1_024;

fn main() {
    let document = prepare_document(
        HtmlInput::new("prepared-engine-benchmark", benchmark_html()).expect("benchmark source"),
        PreparationLimits::default(),
    )
    .expect("prepare benchmark document");
    let plans = (0..PLAN_COUNT)
        .map(compiled_plan)
        .collect::<Result<Vec<_>, _>>()
        .expect("compile benchmark plans");
    let selected_match_count = plans
        .iter()
        .map(|plan| execute(&document, plan).expect("execute benchmark plan"))
        .map(|result| result.selected_matches.len())
        .sum::<usize>();

    assert_eq!(
        selected_match_count, PLAN_COUNT,
        "every benchmark plan must select exactly its own article"
    );
    println!(
        concat!(
            "{{\"workflow\":\"prepared_engine\",",
            "\"prepared_document_count\":1,",
            "\"full_document_parse_count\":1,",
            "\"compiled_plan_count\":{PLAN_COUNT},",
            "\"execution_count\":{PLAN_COUNT},",
            "\"selected_match_count\":{selected_match_count}}}"
        ),
        PLAN_COUNT = PLAN_COUNT,
        selected_match_count = selected_match_count
    );
}

fn compiled_plan(
    index: usize,
) -> Result<htmlcut_core::interop::v2::CompiledPlan, Box<htmlcut_core::interop::v2::InteropError>> {
    let selector = CssSelectorText::new(format!("article[data-benchmark-index=\"{index}\"]"))
        .expect("benchmark selector text");
    compile_plan(&Plan::new(
        PlanStrategy::css_selector(selector),
        Selection::single(),
        Output::text(),
        Rendering::new(TextWhitespace::Normalize, false),
    ))
}

fn benchmark_html() -> String {
    let mut html = String::from("<!doctype html><html><body>");
    for index in 0..DOCUMENT_ARTICLE_COUNT {
        if index < PLAN_COUNT {
            write!(
                html,
                "<article data-benchmark-index=\"{index}\"><h1>Headline {index}</h1><p>Prepared engine benchmark content.</p></article>"
            )
            .expect("write benchmark document");
        } else {
            write!(
                html,
                "<article><h1>Background {index}</h1><p>Prepared engine benchmark content.</p></article>"
            )
            .expect("write benchmark document");
        }
    }
    html.push_str("</body></html>");
    html
}
