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
fn selector_and_reading_contract_remain_miri_sound() {
    let invalid = ExtractionPlan::css("[").unwrap();
    assert_eq!(
        CompiledPlan::compile(&invalid).err().unwrap().code,
        ErrorCode::InvalidSelector
    );
    let document = prepared("<article><p>Hello</p><template>T</template></article>");
    let selector = CompiledPlan::compile(&ExtractionPlan::css("article").unwrap()).unwrap();
    assert_eq!(
        document
            .execute(&selector)
            .unwrap()
            .data()
            .as_values()
            .unwrap(),
        ["Hello"]
    );
    assert_eq!(document.parse_count(), 1);
    let document = prepared(
        "<main><ol start='7'><li>A</li><li id='selected'>B</li></ol><table><tr><td>X | Y</td><td>Z</td></tr></table><pre><code id='code'>x\n</code></pre></main>",
    );
    for (css, expected) in [
        ("#selected", "- 8\\. B"),
        ("tr", "-\n  - X | Y\n  - Z"),
        ("#code", "```\nx\n\n```"),
    ] {
        let mut plan = ExtractionPlan::css(css).unwrap();
        plan.read = Some(Reading::Markdown);
        let compiled = CompiledPlan::compile(&plan).unwrap();
        for _ in 0..2 {
            assert_eq!(
                document
                    .execute(&compiled)
                    .unwrap()
                    .data()
                    .as_values()
                    .unwrap(),
                [expected]
            );
        }
    }
    let mut plan = ExtractionPlan::css("main:has(> ol) li:nth-child(2)").unwrap();
    plan.read = Some(Reading::Attribute("id".into()));
    let compiled = CompiledPlan::compile(&plan).unwrap();
    for _ in 0..2 {
        assert_eq!(
            document
                .execute(&compiled)
                .unwrap()
                .data()
                .as_values()
                .unwrap(),
            ["selected"]
        );
    }
    let document = prepared("<article><p>value</p></article>");
    let plan = ExtractionPlan::from_json(br#"{"version":7,"select":"article","fields":{"text":{"select":"p"},"optional":{"select":".absent","match":"optional"}}}"#).unwrap();
    let compiled = CompiledPlan::compile(&plan).unwrap();
    let first = document.execute(&compiled).unwrap();
    let second = document.execute(&compiled).unwrap();
    assert_eq!(
        serde_json::to_vec(first.data()).unwrap(),
        br#"[{"optional":null,"text":"value"}]"#
    );
    assert_eq!(first.data(), second.data());
    let document = prepared(
        "<article><p>A</p></article><!-- gap --><aside><p>\u{a0}<code>x`y</code><em>!</em>\u{2003}</p></aside>",
    );
    let plan = ExtractionPlan::from_json(br#"{"version":7,"select":"article","following_siblings":1,"fields":{"reading":{"select":":scope + aside p","read":"markdown"},"normalized":{"select":":scope + aside p"}}}"#).unwrap();
    let compiled = CompiledPlan::compile(&plan).unwrap();
    let first = document.execute(&compiled).unwrap();
    assert_eq!(
        serde_json::to_value(first.data()).unwrap(),
        serde_json::json!([{"reading":"\u{a0}``x`y``<em>\\!</em>\u{2003}","normalized":"x`y!"}])
    );
    assert_eq!(first.data(), document.execute(&compiled).unwrap().data());
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
    let minimal = br#"{"version":7,"select":"p"}"#;
    ExtractionPlan::from_json(minimal).unwrap();
    for value in [
        br#"{"version":7,"select":"p","select":"aside"}"#.as_slice(),
        br#"{"version":7,"select":"p","unknown":true}"#.as_slice(),
        br#"{"version":7,"select":"p","schema":"htmlcut.extraction.plan"}"#.as_slice(),
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
#[path = "fragment_context.rs"]
mod fragment_context;
#[path = "selector_accounting.rs"]
mod selector_accounting;
#[path = "contract_validation.rs"]
mod validation;

#[test]
fn t06_t07_attribute_only_execution_never_calls_unrequested_projection_or_preview_paths() {
    let document = prepared(
        "<article data-key='chosen'><pre>large <b>body</b></pre><script>payload</script></article>",
    );
    let mut plan = ExtractionPlan::css("article").unwrap();
    plan.read = Some(Reading::Attribute("data-key".into()));
    let compiled = CompiledPlan::compile(&plan).unwrap();
    crate::projection::take_projection_calls();
    for _ in 0..3 {
        assert_eq!(
            document
                .execute(&compiled)
                .unwrap()
                .data()
                .as_values()
                .unwrap(),
            ["chosen"]
        );
    }
    assert_eq!(document.parse_count(), 1);
    assert_eq!(crate::projection::take_projection_calls(), [3, 0, 0, 0]);
    // Positive controls prove that all counters observe the actual paths.
    for reading in [Reading::Literal, Reading::Markdown, Reading::OuterHtml] {
        plan.read = Some(reading);
        document
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap();
    }
    assert_eq!(crate::projection::take_projection_calls(), [0, 1, 1, 1]);
}
