// SPDX-License-Identifier: MPL-2.0
use super::*;

fn prepared(html: &str) -> PreparedDocument {
    PreparedDocument::new(
        SourceSnapshot::new(html, SnapshotMetadata::default()).unwrap(),
        PreparationLimits::default(),
    )
    .unwrap()
}

#[test]
fn structural_default_is_lazy_reused_and_includes_hidden_content() {
    let source = prepared("<p hidden>A<span>B</span>C</p>");
    assert_eq!(source.parse_count(), 0);
    let compiled = CompiledPlan::compile(&ExtractionPlan::css("p").unwrap()).unwrap();
    for _ in 0..2 {
        let result = source.execute(&compiled).unwrap();
        assert_eq!(result.data().as_values().unwrap(), ["ABC"]);
        assert_eq!((result.candidate_count(), result.selected_count()), (1, 1));
    }
    assert_eq!(source.parse_count(), 1);
}

#[test]
fn preparation_failure_is_cached_and_not_partial_success() {
    let limits = PreparationLimits {
        max_elements: 2,
        ..Default::default()
    };
    let source = PreparedDocument::new(
        SourceSnapshot::new("<p>value</p>", SnapshotMetadata::default()).unwrap(),
        limits,
    )
    .unwrap();
    let compiled = CompiledPlan::compile(&ExtractionPlan::css("p").unwrap()).unwrap();
    for _ in 0..2 {
        assert_eq!(
            source.execute(&compiled).unwrap_err().code,
            ErrorCode::ResourceLimit
        );
    }
    assert_eq!(source.parse_count(), 1);
}

#[test]
fn no_match_duplicates_and_empty_attributes_are_distinct() {
    let source = prepared("<p data-x=''></p><p>second</p>");
    let compile =
        |selector| CompiledPlan::compile(&ExtractionPlan::css(selector).unwrap()).unwrap();
    assert_eq!(
        source.execute(&compile("p")).unwrap_err().code,
        ErrorCode::AmbiguousSelection
    );
    assert_eq!(
        source.execute(&compile("aside")).unwrap_err().code,
        ErrorCode::NoMatch
    );
    let mut plan = ExtractionPlan::css("p[data-x]").unwrap();
    plan.read = Some(Reading::Attribute("data-x".into()));
    assert_eq!(
        source
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap()
            .data()
            .as_values()
            .unwrap(),
        [""]
    );
    plan.read = Some(Reading::Attribute("absent".into()));
    assert_eq!(
        source
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap_err()
            .code,
        ErrorCode::MissingAttribute
    );
}

#[test]
fn closed_json_rejects_nested_duplicates_unknown_members_and_retired_versions() {
    let minimal = br#"{"version":8,"select":"p"}"#;
    ExtractionPlan::from_json(minimal).unwrap();
    for value in [
        br#"{"version":8,"select":"p","select":"aside"}"#.as_slice(),
        br#"{"version":8,"select":"p","unknown":true}"#.as_slice(),
        br#"{"version":8,"select":"p","schema":"htmlcut.extraction.plan"}"#.as_slice(),
        br#"{"version":2,"select":"p"}"#.as_slice(),
    ] {
        assert!(ExtractionPlan::from_json(value).is_err());
    }
}

#[path = "contract_discovery.rs"]
mod discovery;
#[path = "contract_fidelity.rs"]
mod fidelity;
#[path = "contract_guards.rs"]
mod guards;
#[path = "schema_contract.rs"]
mod schema_contract;

#[path = "contract_budgets.rs"]
mod budgets;
#[path = "contract_closure.rs"]
mod closure;
#[path = "contract_corpus.rs"]
mod corpus;
#[path = "selector_accounting.rs"]
mod selector_accounting;
#[path = "contract_validation.rs"]
mod validation;

#[test]
fn attribute_only_queries_never_visit_unrequested_projection_paths() {
    let document =
        prepared("<article data-key='chosen'><pre>body</pre><script>payload</script></article>");
    let mut plan = ExtractionPlan::css("article").unwrap();
    plan.read = Some(Reading::Attribute("data-key".into()));
    crate::projection::take_projection_calls();
    assert_eq!(
        document
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap()
            .data()
            .as_values()
            .unwrap(),
        ["chosen"]
    );
    assert_eq!(crate::projection::take_projection_calls(), [1, 0, 0, 0]);
    for reading in [Reading::Text, Reading::OuterHtml] {
        plan.read = Some(reading);
        document
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap();
    }
    assert_eq!(crate::projection::take_projection_calls(), [0, 1, 0, 1]);
}
