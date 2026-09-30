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
    assert_eq!(document.execute(&selector).unwrap().values, ["HelloT"]);
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
    assert_eq!(document.execute(&slice).unwrap().values, ["\r\n✓\r\n"]);
    assert_eq!(document.parse_count(), 1);
}

#[test]
fn literal_default_is_lazy_reused_and_includes_hidden_content() {
    let source = prepared("<p hidden>A<span>B</span>C</p>");
    assert_eq!(source.parse_count(), 0);
    let compiled = CompiledPlan::compile(&ExtractionPlan::css("p").unwrap()).unwrap();
    for _ in 0..2 {
        let result = source.execute(&compiled).unwrap();
        assert_eq!(result.values, ["ABC"]);
        assert_eq!((result.candidate_count, result.selected_count), (1, 1));
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
    plan.projection = Projection::Source;
    let compiled = CompiledPlan::compile(&plan).unwrap();
    for _ in 0..2 {
        let result = source.execute(&compiled).unwrap();
        assert_eq!(result.values, ["\r\n<X a='1'>✓</X>\r\n"]);
        assert_eq!(result.ranges, Some(vec![SourceRange { start: 7, end: 27 }]));
    }
    assert_eq!(source.parse_count(), 0);
}

#[test]
fn preparation_failure_is_cached_and_not_partial_success() {
    let mut limits = PreparationLimits::default();
    limits.max_elements = 2;
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
    plan.projection = Projection::Attribute {
        name: "data-x".into(),
    };
    assert_eq!(
        source
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap()
            .values,
        [""]
    );
    plan.projection = Projection::Attribute {
        name: "absent".into(),
    };
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
    let minimal = r#"{"schema":"htmlcut.extraction.plan","version":1,"strategy":{"kind":"css","selector":"p"}}"#;
    assert!(ExtractionPlan::from_json(minimal.as_bytes()).is_ok());
    for value in [
        minimal.replace(
            "\"selector\":\"p\"",
            "\"selector\":\"p\",\"selector\":\"aside\"",
        ),
        minimal.replace("\"version\":1", "\"version\":1,\"unknown\":true"),
        minimal.replace("htmlcut.extraction.plan", "htmlcut.plan"),
        minimal.replace("\"version\":1", "\"version\":2"),
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
