use super::*;

fn guard() -> Guard {
    Guard {
        scope: GuardScope::Document,
        selector: "p".into(),
        min: 1,
        max: Some(1),
        read: GuardRead::DomText,
        predicate: Some(Predicate::Exact {
            value: "text".into(),
        }),
    }
}
fn reject(plan: ExtractionPlan) {
    assert!(plan.validate().is_err());
    assert!(CompiledPlan::compile(&plan).is_err());
}

#[test]
fn t05_incompatible_modes_and_obsolete_envelopes_never_compile() {
    let base = ExtractionPlan::css("p").unwrap();
    let mut p = base.clone();
    p.schema = "other".into();
    reject(p);
    let mut p = base.clone();
    p.version = 2;
    reject(p);
    let mut p = base.clone();
    p.projection = Projection::Source;
    reject(p);
    for (min, max) in [(2, Some(1)), (1, Some(20_000))] {
        let mut p = base.clone();
        p.selection = Selection::All { min, max };
        reject(p);
    }
    for index in [0, 100_001] {
        let mut p = base.clone();
        p.selection = Selection::Nth { index };
        reject(p);
    }
    for name in [
        "".to_string(),
        "bad name".into(),
        "bad\0name".into(),
        "x".repeat(257),
    ] {
        let mut p = base.clone();
        p.projection = Projection::Attribute { name };
        reject(p);
    }
    let mut p = base.clone();
    p.projection = Projection::Attribute {
        name: "href".into(),
    };
    p.exclude.push("span".into());
    reject(p);
    let mut p = base.clone();
    p.exclude.push(String::new());
    reject(p);
    let slice = ExtractionPlan::slice(
        Boundary::Literal { value: "[".into() },
        Boundary::Literal { value: "]".into() },
    )
    .unwrap();
    let mut p = slice.clone();
    p.projection = Projection::DomText;
    reject(p);
    let mut p = slice.clone();
    p.exclude.push("p".into());
    reject(p);
    let mut p = slice.clone();
    p.guards.push(guard());
    reject(p);
    let mut p = slice;
    p.transforms.push(Transform::NormalizeWhitespace);
    reject(p);
    for count in [31, 32, 33] {
        let mut p = base.clone();
        p.exclude = vec!["p".into(); count];
        assert_eq!(p.validate().is_ok(), count <= 32);
        let mut p = base.clone();
        p.guards = vec![guard(); count];
        assert_eq!(p.validate().is_ok(), count <= 32);
    }
    let mut p = base;
    p.transforms = vec![Transform::NormalizeWhitespace; 3];
    reject(p);
}

#[test]
fn t18_guards_validate_cardinality_attributes_and_regex_flags() {
    let mut p = ExtractionPlan::css("p").unwrap();
    p.guards.push(guard());
    p.guards[0].predicate = None;
    reject(p.clone());
    p.guards[0].read = GuardRead::Attribute {
        name: "data-x".into(),
    };
    assert!(p.validate().is_ok());
    p.guards[0].read = GuardRead::Attribute {
        name: String::new(),
    };
    reject(p.clone());
    p.guards[0] = guard();
    p.guards[0].min = 2;
    reject(p.clone());
    p.guards[0] = guard();
    p.guards[0].max = Some(100_001);
    reject(p.clone());
    p.guards[0] = guard();
    p.guards[0].selector.clear();
    reject(p.clone());
    for flags in ["ii", "z"] {
        p.guards[0] = guard();
        p.guards[0].predicate = Some(Predicate::Regex {
            pattern: "text".into(),
            flags: flags.into(),
        });
        reject(p.clone());
    }
    p.guards[0] = guard();
    p.guards[0].predicate = Some(Predicate::Regex {
        pattern: String::new(),
        flags: String::new(),
    });
    reject(p);
}

#[test]
fn t28_transform_applicability_is_explicit_and_duplicate_transforms_fail() {
    let base = ExtractionPlan::css("p").unwrap();
    let mut p = base.clone();
    p.transforms = vec![
        Transform::NormalizeWhitespace,
        Transform::NormalizeWhitespace,
    ];
    reject(p);
    let mut p = base.clone();
    p.projection = Projection::InnerHtml;
    p.transforms = vec![Transform::NormalizeWhitespace];
    reject(p);
    let mut p = base.clone();
    p.transforms = vec![Transform::ResolveUrls];
    reject(p);
    let mut p = base.clone();
    p.projection = Projection::Attribute {
        name: "srcset".into(),
    };
    p.transforms = vec![Transform::ResolveUrls];
    reject(p);
    for name in [
        "href",
        "src",
        "action",
        "poster",
        "cite",
        "formaction",
        "data",
    ] {
        let mut p = base.clone();
        p.projection = Projection::Attribute { name: name.into() };
        p.transforms = vec![Transform::ResolveUrls];
        assert!(p.validate().is_ok());
    }
    let mut p = base;
    p.projection = Projection::DocumentText;
    p.transforms = vec![Transform::ResolveUrls];
    assert!(p.validate().is_ok());
}

#[test]
fn t29_compilation_depth_quoting_flags_and_regex_size_are_bounded() {
    for selector in [r".\31 a", r#"[data-x='[(']"#] {
        assert!(CompiledPlan::compile(&ExtractionPlan::css(selector).unwrap()).is_ok());
    }
    for depth in [63, 64, 65] {
        let selector = format!("{}p{}", ":is(".repeat(depth), ")".repeat(depth));
        let result = CompiledPlan::compile(&ExtractionPlan::css(selector).unwrap());
        if depth == 65 {
            assert_eq!(result.err().unwrap().code, ErrorCode::ResourceLimit);
        } else {
            assert!(result.is_ok());
        }
    }
    assert_eq!(
        ExtractionPlan::css("x".repeat(8193)).unwrap_err().code,
        ErrorCode::ResourceLimit
    );
    for (pattern, flags, valid) in [
        ("x", "imsUx", true),
        ("[", "", false),
        ("\\w{1000000}", "", false),
    ] {
        let plan = ExtractionPlan::slice(
            Boundary::Regex {
                pattern: pattern.into(),
                flags: flags.into(),
            },
            Boundary::Literal { value: "]".into() },
        )
        .unwrap();
        let result = CompiledPlan::compile(&plan);
        assert_eq!(result.is_ok(), valid);
        if pattern == "\\w{1000000}" {
            assert_eq!(result.err().unwrap().code, ErrorCode::ResourceLimit);
        }
    }
}

#[test]
fn t05_byte_budget_applies_to_materialized_defaults_and_escaped_predicates() {
    let mut plan = ExtractionPlan::css("p").unwrap();
    let empty_guard = Guard {
        scope: GuardScope::Document,
        selector: "p".into(),
        min: 1,
        max: Some(1),
        read: GuardRead::DomText,
        predicate: Some(Predicate::Exact {
            value: String::new(),
        }),
    };
    plan.guards.push(empty_guard);
    let base = crate::canonical_json(&plan).unwrap().len();
    if let Some(Predicate::Exact { value }) = &mut plan.guards[0].predicate {
        *value = "x".repeat(crate::limits::MAX_PLAN_BYTES - base);
    }
    assert_eq!(
        crate::canonical_json(&plan).unwrap().len(),
        crate::limits::MAX_PLAN_BYTES
    );
    assert!(plan.validate().is_ok());
    let bytes = crate::canonical_json(&plan).unwrap();
    assert!(ExtractionPlan::from_json(bytes.as_bytes()).is_ok());
    if let Some(Predicate::Exact { value }) = &mut plan.guards[0].predicate {
        value.push('x');
    }
    assert_eq!(plan.validate().unwrap_err().code, ErrorCode::ResourceLimit);
    let minimal = serde_json::json!({"schema":"htmlcut.extraction.plan","version":1,"strategy":{"kind":"css","selector":"p"},
        "guards":[{"scope":"document","selector":"p","read":{"kind":"dom_text"},"predicate":{"kind":"exact","value":"x".repeat(crate::limits::MAX_PLAN_BYTES-250)}}]});
    let encoded = serde_json::to_vec(&minimal).unwrap();
    assert!(encoded.len() <= crate::limits::MAX_PLAN_BYTES);
    assert_eq!(
        ExtractionPlan::from_json(&encoded).unwrap_err().code,
        ErrorCode::ResourceLimit
    );
}

#[test]
fn t29_selector_comments_and_quoted_delimiters_do_not_spend_nesting_budget() {
    let comment = format!("p/* {} */", "([".repeat(80));
    assert!(CompiledPlan::compile(&ExtractionPlan::css(comment).unwrap()).is_ok());
    for selector in [
        r#"[data-x="[(]"]"#,
        r#"[data-x='quote\"x']"#,
        r#"p:not([data-x="("])"#,
    ] {
        assert!(
            CompiledPlan::compile(&ExtractionPlan::css(selector).unwrap()).is_ok(),
            "{selector}"
        );
    }
    assert!(CompiledPlan::compile(&ExtractionPlan::css("p/* unterminated").unwrap()).is_ok());
}

#[test]
fn t29_selector_comment_terminators_and_invalid_slashes_reach_the_authoritative_parser() {
    let plan = ExtractionPlan::css("p /* *content */").unwrap();
    assert_eq!(
        prepared("<p>value</p>")
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap()
            .values,
        ["value"]
    );
    let plan = ExtractionPlan::css("p / div").unwrap();
    assert_eq!(
        CompiledPlan::compile(&plan).err().unwrap().code,
        ErrorCode::InvalidSelector
    );
    assert_eq!(
        SourceSnapshot::new(
            "",
            SnapshotMetadata {
                base_url: Some("https://:synthetic@example.test".into())
            }
        )
        .unwrap_err()
        .code,
        ErrorCode::InvalidBaseUrl
    );
}

#[test]
fn t05_schema_and_version_are_independently_required_and_all_defaults_to_nonempty() {
    for wire in [
        r#"{"schema":"wrong","version":1,"strategy":{"kind":"css","selector":"p"}}"#,
        r#"{"schema":"htmlcut.extraction.plan","version":2,"strategy":{"kind":"css","selector":"p"}}"#,
    ] {
        assert_eq!(
            ExtractionPlan::from_json(wire.as_bytes()).unwrap_err().code,
            ErrorCode::InvalidSchema
        );
    }
    let plan = ExtractionPlan::from_json(br#"{"schema":"htmlcut.extraction.plan","version":1,"strategy":{"kind":"css","selector":"aside"},"selection":{"kind":"all"}}"#).unwrap();
    assert_eq!(plan.selection, Selection::All { min: 1, max: None });
    assert_eq!(
        prepared("<p>value</p>")
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap_err()
            .code,
        ErrorCode::Cardinality
    );
}

#[test]
fn t29_unicode_regex_programs_share_one_aggregate_compilation_allowance() {
    // Under the locked regex engine, this valid Unicode program fits the full
    // allowance, but not a half share. Syntax and source size are unchanged.
    let boundary = Boundary::Regex {
        pattern: r"\w{100}".into(),
        flags: String::new(),
    };
    let mut slice = ExtractionPlan::slice(
        boundary.clone(),
        Boundary::Literal {
            value: "end".into(),
        },
    )
    .unwrap();
    CompiledPlan::compile(&slice).unwrap();
    if let Strategy::Slice { end, .. } = &mut slice.strategy {
        *end = boundary;
    }
    assert_eq!(
        CompiledPlan::compile(&slice).err().unwrap().code,
        ErrorCode::ResourceLimit
    );
    let mut css = ExtractionPlan::css("p").unwrap();
    let guard = Guard {
        scope: GuardScope::Document,
        selector: "p".into(),
        min: 1,
        max: Some(1),
        read: GuardRead::DomText,
        predicate: Some(Predicate::Regex {
            pattern: r"\w{100}".into(),
            flags: String::new(),
        }),
    };
    css.guards.push(guard.clone());
    CompiledPlan::compile(&css).unwrap();
    css.guards.push(guard);
    assert_eq!(
        CompiledPlan::compile(&css).err().unwrap().code,
        ErrorCode::ResourceLimit
    );
}

#[test]
fn t29_quote_and_comment_content_cannot_inflate_selector_grammar_depth() {
    let parentheses = "(".repeat(130);
    for selector in [
        format!("p[data-value='{parentheses}']"),
        format!("p[data-value=\"{parentheses}\"]"),
        format!("p[data-value='\\'{parentheses}']"),
        format!("p/* *not-close / {parentheses} */"),
    ] {
        CompiledPlan::compile(&ExtractionPlan::css(&selector).unwrap())
            .unwrap_or_else(|error| panic!("{selector}: {error:?}"));
    }
}
