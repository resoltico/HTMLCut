// SPDX-License-Identifier: MPL-2.0
//! Current query contracts with independent values and rejecting controls.
use htmlcut_core::*;
use serde_json::{Value, json};

#[path = "support/query.rs"]
mod query_support;
use query_support::{compile, data, document};

#[test]
fn names_are_validated_and_first_failure_is_lexical_in_selected_output_order() {
    for key in [
        "",
        "1x",
        "bad key",
        "bad.name",
        "SYNTHETIC_SECRET!",
        &"x".repeat(65),
    ] {
        let mut query = json!({"version":8,"select":"article","fields":{}});
        query["fields"][key] = json!({"select":"p"});
        let error = ExtractionPlan::from_json(&serde_json::to_vec(&query).unwrap()).unwrap_err();
        assert!(!serde_json::to_string(&error).unwrap().contains(key) || key.is_empty());
    }
    let query = compile(
        json!({"version":8,"select":"article","match":"nth","index":2,"fields":{"z":{"select":".missing"},"a":{"select":".absent"}}}),
    );
    let error = document("<article></article><article></article>")
        .execute(&query)
        .unwrap_err();
    assert_eq!(
        (error.field_name.as_deref(), error.row_index),
        (Some("a"), Some(1))
    );
}
#[test]
fn fields_preserve_null_empty_arrays_attributes_and_relationships() {
    let query = json!({"version":8,"select":"article","match":"all","fields":{
        "optional":{"select":".absent","match":"optional"},"empty":{"select":"i"},
        "tags":{"select":"b","match":"all","min":0},"href":{"select":"a","read":"attr:href"}}});
    assert_eq!(
        data(
            "<article><i></i><a href=''></a><b>X</b><b>Y</b></article><article><i>E</i><a href='/a'></a></article>",
            query.clone()
        ),
        json!([
        {"optional":null,"empty":"","tags":["X","Y"],"href":""}, {"optional":null,"empty":"E","tags":[],"href":"/a"}])
    );
    let optional = compile(
        json!({"version":8,"select":"article","fields":{"href":{"select":"a","match":"optional","read":"attr:href"}}}),
    );
    assert_eq!(
        document("<article><a></a></article>")
            .execute(&optional)
            .unwrap_err()
            .code,
        ErrorCode::MissingAttribute
    );
    assert_eq!(
        document("<article><a href=''></a><a href=''></a></article>")
            .execute(&optional)
            .unwrap_err()
            .code,
        ErrorCode::AmbiguousSelection
    );
}
#[test]
fn structural_text_has_independent_boundary_inert_and_pre_answers() {
    for (source, select, expected) in [
        (
            "<div>A<span>B</span>C<p>D</p><br>E<details><summary>F</summary>G</details><address>H</address></div>",
            "div",
            "ABC D E F G H",
        ),
        (
            "<div> A\u{a0}B\u{2003}C\u{200b}D <script>secret</script><style>secret</style><template>secret</template><i hidden>H</i><noscript>N</noscript></div>",
            "div",
            "A B C\u{200b}D HN",
        ),
        ("<pre>\n A\r\nB &amp;\n</pre>", "pre", " A\nB &\n"),
        ("<pre>edge<span> A\n B </span>end</pre>", "span", " A\n B "),
        ("<div>A<pre>\n B\n</pre>C</div>", "div", "A B\nC"),
        ("<script>secret</script>", "script", ""),
        (
            "<svg><script>secret</script><style>secret</style><text>A</text><foreignObject><div>B</div><div>C</div></foreignObject></svg>",
            "svg",
            "A B C",
        ),
    ] {
        assert_eq!(
            data(source, json!({"version":8,"select":select})),
            json!([expected]),
            "{source}"
        );
    }
    assert_eq!(
        data(
            "<div>A<script>S</script><p>B</p></div>",
            json!({"version":8,"select":"div","read":"literal"})
        ),
        json!(["ASB"])
    );
    assert_eq!(
        data(
            "<template><p>inside</p></template>",
            json!({"version":8,"select":"template"})
        ),
        json!([""])
    );
    assert_eq!(
        data(
            "<template><p>inside</p></template>",
            json!({"version":8,"select":"p"})
        ),
        json!(["inside"])
    );
}
#[test]
fn guards_check_original_content_count_only_presence_and_empty_roots() {
    let query =
        json!({"version":8,"select":".price","expect":[{"select":"h1","equals":"Product A"}]});
    assert_eq!(
        data(
            "<h1>Product <b>A</b></h1><p class=price>10</p>",
            query.clone()
        ),
        json!(["10"])
    );
    for source in [
        "<h1>Product B</h1><p class=price>10</p>",
        "<h1>Product A</h1><h1>Product A</h1><p class=price>10</p>",
    ] {
        assert_eq!(
            document(source)
                .execute(&compile(query.clone()))
                .unwrap_err()
                .code,
            ErrorCode::GuardFailed
        );
    }
    let empty = json!({"version":8,"select":".missing","match":"all","min":0,"expect":[{"select":"[data-x]"}]});
    assert_eq!(data("<p data-x=''></p>", empty.clone()), json!([]));
    assert_eq!(
        document("<p></p>")
            .execute(&compile(empty))
            .unwrap_err()
            .code,
        ErrorCode::GuardFailed
    );
    assert_eq!(
        data(
            "<p></p>",
            json!({"version":8,"select":".missing","match":"all","min":0,"expect":[{"select":".missing","scope":"selected","equals":"impossible"}]})
        ),
        json!([])
    );
    assert_eq!(
        data(
            "<div>A<i> B</i></div>",
            json!({"version":8,"select":"div","exclude":["i"],"expect":[{"select":"div","equals":"A B"}]})
        ),
        json!(["A"])
    );
}

#[test]
fn root_and_field_cardinality_reject_every_inapplicable_count_position() {
    for members in [
        json!({"match":"all","index":1}),
        json!({"match":"nth","index":1,"min":0}),
        json!({"match":"nth","index":1,"max":1}),
        json!({"match":"one","max":1}),
    ] {
        let mut root = json!({"version":8,"select":"p"});
        root.as_object_mut()
            .unwrap()
            .extend(members.as_object().unwrap().clone());
        assert!(ExtractionPlan::from_json(&serde_json::to_vec(&root).unwrap()).is_err());
        let mut field = json!({"select":"p"});
        field
            .as_object_mut()
            .unwrap()
            .extend(members.as_object().unwrap().clone());
        let root = json!({"version":8,"select":"p","fields":{"value":field}});
        assert!(ExtractionPlan::from_json(&serde_json::to_vec(&root).unwrap()).is_err());
    }
}

#[test]
fn optional_field_ambiguity_reports_its_zero_to_one_contract() {
    let error = document("<article><b>A</b><b>B</b></article>")
        .execute(&compile(json!({"version":8,"select":"article","fields":{
            "author":{"select":"b","match":"optional"}}})))
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::AmbiguousSelection);
    assert_eq!(error.field_name.as_deref(), Some("author"));
    assert_eq!(error.candidate_count, Some(2));
    assert_eq!((error.expected_min, error.expected_max), (Some(0), Some(1)));
    assert_eq!(error.message, "Optional field matched multiple candidates.");
}

#[test]
fn normalized_field_modes_preserve_omission_and_roundtrip_idempotently() {
    let implicit = json!({"version":8,"select":"article","fields":{
        "many":{"select":"i","match":"all"}, "one":{"select":"b"},
        "optional":{"select":"u","match":"optional"}, "nth":{"select":"a","match":"nth","index":2}}});
    let mut explicit = implicit.clone();
    explicit["fields"]["many"]["min"] = json!(1);
    let first = compile(implicit);
    assert_eq!(first.normalized_json(), compile(explicit).normalized_json());
    let normalized: Value = serde_json::from_str(first.normalized_json()).unwrap();
    assert_eq!(normalized["fields"]["many"]["min"], 1);
    for name in ["one", "optional", "nth"] {
        assert!(normalized["fields"][name].get("min").is_none());
        assert!(normalized["fields"][name].get("max").is_none());
    }
    let roundtrip = ExtractionPlan::from_json(first.normalized_json().as_bytes()).unwrap();
    assert_eq!(
        CompiledPlan::compile(&roundtrip).unwrap().normalized_json(),
        first.normalized_json()
    );
}

#[test]
fn malformed_field_members_report_safe_names_and_paths() {
    let reject =
        |query: Value| ExtractionPlan::from_json(&serde_json::to_vec(&query).unwrap()).unwrap_err();
    for name in ["author", "_price2", "fields", "read"] {
        let query = json!({"version":8,"select":"p","fields":{name:{"select":"p","read":false}}});
        let error = reject(query);
        assert_eq!(error.field_name.as_deref(), Some(name));
        assert_eq!(
            error.plan_path.as_deref(),
            Some(format!("$.fields.{name}.read").as_str())
        );
    }
    for name in ["unsafe name", "private.password", "${secret}"] {
        let error = reject(json!({"version":8,"select":"p","fields":{name:false}}));
        assert!(error.field_name.is_none());
        assert_eq!(error.plan_path.as_deref(), Some("$.fields"));
        assert!(!serde_json::to_string(&error).unwrap().contains(name));
    }
    for query in [
        json!({"version":8,"select":false}),
        json!({"version":8,"select":"p","secret":false}),
    ] {
        let error = reject(query);
        assert!(error.field_name.is_none());
        assert!(!error.plan_path.as_deref().unwrap().contains("secret"));
    }
    let error =
        reject(json!({"version":8,"select":"p","fields":{"author":{"select":"p","secret":false}}}));
    assert_eq!(error.field_name.as_deref(), Some("author"));
    assert!(!error.plan_path.as_deref().unwrap().contains("secret"));
}
