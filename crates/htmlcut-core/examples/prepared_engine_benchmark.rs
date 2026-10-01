//! Records the reusable prepared-engine model for the current extraction contract.
//!
//! The companion `scripts/benchmark-prepared-engine.sh` program measures this example against the
//! immutable `v13.2.0` CLI workflow. This executable deliberately reports model facts only; its
//! wall-clock duration is not a correctness assertion.

#![forbid(unsafe_code)]

use std::fmt::Write as _;

use htmlcut_core::{
    CompiledPlan, ExtractionPlan, PreparationLimits, PreparedDocument, SnapshotMetadata,
    SourceSnapshot,
};

const PLAN_COUNT: usize = 50;
const DOCUMENT_ARTICLE_COUNT: usize = 1_024;

fn main() {
    let document = PreparedDocument::new(
        SourceSnapshot::new(benchmark_html(), SnapshotMetadata::default())
            .expect("benchmark source"),
        PreparationLimits::default(),
    )
    .expect("prepare benchmark document");
    let plans = (0..PLAN_COUNT)
        .map(compiled_plan)
        .collect::<Result<Vec<_>, _>>()
        .expect("compile benchmark plans");
    let selected_match_count = plans
        .iter()
        .map(|plan| document.execute(plan).expect("execute benchmark plan"))
        .map(|result| result.values.len())
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

fn compiled_plan(index: usize) -> Result<CompiledPlan, htmlcut_core::ExtractionError> {
    CompiledPlan::compile(&ExtractionPlan::css(format!(
        "article[data-benchmark-index=\"{index}\"]"
    ))?)
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
