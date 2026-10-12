// SPDX-License-Identifier: MPL-2.0
use htmlcut_core::*;
use serde_json::json;
fn run(source: &str, plan: serde_json::Value) -> Result<ExtractionResult, ExtractionError> {
    let compiled = CompiledPlan::compile(&ExtractionPlan::from_json(
        &serde_json::to_vec(&plan).unwrap(),
    )?)?;
    PreparedDocument::new(
        SourceSnapshot::new(source, Default::default())?,
        Default::default(),
    )?
    .execute(&compiled)
}
#[test]
fn adjacent_row_content_is_outside_the_record_even_when_selector_context_matches() {
    let source = "<table><tr class=entry><td>A</td></tr><tr><td class=score>10</td></tr></table>";
    let plan = json!({"version":8,"select":"tr.entry","fields":{"title":{"select":":scope td","read":"literal"},"score":{"select":":scope + tr .score","match":"optional"}}});
    let result = run(source, plan).unwrap();
    assert_eq!(
        serde_json::to_value(result.data()).unwrap(),
        json!([{"score":null,"title":"A"}])
    );
    assert_eq!(result.field_counts()["score"].candidate_count, 0);
}
#[test]
fn original_ancestors_and_siblings_can_assert_context_without_expanding_payload() {
    let source = "<main><h2>Invoices</h2><article><p id=value>10</p></article><aside><p>wrong</p></aside></main>";
    let plan = json!({"version":8,"select":"article","fields":{"value":{"select":"h2 + article > p","read":"literal"}},"expect":[{"scope":"selected","select":":scope > p","equals":"10"}]});
    assert_eq!(
        serde_json::to_value(run(source, plan).unwrap().data()).unwrap(),
        json!([{"value":"10"}])
    );
}
#[test]
fn retired_readings_and_forests_are_closed_contract_errors() {
    for read in ["markdown", "resolved-markdown", "inner-html"] {
        assert!(
            ExtractionPlan::from_json(
                &serde_json::to_vec(&json!({"version":8,"select":"p","read":read})).unwrap()
            )
            .is_err()
        );
    }
    for n in [0, 1, 64] {
        assert!(
            ExtractionPlan::from_json(
                &serde_json::to_vec(&json!({"version":8,"select":"p","following_siblings":n}))
                    .unwrap()
            )
            .is_err()
        );
    }
    assert!(serde_json::from_str::<PreparationLimits>(r#"{"max_parse_work":1}"#).is_err());
}

#[test]
fn compiled_positional_plan_reuse_has_fresh_document_and_execution_caches() {
    let compiled = CompiledPlan::compile(&ExtractionPlan::css("p:nth-child(2)").unwrap()).unwrap();
    let documents = [
        ("<div><p>A</p><p>B</p><p>C</p></div>", "B"),
        ("<div><b>X</b><p>Y</p></div>", "Y"),
    ]
    .map(|(source, expected)| {
        (
            PreparedDocument::new(
                SourceSnapshot::new(source, Default::default()).unwrap(),
                Default::default(),
            )
            .unwrap(),
            expected,
        )
    });
    for _ in 0..5 {
        for (document, expected) in &documents {
            let reused = document.execute(&compiled).unwrap();
            assert_eq!(reused.candidate_count(), 1);
            assert_eq!(reused.data().as_values().unwrap(), [*expected]);
            let cold =
                CompiledPlan::compile(&ExtractionPlan::css("p:nth-child(2)").unwrap()).unwrap();
            assert_eq!(document.execute(&cold).unwrap().data(), reused.data());
        }
    }
}
