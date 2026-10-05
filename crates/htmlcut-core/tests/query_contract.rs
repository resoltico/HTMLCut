// SPDX-License-Identifier: MPL-2.0
//! Version-six contracts with independent values and rejecting controls.
use htmlcut_core::*;
use serde_json::{Value, json};

#[path = "support/query.rs"]
mod query_support;
use query_support::{compile, data, document};

#[test]
fn closed_members_nulls_duplicates_and_retired_contracts_are_rejected() {
    for query in [
        r#"{"version":5,"select":"p"}"#,
        r#"{"version":6.0,"select":"p"}"#,
        r#"{"version":6,"select":"p","select":"q"}"#,
        r#"{"version":6,"select":"p","fields":{"a":{"select":"p","select":"q"}}}"#,
        r#"{"version":6,"select":"p","fields":{"a":{"select":"p"},"a":{"select":"q"}}}"#,
        r#"{"version":6,"select":"p","expect":[{"select":"p","min":0,"min":1}]}"#,
        r#"{"version":6,"select":"p","limits":{"max_cells":1,"max_cells":2}}"#,
        r#"{"version":6,"select":"p","schema":"htmlcut.extraction.plan"}"#,
        r#"{"version":6,"select":"p","strategy":{"kind":"css","selector":"p"}}"#,
    ] {
        assert!(
            ExtractionPlan::from_json(query.as_bytes()).is_err(),
            "{query}"
        );
    }
    for member in [
        "match",
        "min",
        "max",
        "index",
        "read",
        "exclude",
        "fields",
        "expect",
        "following_siblings",
        "limits",
    ] {
        let mut query = json!({"version":6,"select":"p"});
        query[member] = Value::Null;
        assert!(
            ExtractionPlan::from_json(&serde_json::to_vec(&query).unwrap()).is_err(),
            "{member}"
        );
    }
    for member in ["select", "match", "min", "max", "index", "read", "exclude"] {
        let mut field = json!({"select":"p"});
        field[member] = Value::Null;
        let query = json!({"version":6,"select":"p","fields":{"a":field}});
        assert!(
            ExtractionPlan::from_json(&serde_json::to_vec(&query).unwrap()).is_err(),
            "{member}"
        );
    }
}
#[test]
fn supplied_defaults_do_not_hide_inapplicable_members() {
    for extra in [
        json!({"fields":{}}),
        json!({"min":1}),
        json!({"max":1}),
        json!({"index":1}),
        json!({"following_siblings":0}),
        json!({"match":"optional"}),
        json!({"match":"nth"}),
        json!({"match":"all","min":2,"max":1}),
    ] {
        let mut query = json!({"version":6,"select":"p"});
        query
            .as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        assert!(ExtractionPlan::from_json(&serde_json::to_vec(&query).unwrap()).is_err());
    }
    for extra in [json!({"read":"text"}), json!({"exclude":[]})] {
        let mut query = json!({"version":6,"select":"p","fields":{"a":{"select":"p"}}});
        query
            .as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        assert!(ExtractionPlan::from_json(&serde_json::to_vec(&query).unwrap()).is_err());
    }
    for guard in [
        json!({"select":"p","read":"text"}),
        json!({"select":"p","read":"attr:x"}),
        json!({"select":"p","equals":"x","pattern":"x"}),
        json!({"select":"p","equals":"x","read":"markdown"}),
    ] {
        assert!(
            ExtractionPlan::from_json(
                &serde_json::to_vec(&json!({"version":6,"select":"p","expect":[guard]})).unwrap()
            )
            .is_err()
        );
    }
}
#[test]
fn canonical_defaults_and_field_order_preserve_explicit_assumptions() {
    let a = compile(
        json!({"version":6,"select":"article","match":"all","fields":{"z":{"select":".z"},"a":{"select":".a"}}}),
    );
    let b = compile(
        json!({"select":"article","version":6,"min":1,"following_siblings":0,"limits":{},"match":"all","expect":[],"fields":{"a":{"select":".a","read":"text","match":"one","exclude":[]},"z":{"select":".z"}}}),
    );
    assert_eq!(a.normalized_json(), b.normalized_json());
    assert_eq!(a.plan_sha256(), b.plan_sha256());
    let c = compile(
        json!({"version":6,"select":"article","match":"all","max":10000,"fields":{"a":{"select":".a"},"z":{"select":".z"}}}),
    );
    assert_ne!(a.normalized_json(), c.normalized_json());
    let source = document("<article><i class=a>A</i><i class=z>Z</i></article>");
    let first = source.execute(&a).unwrap();
    let second = source.execute(&b).unwrap();
    assert_eq!(first.payload(), br#"[{"a":"A","z":"Z"}]"#);
    assert_eq!(first.receipt().unwrap(), second.receipt().unwrap());
    assert_eq!(
        first
            .receipt()
            .unwrap()
            .fields
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["a", "z"]
    );
}
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
        let mut query = json!({"version":6,"select":"article","fields":{}});
        query["fields"][key] = json!({"select":"p"});
        let error = ExtractionPlan::from_json(&serde_json::to_vec(&query).unwrap()).unwrap_err();
        assert!(!serde_json::to_string(&error).unwrap().contains(key) || key.is_empty());
    }
    let query = compile(
        json!({"version":6,"select":"article","match":"nth","index":2,"fields":{"z":{"select":".missing"},"a":{"select":".absent"}}}),
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
    let query = json!({"version":6,"select":"article","match":"all","fields":{
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
        json!({"version":6,"select":"article","fields":{"href":{"select":"a","match":"optional","read":"attr:href"}}}),
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
            data(source, json!({"version":6,"select":select})),
            json!([expected]),
            "{source}"
        );
    }
    assert_eq!(
        data(
            "<div>A<script>S</script><p>B</p></div>",
            json!({"version":6,"select":"div","read":"literal"})
        ),
        json!(["ASB"])
    );
    assert_eq!(
        data(
            "<template><p>inside</p></template>",
            json!({"version":6,"select":"template"})
        ),
        json!([""])
    );
    assert_eq!(
        data(
            "<template><p>inside</p></template>",
            json!({"version":6,"select":"p"})
        ),
        json!(["inside"])
    );
}
#[test]
fn guards_check_original_content_count_only_presence_and_empty_roots() {
    let query =
        json!({"version":6,"select":".price","expect":[{"select":"h1","equals":"Product A"}]});
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
    let empty = json!({"version":6,"select":".missing","match":"all","min":0,"expect":[{"select":"[data-x]"}]});
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
            json!({"version":6,"select":".missing","match":"all","min":0,"expect":[{"select":".missing","scope":"selected","equals":"impossible"}]})
        ),
        json!([])
    );
    assert_eq!(
        data(
            "<div>A<i> B</i></div>",
            json!({"version":6,"select":"div","exclude":["i"],"expect":[{"select":"div","equals":"A B"}]})
        ),
        json!(["A"])
    );
}
#[test]
fn result_evidence_is_bound_immutable_optional_and_failure_cached() {
    let source = document("<p>A</p>");
    let mut plan = ExtractionPlan::css("p").unwrap();
    let compiled = CompiledPlan::compile(&plan).unwrap();
    let first = source.execute(&compiled).unwrap();
    plan.select = "wrong".into();
    assert_eq!(first.payload(), br#"["A"]"#);
    assert_eq!(first.normalized_json(), compiled.normalized_json());
    let receipt = first.receipt().unwrap();
    assert_eq!(receipt, first.receipt().unwrap());
    let other = document("<p>A</p><!-- different -->")
        .execute(&compiled)
        .unwrap();
    assert_eq!(other.payload(), first.payload());
    assert_ne!(
        other.receipt().unwrap().source_sha256,
        receipt.source_sha256
    );
    assert_ne!(
        other.receipt().unwrap().extraction_sha256,
        receipt.extraction_sha256
    );
    let huge = document(&format!("<p>A</p><!--{}-->", "x".repeat(100000)));
    let limited = compile(json!({"version":6,"select":"p","limits":{"max_work":100}}));
    let result = huge.execute(&limited).unwrap();
    assert_eq!(result.payload(), br#"["A"]"#);
    let error = result.receipt().unwrap_err();
    assert_eq!(error.code, ErrorCode::ResourceLimit);
    assert_eq!(error, result.receipt().unwrap_err());
}
#[test]
fn bounded_observations_stop_at_emitted_content_and_keep_complete_headers() {
    let source = format!(
        "<main><p>{}</p><p>{}</p></main>",
        "A".repeat(40_000_000),
        "B".repeat(100)
    );
    let observed = document(&source).inspect("p", 2).unwrap();
    assert_eq!(observed.count, 2);
    assert_eq!(observed.samples[0].text, "A".repeat(160));
    assert!(!observed.samples[0].text_complete);
    assert_eq!(observed.samples[1].text, "B".repeat(100));
    assert!(observed.samples[1].text_complete);
    let source = format!("<p>{} <script>omitted</script></p>", "A".repeat(160));
    let observed = document(&source).inspect("p", 1).unwrap();
    assert!(observed.samples[0].text_complete);
    let observed = document("<table><tr><th> A <b>B</b><script>S</script></th></tr><tr><td>1</td></tr><tr><td>2</td></tr></table>").survey(None,4).unwrap();
    let table = observed.groups[0].table.as_ref().unwrap();
    assert_eq!(table.header_read, "text");
    assert_eq!(table.headers, ["A B"]);
    assert!(table.headers_complete);
    for (length, complete) in [(32, true), (33, false)] {
        let header = "🦀".repeat(length);
        let source = format!(
            "<table><tr><th>{header}</th></tr><tr><td>1</td></tr><tr><td>2</td></tr></table>"
        );
        let observed = document(&source).survey(None, 4).unwrap();
        let shape = observed.groups[0].table.as_ref().unwrap();
        assert_eq!(shape.headers_complete, complete);
        assert_eq!(shape.headers, if complete { vec![header] } else { vec![] });
    }
    let parent_tag = "x".repeat(129);
    let source =
        format!("<{parent_tag} class=parent><main><p>A</p><p>B</p><p>C</p></main></{parent_tag}>");
    let observed = document(&source).survey(Some("main"), 4).unwrap();
    assert_eq!(observed.groups[0].selector.as_deref(), Some("p"));
}
#[test]
fn generated_query_schema_has_omission_only_members_and_distinct_match_contracts() {
    let contract = schema("htmlcut.extraction.plan").unwrap();
    let required = contract["required"].as_array().unwrap();
    assert_eq!(required, &vec![json!("version"), json!("select")]);
    let location = "https://schemas.htmlcut.invalid/query-v6.json";
    let mut schemas = boon::Schemas::new();
    let mut compiler = boon::Compiler::new();
    compiler.add_resource(location, contract).unwrap();
    let index = compiler.compile(location, &mut schemas).unwrap();
    for valid in [
        json!({"version":6,"select":"p"}),
        json!({"version":6,"select":"p","fields":{"a":{"select":"p","match":"optional"}}}),
        json!({"version":6,"select":"p","expect":[{"select":"p","equals":"A"}]}),
        json!({"version":6,"select":"p","limits":{"max_work":100}}),
    ] {
        assert!(schemas.validate(&valid, index).is_ok(), "{valid}");
        assert!(ExtractionPlan::from_json(&serde_json::to_vec(&valid).unwrap()).is_ok());
    }
    let invalid = json!({"version":6,"select":"p","match":"optional"});
    assert!(schemas.validate(&invalid, index).is_err());
    for member in [
        "min",
        "max",
        "index",
        "read",
        "exclude",
        "fields",
        "following_siblings",
        "limits",
        "expect",
    ] {
        let mut invalid = json!({"version":6,"select":"p"});
        invalid[member] = Value::Null;
        assert!(schemas.validate(&invalid, index).is_err(), "{member}");
    }
    for member in ["min", "max", "index", "read", "exclude"] {
        let mut invalid = json!({"version":6,"select":"p","fields":{"a":{"select":"p"}}});
        invalid["fields"]["a"][member] = Value::Null;
        assert!(schemas.validate(&invalid, index).is_err(), "field {member}");
    }
    for member in ["read", "equals", "pattern"] {
        let mut invalid = json!({"version":6,"select":"p","expect":[{"select":"p"}]});
        invalid["expect"][0][member] = Value::Null;
        assert!(
            schemas.validate(&invalid, index).is_err(),
            "expect {member}"
        );
    }
}

#[test]
fn root_and_field_cardinality_reject_every_inapplicable_count_position() {
    for members in [
        json!({"match":"all","index":1}),
        json!({"match":"nth","index":1,"min":0}),
        json!({"match":"nth","index":1,"max":1}),
        json!({"match":"one","max":1}),
    ] {
        let mut root = json!({"version":6,"select":"p"});
        root.as_object_mut()
            .unwrap()
            .extend(members.as_object().unwrap().clone());
        assert!(ExtractionPlan::from_json(&serde_json::to_vec(&root).unwrap()).is_err());
        let mut field = json!({"select":"p"});
        field
            .as_object_mut()
            .unwrap()
            .extend(members.as_object().unwrap().clone());
        let root = json!({"version":6,"select":"p","fields":{"value":field}});
        assert!(ExtractionPlan::from_json(&serde_json::to_vec(&root).unwrap()).is_err());
    }
}

#[test]
fn optional_field_ambiguity_reports_its_zero_to_one_contract() {
    let error = document("<article><b>A</b><b>B</b></article>")
        .execute(&compile(json!({"version":6,"select":"article","fields":{
            "author":{"select":"b","match":"optional"}}})))
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::AmbiguousSelection);
    assert_eq!(error.field_name.as_deref(), Some("author"));
    assert_eq!(error.candidate_count, Some(2));
    assert_eq!((error.expected_min, error.expected_max), (Some(0), Some(1)));
    assert_eq!(error.message, "Optional field matched multiple candidates.");
}
