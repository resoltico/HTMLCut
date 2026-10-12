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
        br#"{"version":8,"select":"article","fields":{"price":{"select":"["}}}"#,
    )
    .unwrap();
    let error = CompiledPlan::compile(&parsed).err().unwrap();
    assert_eq!(error.field_name.as_deref(), Some("price"));
    assert_eq!(error.plan_path.as_deref(), Some("fields.price.select"));
    for query in [
        json!({"version":8,"select":"p","match":"nth","index":3,"limits":{"max_candidates":2}}),
        json!({"version":8,"select":"p","expect":[{"select":"p","max":3}],"limits":{"max_candidates":2}}),
    ] {
        assert!(ExtractionPlan::from_json(&serde_json::to_vec(&query).unwrap()).is_err());
    }
    let error = document("<p>A</p><p>B</p>")
        .execute(&compile(
            json!({"version":8,"select":"p","match":"all","expect":[{"select":"p"}]}),
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
            json!({"version":8,"select":"p","limits":{"max_value_bytes":1}}),
        ))
        .unwrap_err();
    assert_eq!(
        (error.resource_counter.as_deref(), error.configured_bound),
        (Some("max_value_bytes"), Some(1))
    );
    let error = document("<p>AB</p><p>CD</p>")
        .execute(&compile(
            json!({"version":8,"select":"p","match":"all","limits":{"max_total_value_bytes":3}}),
        ))
        .unwrap_err();
    assert_eq!(
        (error.resource_counter.as_deref(), error.configured_bound),
        (Some("max_total_value_bytes"), Some(3))
    );
    let huge = format!("<p>{}</p>", "A".repeat(10000));
    let error = document(&huge).execute(&compile(json!({"version":8,"select":"p","limits":{"max_work":100,"max_total_value_bytes":15000}}))).unwrap_err();
    assert_eq!(
        (error.resource_counter.as_deref(), error.configured_bound),
        (Some("max_work"), Some(100))
    );
    let error = document("<p>AB</p>").execute(&compile(json!({"version":8,"select":"p","read":"literal","expect":[{"select":"p","equals":"AB"}],"limits":{"max_value_bytes":1}}))).unwrap_err();
    assert_eq!(
        (error.resource_counter.as_deref(), error.configured_bound),
        (Some("max_value_bytes"), Some(1))
    );
}

#[test]
fn url_processing_cap_rejects_real_uts46_expansion_at_its_exact_edge() {
    let prefix = format!("https://{}a/", "㌖.".repeat(1926));
    let query = compile(json!({"version":8,"select":"a","read":"url:href"}));
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
    let query = compile(json!({"version":8,"select":"p","match":"nth","index":2}));
    let result = document("<p>A</p><p>B</p><p>C</p>")
        .execute(&query)
        .unwrap();
    assert_eq!((result.candidate_count(), result.selected_count()), (3, 1));
    assert_eq!(serde_json::to_vec(result.data()).unwrap(), br#"["B"]"#);
    let query = compile(
        json!({"version":8,"select":"p","match":"all","max":3,"limits":{"max_selected":2}}),
    );
    let error = document("<p>A</p><p>B</p><p>C</p>")
        .execute(&query)
        .unwrap_err();
    assert_eq!(
        (error.code, error.candidate_count),
        (ErrorCode::ResourceLimit, Some(3))
    );
    let query = compile(
        json!({"version":8,"select":"article","fields":{"a":{"select":".absent","match":"optional"},"b":{"select":".absent","match":"all","min":0}},"limits":{"max_cells":1}}),
    );
    let error = document("<article></article>").execute(&query).unwrap_err();
    assert_eq!(
        (error.code, error.field_name.as_deref()),
        (ErrorCode::ResourceLimit, Some("b"))
    );
}

#[test]
fn url_limit_ownership_distinguishes_normalization_and_the_fixed_processing_cap() {
    let error = document("<a href='https://a'>A</a>")
        .execute(&compile(
            json!({"version":8,"select":"a","read":"url:href","limits":{"max_value_bytes":9}}),
        ))
        .unwrap_err();
    assert_eq!(error.resource_counter.as_deref(), Some("max_value_bytes"));
    assert_eq!(error.configured_bound, Some(9));
    let url = format!("https://{}a/{}", "㌖.".repeat(1926), "x".repeat(17));
    let source = format!("<a href='{url}'>A</a>");
    let error = document(&source)
        .execute(&compile(
            json!({"version":8,"select":"a","read":"url:href","limits":{"max_value_bytes":32768}}),
        ))
        .unwrap_err();
    assert_eq!(
        error.resource_counter.as_deref(),
        Some("url_processing_bytes")
    );
    assert_eq!(error.configured_bound, Some(32768));
}

#[test]
fn text_does_not_interpret_numeric_metadata_as_list_ordinals() {
    assert_eq!(
        data(
            "<ol start='9223372036854775808'><li>A</li><li>B</li></ol>",
            json!({"version":8,"select":"ol"})
        ),
        json!(["A B"])
    );
}
