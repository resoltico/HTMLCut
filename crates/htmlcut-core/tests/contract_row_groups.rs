// SPDX-License-Identifier: MPL-2.0
//! Original-DOM containment and complete row-group expectations at the public boundary.

use htmlcut_core::*;
use serde_json::json;

fn execute(
    source: &str,
    following: u32,
    optional: bool,
    nth: bool,
) -> Result<ExtractionResult, ExtractionError> {
    let mut plan = json!({"schema":"htmlcut.extraction.plan","version":SCHEMA_VERSION,
    "strategy":{"kind":"css","selector":"tr.entry"},
    "selection":{"kind":"all"},
    "projection":{"kind":"records","following_siblings":following,"fields":[
        {"name":"title","selector":":scope td"},
        {"name":"score","selector":":scope + tr .score","selection":{"kind":"single"}}
    ]}});
    if optional {
        plan["projection"]["fields"][1]["selection"] = json!({"kind":"optional"});
    }
    if nth {
        plan["selection"] = json!({"kind":"nth","index":1});
    }
    execute_plan(source, plan)
}

fn execute_plan(
    source: &str,
    plan: serde_json::Value,
) -> Result<ExtractionResult, ExtractionError> {
    let compiled = CompiledPlan::compile(&ExtractionPlan::from_json(
        &serde_json::to_vec(&plan).unwrap(),
    )?)?;
    PreparedDocument::new(
        SourceSnapshot::new(source, SnapshotMetadata::default())?,
        PreparationLimits::default(),
    )?
    .execute(&compiled)
}

#[test]
fn forest_candidate_ceiling_is_shared_and_context_cannot_return_outside_payload() {
    let source = "<main><article><i class=cell>A</i></article><aside><i class=cell>B</i></aside><aside class=outside>C</aside></main>";
    let mut plan = json!({"schema":"htmlcut.extraction.plan","version":SCHEMA_VERSION,
        "strategy":{"kind":"css","selector":"article"},
        "projection":{"kind":"records","following_siblings":1,"fields":[
            {"name":"cells","selector":".cell","selection":{"kind":"all"}},
            {"name":"outside","selector":":scope ~ .outside","selection":{"kind":"optional"}}
        ]},"limits":{"max_candidates":2}});
    assert_eq!(
        serde_json::to_value(execute_plan(source, plan.clone()).unwrap().data).unwrap(),
        json!([{"cells":["A","B"],"outside":null}])
    );
    plan["limits"]["max_candidates"] = json!(1);
    let error = execute_plan(source, plan).unwrap_err();
    assert_eq!(error.code, ErrorCode::ResourceLimit);
    assert_eq!(error.observed_at_least, Some(2));
    assert_eq!((error.row_index, error.field_index), (Some(1), Some(1)));
}

#[test]
fn maximum_group_preserves_all_elements_and_rejects_a_missing_last_sibling() {
    let source = format!(
        "<main><article>A</article>{}</main>",
        "<aside>B</aside>".repeat(63)
    );
    let plan = json!({"schema":"htmlcut.extraction.plan","version":SCHEMA_VERSION,
    "strategy":{"kind":"css","selector":"article"},
    "projection":{"kind":"records","following_siblings":63,"fields":[
        {"name":"payloads","selector":"article, aside","selection":{"kind":"all"}}
    ]}});
    let result = execute_plan(&source, plan.clone()).unwrap();
    let expected = std::iter::once("A")
        .chain(std::iter::repeat_n("B", 63))
        .collect::<Vec<_>>();
    assert_eq!(
        serde_json::to_value(result.data).unwrap(),
        json!([{"payloads":expected}])
    );
    let missing = format!(
        "<main><article>A</article>{}</main>",
        "<aside>B</aside>".repeat(62)
    );
    assert_eq!(
        execute_plan(&missing, plan).unwrap_err().code,
        ErrorCode::MissingRowSibling
    );
}

#[test]
fn added_subtree_cannot_hide_a_nested_root_candidate() {
    let plan = json!({"schema":"htmlcut.extraction.plan","version":SCHEMA_VERSION,
        "strategy":{"kind":"css","selector":"article"},"selection":{"kind":"nth","index":1},
        "projection":{"kind":"records","following_siblings":1,"fields":[{"name":"text","selector":":scope"}]}});
    let error = execute_plan(
        "<main><article>A</article><aside><article>B</article></aside></main>",
        plan,
    )
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::OverlappingRowScope);
    assert_eq!(error.row_index, Some(1));
}

#[test]
fn sibling_payloads_retain_anchor_context_and_complete_relationships() {
    let source = "<table><tr class=entry><td>A</td></tr> \n<!--comment--><tr><td><span class=score>10</span></td></tr><tr class=entry><td>B</td></tr><tr><td><span class=score>20</span></td></tr></table>";
    let result = execute(source, 1, false, false).unwrap();
    assert_eq!(
        serde_json::to_value(result.data).unwrap(),
        json!([{"title":"A","score":"10"},{"title":"B","score":"20"}])
    );
    assert_eq!(
        (
            result.receipt.candidate_count,
            result.receipt.selected_count
        ),
        (2, 2)
    );
    assert!(
        result
            .receipt
            .fields
            .iter()
            .all(|f| f.candidate_count == 2 && f.projected_count == 2)
    );
    let absent = execute(source, 0, true, false).unwrap();
    assert_eq!(
        serde_json::to_value(absent.data).unwrap(),
        json!([{"title":"A","score":null},{"title":"B","score":null}])
    );
}

#[test]
fn explicit_scope_missing_sibling_is_not_optional_absence() {
    let error = execute(
        "<table><tr class=entry><td>A</td></tr></table>",
        1,
        true,
        false,
    )
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::MissingRowSibling);
    assert_eq!((error.row_index, error.field_index), (Some(1), None));
}

#[test]
fn added_root_candidates_are_rejected_even_when_not_selected() {
    for nth in [false, true] {
        let error = execute(
            "<table><tr class=entry><td>A</td></tr><tr class=entry><td>B</td></tr></table>",
            1,
            true,
            nth,
        )
        .unwrap_err();
        assert_eq!(error.code, ErrorCode::OverlappingRowScope);
        assert_eq!((error.row_index, error.field_index), (Some(1), None));
    }
}

#[test]
fn following_scope_configuration_is_bounded_before_execution() {
    let error = execute("", 64, true, false).unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidPlan);
}

#[test]
fn selected_guards_enumerate_the_whole_group_once() {
    let value = json!({"schema":"htmlcut.extraction.plan","version":SCHEMA_VERSION,
        "strategy":{"kind":"css","selector":"tr.entry"},
        "projection":{"kind":"records","following_siblings":1,"fields":[{"name":"title","selector":":scope td"}]},
        "guards":[{"scope":"selected","selector":".score","min":1,"max":1,"read":{"kind":"dom_text"},"predicate":{"kind":"exact","value":"10"}}]});
    let compiled = CompiledPlan::compile(
        &ExtractionPlan::from_json(&serde_json::to_vec(&value).unwrap()).unwrap(),
    )
    .unwrap();
    let doc = PreparedDocument::new(
        SourceSnapshot::new(
            "<table><tr class=entry><td>A</td></tr><tr><td class=score>10</td></tr></table>",
            SnapshotMetadata::default(),
        )
        .unwrap(),
        PreparationLimits::default(),
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(doc.execute(&compiled).unwrap().data).unwrap(),
        json!([{"title":"A"}])
    );
}
