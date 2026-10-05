// SPDX-License-Identifier: MPL-2.0
//! Public resource edges, representation limits and truthful ownership diagnostics.
use htmlcut_core::*;
use serde_json::json;
#[path = "support/query.rs"]
mod query_support;
use query_support::{compile, data, document};

#[test]
fn diagnostics_identify_owning_counter_field_and_completed_assumptions() {
    let parsed = ExtractionPlan::from_json(
        br#"{"version":6,"select":"article","fields":{"price":{"select":"["}}}"#,
    )
    .unwrap();
    let error = CompiledPlan::compile(&parsed).err().unwrap();
    assert_eq!(error.field_name.as_deref(), Some("price"));
    assert_eq!(error.plan_path.as_deref(), Some("fields.price.select"));
    for query in [
        json!({"version":6,"select":"p","match":"nth","index":3,"limits":{"max_candidates":2}}),
        json!({"version":6,"select":"p","expect":[{"select":"p","max":3}],"limits":{"max_candidates":2}}),
    ] {
        assert!(ExtractionPlan::from_json(&serde_json::to_vec(&query).unwrap()).is_err());
    }
    let error = document("<p>A</p><p>B</p>")
        .execute(&compile(
            json!({"version":6,"select":"p","match":"all","expect":[{"select":"p"}]}),
        ))
        .unwrap_err();
    assert_eq!(
        (
            error.code,
            error.candidate_count,
            error.expected_min,
            error.expected_max
        ),
        (ErrorCode::GuardFailed, Some(2), Some(1), Some(1))
    );
    let error = document("<p>AB</p>")
        .execute(&compile(
            json!({"version":6,"select":"p","limits":{"max_value_bytes":1}}),
        ))
        .unwrap_err();
    assert_eq!(
        (error.resource_counter.as_deref(), error.configured_bound),
        (Some("max_value_bytes"), Some(1))
    );
    let error = document("<p>AB</p><p>CD</p>")
        .execute(&compile(
            json!({"version":6,"select":"p","match":"all","limits":{"max_total_value_bytes":3}}),
        ))
        .unwrap_err();
    assert_eq!(
        (error.resource_counter.as_deref(), error.configured_bound),
        (Some("max_total_value_bytes"), Some(3))
    );
    let huge = format!("<p>{}</p>", "A".repeat(10000));
    let error = document(&huge).execute(&compile(json!({"version":6,"select":"p","limits":{"max_work":100,"max_total_value_bytes":15000}}))).unwrap_err();
    assert_eq!(
        (error.resource_counter.as_deref(), error.configured_bound),
        (Some("max_work"), Some(100))
    );
    let error = document("<p>AB</p>").execute(&compile(json!({"version":6,"select":"p","read":"literal","expect":[{"select":"p","equals":"AB"}],"limits":{"max_value_bytes":1}}))).unwrap_err();
    assert_eq!(
        (error.resource_counter.as_deref(), error.configured_bound),
        (Some("max_value_bytes"), Some(1))
    );
}

#[test]
fn preparation_failures_identify_exact_policy_counter_and_reuse_failure() {
    for (limits, counter, bound) in [
        (
            PreparationLimits {
                max_nodes: 1,
                ..Default::default()
            },
            "max_nodes",
            1,
        ),
        (
            PreparationLimits {
                max_depth: 1,
                ..Default::default()
            },
            "max_depth",
            1,
        ),
        (
            PreparationLimits {
                max_parse_work: 1,
                ..Default::default()
            },
            "max_parse_work",
            1,
        ),
    ] {
        let source = PreparedDocument::new(
            SourceSnapshot::new("<p>A</p>", Default::default()).unwrap(),
            limits,
        )
        .unwrap();
        let query = compile(json!({"version":6,"select":"p"}));
        let first = source.execute(&query).unwrap_err();
        let second = source.execute(&query).unwrap_err();
        assert_eq!(first, second);
        assert_eq!(
            (first.resource_counter.as_deref(), first.configured_bound),
            (Some(counter), Some(bound))
        );
    }
}

#[test]
fn signed_ordinals_reject_both_overflow_directions_but_explicit_reset_can_recover() {
    let query = compile(json!({"version":6,"select":"ol","read":"markdown"}));
    for source in [
        "<ol start='9223372036854775808'><li>A</li></ol>",
        "<ol start='-9223372036854775809'><li>A</li></ol>",
        "<ol start='9223372036854775807'><li>A</li><li>B</li></ol>",
        "<ol reversed start='-9223372036854775808'><li>A</li><li>B</li></ol>",
    ] {
        let error = document(source).execute(&query).unwrap_err();
        assert_eq!(
            (
                error.code,
                error.resource_counter.as_deref(),
                error.configured_bound
            ),
            (
                ErrorCode::ResourceLimit,
                Some("signed_list_ordinal_bits"),
                Some(64)
            )
        );
        assert!(!serde_json::to_string(&error).unwrap().contains("922337"));
    }
    assert_eq!(
        data(
            "<ol start='9223372036854775807'><li>A</li><li value='0'>B</li></ol>",
            json!({"version":6,"select":"ol","read":"markdown"})
        ),
        json!(["- 9223372036854775807\\. A\n- 0\\. B"])
    );
}

#[test]
fn receipt_failure_can_exhaust_query_hash_work_and_debug_preserves_essential_counts() {
    let source = document("<p>A</p>");
    let mut query = ExtractionPlan::css("p").unwrap();
    // A large explicit query exclusion scans no extra nodes when it is a CSS comment.
    query.exclude = Some(vec![format!("q/*{}*/", "x".repeat(8000))]);
    query.limits.max_work = 100;
    let result = source
        .execute(&CompiledPlan::compile(&query).unwrap())
        .unwrap();
    assert_eq!(result.payload(), br#"["A"]"#);
    let failure = result.receipt().unwrap_err();
    assert_eq!(failure.resource_counter.as_deref(), Some("max_work"));
    assert_eq!(result.receipt().unwrap_err(), failure);
    let debug = format!("{result:?}");
    assert!(debug.contains("candidate_count: 1") && debug.contains("selected_count: 1"));
}

#[test]
fn url_processing_cap_rejects_real_uts46_expansion_at_its_exact_edge() {
    let prefix = format!("https://{}a/", "㌖.".repeat(1926));
    let query = compile(json!({"version":6,"select":"a","read":"url:href"}));
    for (extra, accepted) in [(16, true), (17, false)] {
        let url = format!("{prefix}{}", "x".repeat(extra));
        assert!(url.len() <= 8192);
        let source = document(&format!("<a href='{url}'>label</a>"));
        let result = source.execute(&query);
        if accepted {
            let result = result.unwrap();
            let value = &result.data().as_values().unwrap()[0];
            assert_eq!(value.len(), 32768);
            assert!(value.starts_with("https://xn--nckucudvbh5g."));
        } else {
            let error = result.unwrap_err();
            assert_eq!(
                (
                    error.code,
                    error.resource_counter.as_deref(),
                    error.configured_bound
                ),
                (
                    ErrorCode::ResourceLimit,
                    Some("url_processing_bytes"),
                    Some(32768)
                )
            );
        }
    }
}

#[test]
fn rust_query_encoding_admits_escaped_defaults_before_source_execution() {
    let mut plan = ExtractionPlan::css("p").unwrap();
    plan.expect.push(Guard {
        select: "p".into(),
        scope: GuardScope::Document,
        min: 1,
        max: 1,
        read: None,
        equals: Some("\u{1}".repeat(50_000)),
        pattern: None,
    });
    plan.validate().unwrap();
    let error = CompiledPlan::compile(&plan).err().unwrap();
    assert_eq!(error.code, ErrorCode::ResourceLimit);
    assert_eq!(
        (error.resource_counter.as_deref(), error.configured_bound),
        (Some("query_bytes"), Some(MAX_PLAN_BYTES as u64))
    );
}

#[test]
fn complete_counts_and_cell_resources_do_not_clamp_assumptions() {
    let query = compile(json!({"version":6,"select":"p","match":"nth","index":2}));
    let result = document("<p>A</p><p>B</p><p>C</p>")
        .execute(&query)
        .unwrap();
    assert_eq!((result.candidate_count(), result.selected_count()), (3, 1));
    assert_eq!(result.payload(), br#"["B"]"#);
    let query = compile(
        json!({"version":6,"select":"p","match":"all","max":3,"limits":{"max_selected":2}}),
    );
    let error = document("<p>A</p><p>B</p><p>C</p>")
        .execute(&query)
        .unwrap_err();
    assert_eq!(
        (error.code, error.candidate_count),
        (ErrorCode::ResourceLimit, Some(3))
    );
    let query = compile(
        json!({"version":6,"select":"article","fields":{"a":{"select":".absent","match":"optional"},"b":{"select":".absent","match":"all","min":0}},"limits":{"max_cells":1}}),
    );
    let error = document("<article></article>").execute(&query).unwrap_err();
    assert_eq!(
        (error.code, error.field_name.as_deref()),
        (ErrorCode::ResourceLimit, Some("b"))
    );
}

#[test]
fn json_escaping_cannot_publish_data_beyond_the_encoded_byte_bound() {
    let source = format!("<article data-value='{}'></article>", "\u{1}".repeat(20)).repeat(10_000);
    let fields = (0..64)
        .map(|index| {
            (
                format!("f{index:02}{}", "x".repeat(61)),
                json!({"select":":scope","read":"attr:data-value"}),
            )
        })
        .collect::<serde_json::Map<_, _>>();
    // 64 keys of 64 ASCII bytes and twenty six-byte escapes per value yield
    // 121,620,001 encoded bytes for 10,000 rows, despite only 12,800,000 leaf bytes.
    let plan = compile(
        json!({"version":6,"select":"article","match":"all","fields":fields,
        "limits":{"max_cells":1_000_000,"max_work":10_000_000}}),
    );
    let error = document(&source).execute(&plan).unwrap_err();
    assert_eq!(error.code, ErrorCode::ResourceLimit);
    assert_eq!(error.resource_counter.as_deref(), Some("encoded_bytes"));
    assert_eq!(error.configured_bound, Some(64 * 1024 * 1024));
}
