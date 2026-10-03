//! Public-boundary record contracts with independent complete JSON answers.

use htmlcut_core::*;
use serde_json::json;

fn document(source: &str) -> PreparedDocument {
    PreparedDocument::new(
        SourceSnapshot::new(source, SnapshotMetadata::default()).unwrap(),
        PreparationLimits::default(),
    )
    .unwrap()
}

fn compile(value: serde_json::Value) -> CompiledPlan {
    let plan = ExtractionPlan::from_json(&serde_json::to_vec(&value).unwrap()).unwrap();
    CompiledPlan::compile(&plan).unwrap()
}

fn plan(fields: serde_json::Value) -> serde_json::Value {
    json!({"schema":"htmlcut.extraction.plan","version":3,
        "strategy":{"kind":"css","selector":"article"},"selection":{"kind":"all"},
        "projection":{"kind":"records","fields":fields}})
}

#[test]
fn complete_rows_preserve_field_relationships_and_counts() {
    let source = document(
        "<article id='a'><h2>First</h2><p class='price'>10</p><a href='/a'></a></article><article id='b'><h2>Second</h2><p class='price'>20</p><a href=''></a></article>",
    );
    let query = compile(plan(json!([
        {"name":"title","selector":"h2"},
        {"name":"price","selector":".price"},
        {"name":"href","selector":"a","projection":{"kind":"attribute","name":"href"}},
        {"name":"id","selector":":scope","projection":{"kind":"attribute","name":"id"}}
    ])));
    for _ in 0..2 {
        let result = source.execute(&query).unwrap();
        assert_eq!(
            serde_json::to_value(&result.data).unwrap(),
            json!([
                {"id":"a","title":"First","price":"10","href":"/a"},
                {"id":"b","title":"Second","price":"20","href":""}
            ])
        );
        assert_eq!(
            (
                result.receipt.candidate_count,
                result.receipt.selected_count
            ),
            (2, 2)
        );
        assert_eq!(result.receipt.data_kind, DataKind::Records);
        assert!(
            result
                .receipt
                .fields
                .iter()
                .all(|f| f.candidate_count == 2 && f.projected_count == 2 && f.absent_count == 0)
        );
        assert_eq!(
            result
                .receipt
                .fields
                .iter()
                .map(|f| f.field_index)
                .collect::<Vec<_>>(),
            [1, 2, 3, 4]
        );
    }
}

#[test]
fn optional_absence_empty_text_and_empty_all_are_distinct() {
    let source = document("<article><span class='empty'></span></article>");
    let query = compile(plan(json!([
        {"name":"absent","selector":".missing","selection":{"kind":"optional"}},
        {"name":"empty","selector":".empty"},
        {"name":"all","selector":".missing","selection":{"kind":"all","min":0}}
    ])));
    let result = source.execute(&query).unwrap();
    assert_eq!(
        serde_json::to_value(&result.data).unwrap(),
        json!([{"absent":null,"empty":"","all":[]}])
    );
    assert_eq!(result.receipt.fields[0].absent_count, 1);
    assert_eq!(result.receipt.fields[1].projected_count, 1);
    assert_eq!(result.receipt.fields[2].projected_count, 0);
}

#[test]
fn optionality_does_not_suppress_missing_attribute_or_duplicate_nodes() {
    let query = compile(plan(json!([
        {"name":"href","selector":"a","selection":{"kind":"optional"},"projection":{"kind":"attribute","name":"href"}}
    ])));
    for (html, code) in [
        (
            "<article><a>missing</a></article>",
            ErrorCode::MissingAttribute,
        ),
        (
            "<article><a href='a'></a><a href='b'></a></article>",
            ErrorCode::AmbiguousSelection,
        ),
    ] {
        let error = document(html).execute(&query).unwrap_err();
        assert_eq!(error.code, code);
        assert_eq!((error.row_index, error.field_index), (Some(1), Some(1)));
    }
}

#[test]
fn payload_is_row_contained_while_original_predicates_retain_context() {
    let source = document(
        "<body class='context'><article><p class='price'>10</p></article><article><p class='price'>20</p></article></body>",
    );
    let query = compile(plan(json!([
        {"name":"price","selector":"body.context .price"},
        {"name":"neighbor","selector":":scope + article .price","selection":{"kind":"optional"}}
    ])));
    assert_eq!(
        serde_json::to_value(&source.execute(&query).unwrap().data).unwrap(),
        json!([{"price":"10","neighbor":null},{"price":"20","neighbor":null}])
    );
}

#[test]
fn later_row_failure_and_cell_exhaustion_reject_whole_execution() {
    let source = document("<article><b>A</b><i>B</i></article><article><b>C</b></article>");
    let query = compile(plan(
        json!([{"name":"first","selector":"b"},{"name":"second","selector":"i"}]),
    ));
    let error = source.execute(&query).unwrap_err();
    assert_eq!(error.code, ErrorCode::NoMatch);
    assert_eq!((error.row_index, error.field_index), (Some(2), Some(2)));
    let mut request =
        plan(json!([{"name":"first","selector":"b"},{"name":"second","selector":"i"}]));
    request["limits"] = json!({"max_cells":3});
    let error = document("<article><b>A</b><i>B</i></article><article><b>C</b><i>D</i></article>")
        .execute(&compile(request))
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::ResourceLimit);
    assert_eq!((error.row_index, error.field_index), (Some(2), Some(2)));
}

#[test]
fn field_names_and_grammar_are_closed_and_old_wire_is_rejected() {
    for fields in [
        json!([]),
        json!([{"name":"a","selector":"b"},{"name":"a","selector":"i"}]),
        json!([{"name":"bad name","selector":"b"}]),
        json!([{"name":"nested","selector":"b","projection":{"kind":"records","fields":[]}}]),
        json!([{"name":"source","selector":"b","projection":{"kind":"source"}}]),
        json!([{"name":"a","selector":"b","unknown":true}]),
    ] {
        assert!(ExtractionPlan::from_json(&serde_json::to_vec(&plan(fields)).unwrap()).is_err());
    }
    let mut old = plan(json!([{"name":"a","selector":"b"}]));
    old["version"] = json!(2);
    assert_eq!(
        ExtractionPlan::from_json(&serde_json::to_vec(&old).unwrap())
            .unwrap_err()
            .code,
        ErrorCode::InvalidSchema
    );
    let malformed = plan(json!([{"name":"a","selector":"["}]));
    let parsed = ExtractionPlan::from_json(&serde_json::to_vec(&malformed).unwrap()).unwrap();
    assert_eq!(
        CompiledPlan::compile(&parsed).err().unwrap().code,
        ErrorCode::InvalidSelector
    );
}

#[test]
fn empty_payload_kinds_have_valid_nonexclusive_schema_and_distinct_receipts() {
    let data_schema = schema("htmlcut.extraction.data").unwrap();
    assert!(data_schema.get("anyOf").is_some());
    let source = document("<p>x</p>");
    let records = compile(json!({"schema":"htmlcut.extraction.plan","version":3,
        "strategy":{"kind":"css","selector":"article"},"selection":{"kind":"all","min":0},
        "projection":{"kind":"records","fields":[{"name":"a","selector":"b"}]}}));
    let values = compile(json!({"schema":"htmlcut.extraction.plan","version":3,
        "strategy":{"kind":"css","selector":"article"},"selection":{"kind":"all","min":0}}));
    let a = source.execute(&records).unwrap();
    let b = source.execute(&values).unwrap();
    assert_eq!(serde_json::to_string(&a.data).unwrap(), "[]");
    assert_eq!(
        a.receipt.data_sha256,
        "4f53cda18c2baa0c0354bb5f9a3ecbe5ed12ab4d8e11ba873c2f11161202b945"
    );
    assert_eq!(a.receipt.data_sha256, b.receipt.data_sha256);
    assert_ne!(a.receipt.data_kind, b.receipt.data_kind);
    assert_ne!(a.receipt.extraction_sha256, b.receipt.extraction_sha256);
}

#[test]
fn targeted_inspection_counts_all_matches_and_labels_sample_abbreviation() {
    let result = document("<p id='a'> A   B </p><p>C</p><p>D</p><p>E</p>")
        .inspect("p", 3)
        .unwrap();
    assert_eq!(result.count, 4);
    assert!(!result.samples_complete);
    assert_eq!(result.samples[0].text, "A B");
    assert_eq!(result.samples[0].attributes, ["id"]);
    assert!(result.samples[0].text_complete && result.samples[0].attributes_complete);
    assert!(document("<p>x</p>").inspect("[", 3).is_err());
    assert!(document("<p>x</p>").inspect("p", 0).is_err());
}

#[test]
fn row_guards_read_original_content_and_identify_the_failed_row() {
    let mut request = plan(json!([{"name":"text","selector":":scope","exclude":[".label"]}]));
    request["guards"] = json!([{"scope":"selected","selector":".label","min":1,"max":1,
        "read":{"kind":"dom_text"},"predicate":{"kind":"exact","value":"OK"}}]);
    let query = compile(request);
    let good = document("<article><b class='label'>OK</b><i>Price</i></article>")
        .execute(&query)
        .unwrap();
    assert_eq!(
        serde_json::to_value(&good.data).unwrap(),
        json!([{"text":"Price"}])
    );
    let error=document("<article><b class='label'>OK</b><i>A</i></article><article><b class='label'>WRONG</b><i>B</i></article>")
        .execute(&query).unwrap_err();
    assert_eq!(error.code, ErrorCode::GuardFailed);
    assert_eq!(error.row_index, Some(2));
    assert_eq!(error.field_index, None);
}

#[test]
fn fields_share_work_rather_than_getting_new_independent_budgets() {
    let source = document(
        "<article id='a'><span>A</span></article><article id='b'><span>B</span></article>",
    );
    let scalar = compile(json!({"schema":"htmlcut.extraction.plan","version":3,
        "strategy":{"kind":"css","selector":"article"},"selection":{"kind":"all"},
        "projection":{"kind":"attribute","name":"id"},"limits":{"max_work":100}}));
    assert!(source.execute(&scalar).is_ok());
    let fields = (0..64)
        .map(|i| {
            json!({"name":format!("field_{i}"),"selector":".absent",
        "selection":{"kind":"optional"}})
        })
        .collect::<Vec<_>>();
    let mut request = plan(json!(fields));
    request["limits"] = json!({"max_work":100});
    let error = source.execute(&compile(request)).unwrap_err();
    assert_eq!(error.code, ErrorCode::ResourceLimit);
    assert_eq!(error.row_index, Some(1));
    assert!(error.field_index.is_some());
}

#[test]
fn nth_and_nonempty_arrays_preserve_complete_counts_and_typed_accessors() {
    let source = document("<article><b>A</b><b>B</b><b>C</b></article>");
    let query = compile(plan(json!([
        {"name":"second","selector":"b","selection":{"kind":"nth","index":2}},
        {"name":"all","selector":"b","selection":{"kind":"all","min":3,"max":3}}
    ])));
    let result = source.execute(&query).unwrap();
    assert!(result.data.as_values().is_none());
    assert_eq!(
        result.data.as_records().unwrap()[0]["second"],
        FieldValue::Text("B".into())
    );
    assert_eq!(
        result.data.as_records().unwrap()[0]["all"],
        FieldValue::Many(vec!["A".into(), "B".into(), "C".into()])
    );
    assert_eq!(result.receipt.fields[0].candidate_count, 3);
    assert_eq!(result.receipt.fields[0].projected_count, 1);
    assert_eq!(result.receipt.fields[1].projected_count, 3);
    let flat = source
        .execute(&CompiledPlan::compile(&ExtractionPlan::css("article").unwrap()).unwrap())
        .unwrap();
    assert!(flat.data.as_records().is_none());
    assert_eq!(flat.data.as_values().unwrap(), ["ABC"]);
}

#[test]
fn record_configuration_rejects_invalid_cardinality_names_and_aggregate_exclusions() {
    for selection in [
        json!({"kind":"nth","index":0}),
        json!({"kind":"nth","index":100001}),
        json!({"kind":"all","min":2,"max":1}),
        json!({"kind":"all","max":10001}),
    ] {
        let request = plan(json!([{"name":"text","selector":"b","selection":selection}]));
        assert!(ExtractionPlan::from_json(&serde_json::to_vec(&request).unwrap()).is_err());
    }
    for name in ["", "2bad", "bad-", &"a".repeat(65)] {
        assert!(
            ExtractionPlan::from_json(
                &serde_json::to_vec(&plan(json!([{"name":name,"selector":"b"}]))).unwrap()
            )
            .is_err()
        );
    }
    for extra in [
        json!({"exclude":["b"]}),
        json!({"transforms":[{"kind":"normalize_whitespace"}]}),
    ] {
        let mut request = plan(json!([{"name":"text","selector":"b"}]));
        for (key, value) in extra.as_object().unwrap() {
            request[key] = value.clone();
        }
        assert!(ExtractionPlan::from_json(&serde_json::to_vec(&request).unwrap()).is_err());
    }
    let fields = (0..65)
        .map(|i| json!({"name":format!("f_{i}"),"selector":"b"}))
        .collect::<Vec<_>>();
    assert!(ExtractionPlan::from_json(&serde_json::to_vec(&plan(json!(fields))).unwrap()).is_err());
    let exclusions = (0..17).map(|i| format!(".omit_{i}")).collect::<Vec<_>>();
    let request = plan(
        json!([{"name":"a","selector":"b","exclude":exclusions},{"name":"b","selector":"i","exclude":exclusions}]),
    );
    assert!(ExtractionPlan::from_json(&serde_json::to_vec(&request).unwrap()).is_err());
    let request = plan(json!([{"name":"a","selector":"b","exclude":["["]}]));
    let parsed = ExtractionPlan::from_json(&serde_json::to_vec(&request).unwrap()).unwrap();
    assert_eq!(
        CompiledPlan::compile(&parsed).err().unwrap().code,
        ErrorCode::InvalidSelector
    );
}

#[test]
fn cell_limits_reject_zero_and_above_supported_ceiling() {
    for cells in [0, 1000001] {
        let mut request = plan(json!([{"name":"a","selector":"b"}]));
        request["limits"] = json!({"max_cells":cells});
        assert!(ExtractionPlan::from_json(&serde_json::to_vec(&request).unwrap()).is_err());
        let limits = ExecutionLimits {
            max_cells: cells,
            ..Default::default()
        };
        assert_eq!(limits.validate().unwrap_err().code, ErrorCode::InvalidLimit);
    }
}

#[test]
fn record_field_transform_lists_cannot_exceed_the_closed_vocabulary() {
    let request = plan(
        json!([{"name":"text","selector":"p","transforms":[{"kind":"normalize_whitespace"},{"kind":"resolve_urls"},{"kind":"normalize_whitespace"}]}]),
    );
    assert_eq!(
        ExtractionPlan::from_json(&serde_json::to_vec(&request).unwrap())
            .unwrap_err()
            .code,
        ErrorCode::InvalidPlan
    );
}

#[test]
fn field_payload_bytes_share_one_aggregate_allowance_at_the_exact_boundary() {
    let source = document("<article><b>AB</b><i>CD</i></article>");
    for (maximum, accepted) in [(3, false), (4, true), (5, true)] {
        let mut request = plan(json!([{"name":"a","selector":"b"},{"name":"b","selector":"i"}]));
        request["limits"] = json!({"max_total_value_bytes":maximum});
        let result = source.execute(&compile(request));
        if accepted {
            assert_eq!(
                serde_json::to_value(result.unwrap().data).unwrap(),
                json!([{"a":"AB","b":"CD"}])
            );
        } else {
            let error = result.unwrap_err();
            assert_eq!(error.code, ErrorCode::ResourceLimit);
            assert_eq!((error.row_index, error.field_index), (Some(1), Some(2)));
        }
    }
}

#[test]
fn record_cells_and_scalar_schema_role_have_exact_declared_contracts() {
    let limits = ExecutionLimits {
        max_cells: 1_000_000,
        ..Default::default()
    };
    limits.validate().unwrap();
    assert_eq!(
        <ExtractionData as schemars::JsonSchema>::schema_name(),
        "ExtractionData"
    );
    let schema = schemars::schema_for!(ExtractionData);
    assert_eq!(schema.as_value()["title"], "ExtractionData");
    assert_eq!(MAX_DATA_BYTES, 67_108_864);
    assert_eq!(MAX_RECEIPT_BYTES, 4_194_304);
}

#[test]
fn document_and_scalar_guard_failures_do_not_claim_a_record_row() {
    for records in [false, true] {
        let mut request = plan(json!([{"name":"text","selector":"b"}]));
        if !records {
            request["projection"] = json!({"kind":"dom_text"});
        }
        request["guards"] = json!([{"scope":"document","selector":"#label","read":{"kind":"dom_text"},"predicate":{"kind":"exact","value":"Expected"}}]);
        let error = document("<p id='label'>Wrong</p><article><b>Text</b></article>")
            .execute(&compile(request))
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::GuardFailed);
        assert_eq!(error.row_index, None);
    }
    let mut request = plan(json!([{"name":"text","selector":"b"}]));
    request["projection"] = json!({"kind":"dom_text"});
    request["guards"] = json!([{"scope":"selected","selector":"b","read":{"kind":"dom_text"},"predicate":{"kind":"exact","value":"Expected"}}]);
    assert_eq!(
        document("<article><b>Wrong</b></article>")
            .execute(&compile(request))
            .unwrap_err()
            .row_index,
        None
    );
}

#[test]
fn preparation_policy_getter_preserves_the_actual_nondefault_document_configuration() {
    let limits = PreparationLimits {
        max_source_bytes: 1024,
        max_depth: 64,
        ..Default::default()
    };
    let source = PreparedDocument::new(
        SourceSnapshot::new("<p>Text</p>", Default::default()).unwrap(),
        limits.clone(),
    )
    .unwrap();
    assert_eq!(source.preparation_limits(), &limits);
    assert_eq!(
        source
            .execute(&CompiledPlan::compile(&ExtractionPlan::css("p").unwrap()).unwrap())
            .unwrap()
            .data
            .as_values()
            .unwrap(),
        ["Text"]
    );
}

#[test]
fn exactly_thirty_two_exclusions_and_guards_are_valid_contracts() {
    let exclusions = (0..16).map(|i| format!(".omit_{i}")).collect::<Vec<_>>();
    let request = plan(
        json!([{"name":"a","selector":"b","exclude":exclusions},{"name":"b","selector":"i","exclude":exclusions}]),
    );
    let source = document("<p id='label'>Expected</p><article><b>AB</b><i>CD</i></article>");
    assert_eq!(
        serde_json::to_value(source.execute(&compile(request)).unwrap().data).unwrap(),
        json!([{"a":"AB","b":"CD"}])
    );
    let mut flat = ExtractionPlan::css("b").unwrap();
    flat.exclude = (0..32).map(|i| format!(".omit_{i}")).collect();
    flat.guards = vec![
        Guard {
            scope: GuardScope::Document,
            selector: "#label".into(),
            min: 1,
            max: Some(1),
            read: GuardRead::DomText {},
            predicate: Some(Predicate::Exact {
                value: "Expected".into()
            })
        };
        32
    ];
    assert_eq!(
        source
            .execute(&CompiledPlan::compile(&flat).unwrap())
            .unwrap()
            .data
            .as_values()
            .unwrap(),
        ["AB"]
    );
}

#[test]
fn regex_program_and_dfa_allowances_are_divided_across_every_compiled_regex() {
    for (count, repetitions, accepted) in [(1, 120_000, true), (32, 6_000, false)] {
        let pattern = format!("a{{{repetitions}}}|x");
        let mut request = ExtractionPlan::css("p").unwrap();
        request.guards = vec![
            Guard {
                scope: GuardScope::Document,
                selector: "p".into(),
                min: 1,
                max: Some(1),
                read: GuardRead::DomText {},
                predicate: Some(Predicate::Regex {
                    pattern,
                    flags: String::new()
                }),
            };
            count
        ];
        let compiled = CompiledPlan::compile(&request);
        if accepted {
            assert_eq!(
                document("<p>x</p>")
                    .execute(&compiled.unwrap())
                    .unwrap()
                    .data
                    .as_values()
                    .unwrap(),
                ["x"]
            );
        } else {
            assert_eq!(compiled.err().unwrap().code, ErrorCode::ResourceLimit);
        }
    }
}
