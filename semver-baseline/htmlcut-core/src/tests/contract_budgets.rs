// SPDX-License-Identifier: MPL-2.0
use super::*;
use serde_json::{Value, json};

#[test]
fn t29_each_policy_rejects_zero_and_overflow_without_hiding_runtime() {
    let cases = [
        (true, "max_source_bytes", 52_428_800),
        (true, "max_elements", 1_000_000),
        (true, "max_nodes", 4_000_000),
        (true, "max_depth", 4_096),
        (true, "max_parse_work", 100_000_000),
        (false, "max_work", 10_000_000),
        (false, "max_candidates", 1_000_000),
        (false, "max_selected", 100_000),
        (false, "max_value_bytes", 8_388_608),
        (false, "max_total_value_bytes", 67_108_864),
    ];
    for (preparation, field, maximum) in cases {
        for (value, accepted) in [
            (0, false),
            (maximum - 1, true),
            (maximum, true),
            (maximum + 1, false),
        ] {
            let mut data = if preparation {
                serde_json::to_value(PreparationLimits::default()).unwrap()
            } else {
                serde_json::to_value(ExecutionLimits::default()).unwrap()
            };
            data[field] = json!(value);
            let valid = if preparation {
                serde_json::from_value::<PreparationLimits>(data).is_ok()
            } else {
                serde_json::from_value::<ExecutionLimits>(data).is_ok()
            };
            assert_eq!(valid, accepted, "{field}={value}");
        }
    }
    assert!(serde_json::from_value::<ExecutionLimits>(json!({"max_work":1.5})).is_err());
    assert!(serde_json::from_value::<PreparationLimits>(json!({"max_nodes":u64::MAX})).is_err());
    assert!(serde_json::from_value::<ExecutionLimits>(json!({"unknown":1})).is_err());
}

#[test]
fn t29_source_value_aggregate_candidate_and_selection_budget_triplets() {
    for (bytes, accepted) in [(3, true), (4, true), (5, false)] {
        let snapshot = SourceSnapshot::new("x".repeat(bytes), SnapshotMetadata::default()).unwrap();
        let limits = PreparationLimits {
            max_source_bytes: 4,
            ..Default::default()
        };
        assert_eq!(PreparedDocument::new(snapshot, limits).is_ok(), accepted);
    }
    let source = prepared("<p>é</p><p>é</p>");
    for (maximum, accepted) in [(1, false), (2, true), (3, true)] {
        let mut plan = ExtractionPlan::css("p").unwrap();
        plan.selection = Selection::Nth { index: 1 };
        plan.limits.max_value_bytes = maximum;
        assert_eq!(
            source
                .execute(&CompiledPlan::compile(&plan).unwrap())
                .is_ok(),
            accepted
        );
        plan.selection = Selection::All { min: 1, max: None };
        plan.limits.max_value_bytes = 8;
        plan.limits.max_candidates = maximum;
        assert_eq!(
            source
                .execute(&CompiledPlan::compile(&plan).unwrap())
                .is_ok(),
            accepted
        );
        plan.limits.max_candidates = 10;
        plan.limits.max_selected = maximum;
        assert_eq!(
            source
                .execute(&CompiledPlan::compile(&plan).unwrap())
                .is_ok(),
            accepted
        );
    }
    for (maximum, accepted) in [(3, false), (4, true), (5, true)] {
        let mut plan = ExtractionPlan::css("p").unwrap();
        plan.selection = Selection::All { min: 1, max: None };
        plan.limits.max_total_value_bytes = maximum;
        assert_eq!(
            source
                .execute(&CompiledPlan::compile(&plan).unwrap())
                .is_ok(),
            accepted
        );
    }
    let mut plan = ExtractionPlan::slice(
        Boundary::Literal { value: "[".into() },
        Boundary::Literal { value: "]".into() },
    )
    .unwrap();
    plan.limits.max_value_bytes = 1;
    assert_eq!(
        prepared("[é]")
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap_err()
            .code,
        ErrorCode::ResourceLimit
    );
    plan.selection = Selection::All { min: 1, max: None };
    plan.limits.max_value_bytes = 8;
    plan.limits.max_candidates = 1;
    let error = prepared("[a][b]")
        .execute(&CompiledPlan::compile(&plan).unwrap())
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::ResourceLimit);
}

#[test]
fn t29_shared_work_does_not_reset_for_guards_or_exclusions() {
    let source = prepared("<p id='x'>value<span>guard</span></p>");
    let plain = ExtractionPlan::css("#x").unwrap();
    let minimum = (1..1000)
        .find(|limit| {
            let mut plan = plain.clone();
            plan.limits.max_work = *limit;
            source
                .execute(&CompiledPlan::compile(&plan).unwrap())
                .is_ok()
        })
        .unwrap();
    for (limit, accepted) in [(minimum - 1, false), (minimum, true), (minimum + 1, true)] {
        let mut plan = plain.clone();
        plan.limits.max_work = limit;
        assert_eq!(
            source
                .execute(&CompiledPlan::compile(&plan).unwrap())
                .is_ok(),
            accepted
        );
    }
    let mut guarded = plain.clone();
    guarded.limits.max_work = minimum;
    guarded.guards.push(Guard {
        scope: GuardScope::Selected,
        selector: "span".into(),
        min: 1,
        max: Some(1),
        read: GuardRead::DomText {},
        predicate: Some(Predicate::Exact {
            value: "guard".into(),
        }),
    });
    assert_eq!(
        source
            .execute(&CompiledPlan::compile(&guarded).unwrap())
            .unwrap_err()
            .code,
        ErrorCode::ResourceLimit
    );
    guarded.guards.clear();
    guarded.exclude.push("span".into());
    assert_eq!(
        source
            .execute(&CompiledPlan::compile(&guarded).unwrap())
            .unwrap_err()
            .code,
        ErrorCode::ResourceLimit
    );
    assert!(
        source
            .execute(&CompiledPlan::compile(&plain).unwrap())
            .is_ok()
    );
}

#[test]
fn t05_t29_closed_json_depth_size_primitives_and_duplicates() {
    for data in [
        b"null".as_slice(),
        b"true",
        b"-2",
        b"1.5",
        b"[1,2]",
        b"{\"x\":{\"a\":1,\"a\":2}}",
        b"{\"x\":1}",
    ] {
        let closed = parse_closed_json(data, crate::MAX_PLAN_BYTES);
        if data.contains(&b'x') && data.iter().filter(|c| **c == b'a').count() == 2 {
            assert!(closed.is_err());
        } else {
            assert!(closed.is_ok(), "{}", String::from_utf8_lossy(data));
        }
    }
    let deep = format!("{}0{}", "[".repeat(66), "]".repeat(66));
    assert_eq!(
        parse_closed_json(deep.as_bytes(), crate::MAX_PLAN_BYTES)
            .unwrap_err()
            .code,
        ErrorCode::ResourceLimit
    );
    let oversized = vec![b' '; crate::limits::MAX_PLAN_BYTES + 1];
    assert_eq!(
        parse_closed_json(&oversized, crate::MAX_PLAN_BYTES)
            .unwrap_err()
            .code,
        ErrorCode::ResourceLimit
    );
    assert_eq!(
        ExtractionPlan::from_json(&oversized).unwrap_err().code,
        ErrorCode::ResourceLimit
    );
    let mut plan = ExtractionPlan::css("p").unwrap();
    plan.guards.push(Guard {
        scope: GuardScope::Document,
        selector: "p".into(),
        min: 1,
        max: Some(1),
        read: GuardRead::DomText {},
        predicate: Some(Predicate::Exact {
            value: "x".repeat(crate::limits::MAX_PLAN_BYTES),
        }),
    });
    assert_eq!(plan.validate().unwrap_err().code, ErrorCode::ResourceLimit);
    let mut json = serde_json::to_value(ExtractionPlan::css("p").unwrap()).unwrap();
    json["guards"] = json!([{"scope":"document","selector":"p","read":{"kind":"dom_text"},"predicate":{"kind":"exact","value":"\u{0001}".repeat(50_000)}}]);
    assert!(ExtractionPlan::from_json(&serde_json::to_vec(&json).unwrap()).is_err());
    assert_eq!(serde_json::from_slice::<Value>(b"0").unwrap(), json!(0));
}

#[test]
fn t29_hard_source_cap_is_checked_before_owned_snapshot_copy() {
    let mut text = "x".repeat(52_428_800);
    assert!(SourceSnapshot::new(&text, SnapshotMetadata::default()).is_ok());
    text.pop();
    assert!(SourceSnapshot::new(&text, SnapshotMetadata::default()).is_ok());
    text.push('x');
    text.push('x');
    assert_eq!(
        SourceSnapshot::new(&text, SnapshotMetadata::default())
            .unwrap_err()
            .code,
        ErrorCode::ResourceLimit
    );
    for base in [
        "not a URL",
        "https://user:secret@example.test/",
        "file:///tmp/a",
        "https://example.test",
    ] {
        let result = SourceSnapshot::new(
            "",
            SnapshotMetadata {
                base_url: Some(base.into()),
            },
        );
        assert_eq!(result.is_ok(), base == "https://example.test");
    }
}

#[test]
fn t29_url_metadata_and_resolution_processing_are_bounded() {
    let prefix = "https://example.test/";
    for (length, valid) in [(8191, true), (8192, true), (8193, false)] {
        let base = format!("{prefix}{}", "x".repeat(length - prefix.len()));
        assert_eq!(
            SourceSnapshot::new(
                "",
                SnapshotMetadata {
                    base_url: Some(base)
                }
            )
            .is_ok(),
            valid
        );
    }
    let expanded = format!("https://example.test/{}", "é".repeat(2000));
    assert_eq!(
        SourceSnapshot::new(
            "",
            SnapshotMetadata {
                base_url: Some(expanded)
            }
        )
        .unwrap_err()
        .code,
        ErrorCode::ResourceLimit
    );
    let html = format!("<a href='{}'>value</a>", "x".repeat(8193));
    let document = PreparedDocument::new(
        SourceSnapshot::new(
            &html,
            SnapshotMetadata {
                base_url: Some("https://example.test/".into()),
            },
        )
        .unwrap(),
        Default::default(),
    )
    .unwrap();
    let mut plan = ExtractionPlan::css("a").unwrap();
    plan.projection = Projection::Value(ValueProjection::Attribute {
        name: "href".into(),
    });
    plan.transforms = vec![Transform::ResolveUrls {}];
    assert_eq!(
        document
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap_err()
            .code,
        ErrorCode::ResourceLimit
    );
}

#[test]
fn t29_structural_rendering_never_returns_a_prefix_at_any_output_limit() {
    let source = prepared(
        "<article><!--comment--><h3>Header</h3><p>text<img alt='[alternative]'><a href='next'>link</a></p><ol reversed> stray<li>A</li><li>B</li></ol><pre>code<script>ignored</script><span class='omit'>discard</span><a href='```'>label</a><img alt='````'></pre><table><tr><th rowspan='2'>H</th><td colspan='3'>V</td></tr></table></article>",
    );
    let mut plan = ExtractionPlan::css("article").unwrap();
    plan.projection = Projection::Value(ValueProjection::Markdown {});
    plan.exclude = vec![".omit".into()];
    let expected = "### Header\n\ntext\\[alternative\\][link](<next>)\n\nstray\n- 2\\. A\n- 1\\. B\n\n```\ncodelabel\n```\n\n- [label](<```>)\n- \\`\\`\\`\\`\n\n-\n  - <strong>H</strong>\n  - V";
    let complete = source
        .execute(&CompiledPlan::compile(&plan).unwrap())
        .unwrap();
    assert_eq!(complete.data.as_values().unwrap(), [expected]);
    for limit in 1..=expected.len() + 1 {
        plan.limits.max_value_bytes = limit as u32;
        let result = source.execute(&CompiledPlan::compile(&plan).unwrap());
        if limit < expected.len() {
            assert_eq!(
                result.unwrap_err().code,
                ErrorCode::ResourceLimit,
                "limit={limit}"
            );
        } else {
            assert_eq!(result.unwrap().data.as_values().unwrap(), [expected]);
        }
    }
    plan.limits.max_value_bytes = 1024;
    let mut first_success = None;
    for work in 1..500 {
        plan.limits.max_work = work;
        match source.execute(&CompiledPlan::compile(&plan).unwrap()) {
            Ok(result) => {
                assert_eq!(result.data.as_values().unwrap(), [expected]);
                first_success = Some(work);
                break;
            }
            Err(error) => assert_eq!(error.code, ErrorCode::ResourceLimit, "work={work}"),
        }
    }
    let work = first_success.expect("finite shared rendering work");
    plan.limits.max_work = work + 1;
    assert_eq!(
        source
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap()
            .data
            .as_values()
            .unwrap(),
        [expected]
    );
}

#[test]
fn t05_closed_json_preserves_all_primitive_values_and_large_signedness() {
    let wire = br#"[null,true,false,-7,4294967296,1.5,"x",{"k":"v"}]"#;
    assert_eq!(
        crate::parse_closed_json(wire, crate::MAX_PLAN_BYTES).unwrap(),
        json!([null,true,false,-7,4294967296_u64,1.5,"x",{"k":"v"}])
    );
}

#[test]
fn t29_attribute_name_and_projection_value_exact_boundaries_are_accepted() {
    let name = "a".repeat(256);
    let html = format!("<p {name}='é'>value</p>");
    let source = prepared(&html);
    let mut plan = ExtractionPlan::css("p").unwrap();
    plan.projection = Projection::Value(ValueProjection::Attribute { name });
    for (maximum, accepted) in [(1, false), (2, true), (3, true)] {
        plan.limits.max_value_bytes = maximum;
        let result = source.execute(&CompiledPlan::compile(&plan).unwrap());
        if accepted {
            assert_eq!(result.unwrap().data.as_values().unwrap(), ["é"]);
        } else {
            assert_eq!(result.unwrap_err().code, ErrorCode::ResourceLimit);
        }
    }
    plan.projection = Projection::Value(ValueProjection::Attribute {
        name: "a".repeat(257),
    });
    assert_eq!(
        CompiledPlan::compile(&plan).err().unwrap().code,
        ErrorCode::InvalidPlan
    );
    let discovery = prepared(&format!("<p {}='value'>text</p>", "a".repeat(128)));
    let page = discovery.inspect("p", 3).unwrap();
    assert_eq!(
        page.samples
            .iter()
            .find(|e| e.tag == "p")
            .unwrap()
            .attributes[0]
            .len(),
        128
    );
}

#[test]
fn t29_slice_aggregate_budget_is_debited_for_every_selected_value() {
    let source = prepared("[AB][CD][EF]");
    let mut plan = ExtractionPlan::slice(
        Boundary::Literal { value: "[".into() },
        Boundary::Literal { value: "]".into() },
    )
    .unwrap();
    plan.selection = Selection::All { min: 1, max: None };
    for (maximum, success) in [(5, false), (6, true), (7, true)] {
        plan.limits.max_total_value_bytes = maximum;
        let result = source.execute(&CompiledPlan::compile(&plan).unwrap());
        if success {
            assert_eq!(
                result.unwrap().data.as_values().unwrap(),
                ["AB", "CD", "EF"]
            );
        } else {
            assert_eq!(result.unwrap_err().code, ErrorCode::ResourceLimit);
        }
    }
}

#[test]
fn t29_url_raw_and_resolved_lengths_accept_the_exact_bound() {
    let prefix = "https://example.test/?q=";
    let value = format!("{prefix}{}", "a".repeat(8192 - prefix.len()));
    assert_eq!(
        crate::projection::resolve_url(&value, None, 8192).unwrap(),
        value
    );
    assert_eq!(
        crate::projection::resolve_url(&value, None, 8191)
            .unwrap_err()
            .code,
        ErrorCode::ResourceLimit
    );
    assert_eq!(
        crate::projection::resolve_url(&(value + "a"), None, 10000)
            .unwrap_err()
            .code,
        ErrorCode::ResourceLimit
    );
}
