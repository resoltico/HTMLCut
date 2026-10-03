use super::*;

fn prepared(html: &str) -> PreparedDocument {
    PreparedDocument::new(
        SourceSnapshot::new(html, SnapshotMetadata::default()).unwrap(),
        PreparationLimits::default(),
    )
    .unwrap()
}

#[test]
fn selector_and_slice_contract_remain_miri_sound() {
    let invalid = ExtractionPlan::css("[").unwrap();
    assert_eq!(
        CompiledPlan::compile(&invalid).err().unwrap().code,
        ErrorCode::InvalidSelector
    );
    let document =
        prepared("<article><p>Hello</p><template>T</template></article>BEGIN\r\n✓\r\nEND");
    let selector = CompiledPlan::compile(&ExtractionPlan::css("article").unwrap()).unwrap();
    assert_eq!(
        document
            .execute(&selector)
            .unwrap()
            .data
            .as_values()
            .unwrap(),
        ["HelloT"]
    );
    let slice = CompiledPlan::compile(
        &ExtractionPlan::slice(
            Boundary::Literal {
                value: "BEGIN".into(),
            },
            Boundary::Literal {
                value: "END".into(),
            },
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        document.execute(&slice).unwrap().data.as_values().unwrap(),
        ["\r\n✓\r\n"]
    );
    assert_eq!(document.parse_count(), 1);

    // The same strict-provenance proof owns scoped cache reuse and fragment context.
    let document = prepared(
        "<main><ol start='7'><li>A</li><li id='selected'>B</li></ol><table><tr><td>X | Y</td><td>Z</td></tr></table><pre><code id='code'>x\n</code></pre></main>",
    );
    for (css, expected) in [
        ("#selected", "- 8\\. B"),
        ("tr", "-\n  - X | Y\n  - Z"),
        ("#code", "```\nx\n\n```"),
    ] {
        let mut plan = ExtractionPlan::css(css).unwrap();
        plan.projection = Projection::Value(ValueProjection::Markdown {});
        let compiled = CompiledPlan::compile(&plan).unwrap();
        for _ in 0..2 {
            assert_eq!(
                document
                    .execute(&compiled)
                    .unwrap()
                    .data
                    .as_values()
                    .unwrap(),
                [expected]
            );
        }
    }
    let mut plan = ExtractionPlan::css("main:has(> ol) li:nth-child(2)").unwrap();
    plan.projection = Projection::Value(ValueProjection::Attribute { name: "id".into() });
    let compiled = CompiledPlan::compile(&plan).unwrap();
    for _ in 0..2 {
        assert_eq!(
            document
                .execute(&compiled)
                .unwrap()
                .data
                .as_values()
                .unwrap(),
            ["selected"]
        );
    }
    let document = prepared("<article><p>value</p></article>");
    let plan=ExtractionPlan::from_json(br#"{"schema":"htmlcut.extraction.plan","version":3,"strategy":{"kind":"css","selector":"article"},"projection":{"kind":"records","fields":[{"name":"text","selector":"p"},{"name":"optional","selector":".absent","selection":{"kind":"optional"}}]}}"#).unwrap();
    let compiled = CompiledPlan::compile(&plan).unwrap();
    let first = document.execute(&compiled).unwrap();
    assert_eq!(
        serde_json::to_value(&first.data).unwrap(),
        serde_json::json!([{"text":"value","optional":null}])
    );
    assert_eq!(first, document.execute(&compiled).unwrap());
}

#[test]
fn literal_default_is_lazy_reused_and_includes_hidden_content() {
    let source = prepared("<p hidden>A<span>B</span>C</p>");
    assert_eq!(source.parse_count(), 0);
    let compiled = CompiledPlan::compile(&ExtractionPlan::css("p").unwrap()).unwrap();
    for _ in 0..2 {
        let result = source.execute(&compiled).unwrap();
        assert_eq!(result.data.as_values().unwrap(), ["ABC"]);
        assert_eq!(
            (
                result.receipt.candidate_count,
                result.receipt.selected_count
            ),
            (1, 1)
        );
    }
    assert_eq!(source.parse_count(), 1);
}

#[test]
fn source_slices_preserve_unicode_crlf_and_never_parse() {
    let source = prepared("éSTART\r\n<X a='1'>✓</X>\r\nEND");
    let mut plan = ExtractionPlan::css("p").unwrap();
    plan.strategy = Strategy::Slice {
        start: Boundary::Literal {
            value: "START".into(),
        },
        end: Boundary::Literal {
            value: "END".into(),
        },
        include_start: false,
        include_end: false,
    };
    plan.projection = Projection::Source {};
    let compiled = CompiledPlan::compile(&plan).unwrap();
    for _ in 0..2 {
        let result = source.execute(&compiled).unwrap();
        assert_eq!(result.data.as_values().unwrap(), ["\r\n<X a='1'>✓</X>\r\n"]);
        assert_eq!(
            result.receipt.ranges,
            Some(vec![SourceRange { start: 7, end: 27 }])
        );
    }
    assert_eq!(source.parse_count(), 0);
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
    plan.projection = Projection::Value(ValueProjection::Attribute {
        name: "data-x".into(),
    });
    assert_eq!(
        source
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap()
            .data
            .as_values()
            .unwrap(),
        [""]
    );
    plan.projection = Projection::Value(ValueProjection::Attribute {
        name: "absent".into(),
    });
    assert_eq!(
        source
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap_err()
            .code,
        ErrorCode::MissingAttribute
    );
}

#[test]
fn closed_json_rejects_nested_duplicates_unknown_fields_and_old_schema() {
    let minimal = r#"{"schema":"htmlcut.extraction.plan","version":3,"strategy":{"kind":"css","selector":"p"}}"#;
    assert!(ExtractionPlan::from_json(minimal.as_bytes()).is_ok());
    for value in [
        minimal.replace(
            "\"selector\":\"p\"",
            "\"selector\":\"p\",\"selector\":\"aside\"",
        ),
        minimal.replace("\"version\":3", "\"version\":3,\"unknown\":true"),
        minimal.replace("htmlcut.extraction.plan", "htmlcut.plan"),
        minimal.replace("\"version\":3", "\"version\":2"),
    ] {
        assert!(
            ExtractionPlan::from_json(value.as_bytes()).is_err(),
            "{value}"
        );
    }
}

#[path = "contract_discovery.rs"]
mod discovery;
#[path = "contract_fidelity.rs"]
mod fidelity;
#[path = "contract_guards.rs"]
mod guards;
#[path = "contract_identity.rs"]
mod identity;

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
    plan.projection = Projection::Value(ValueProjection::Attribute {
        name: "data-key".into(),
    });
    let compiled = CompiledPlan::compile(&plan).unwrap();
    crate::projection::take_projection_calls();
    for _ in 0..3 {
        assert_eq!(
            document
                .execute(&compiled)
                .unwrap()
                .data
                .as_values()
                .unwrap(),
            ["chosen"]
        );
    }
    assert_eq!(document.parse_count(), 1);
    assert_eq!(crate::projection::take_projection_calls(), [3, 0, 0, 0]);
    // Positive controls prove that all counters observe the actual paths.
    for projection in [
        Projection::Value(ValueProjection::DomText {}),
        Projection::Value(ValueProjection::Markdown {}),
        Projection::Value(ValueProjection::OuterHtml {}),
    ] {
        plan.projection = projection;
        document
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap();
    }
    assert_eq!(crate::projection::take_projection_calls(), [0, 1, 1, 1]);
}
