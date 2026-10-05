// SPDX-License-Identifier: MPL-2.0
use super::*;

#[test]
fn t26_t28_normalized_defaults_metadata_array_order_and_navigation() {
    let plan = ExtractionPlan::css("#amount").unwrap();
    let minimal = r##"{"version":5,"strategy":{"selector":"#amount","kind":"css"},"schema":"htmlcut.extraction.plan"}"##;
    let compiled = CompiledPlan::compile(&plan).unwrap();
    assert_eq!(
        CompiledPlan::compile(&ExtractionPlan::from_json(minimal.as_bytes()).unwrap())
            .unwrap()
            .plan_sha256(),
        compiled.plan_sha256()
    );
    let original = prepared("<nav>A</nav><p id='amount'>180</p>")
        .execute(&compiled)
        .unwrap();
    let navigation = prepared("<nav>B</nav><p id='amount'>180</p>")
        .execute(&compiled)
        .unwrap();
    assert_eq!(
        original.data.as_values().unwrap(),
        navigation.data.as_values().unwrap()
    );
    assert_ne!(
        original.receipt.source_sha256,
        navigation.receipt.source_sha256
    );
    assert_ne!(
        original.receipt.extraction_sha256,
        navigation.receipt.extraction_sha256
    );
    let with_base = PreparedDocument::new(
        SourceSnapshot::new(
            "<nav>A</nav><p id='amount'>180</p>",
            SnapshotMetadata {
                base_url: Some("https://example.test/?secret=sentinel".into()),
            },
        )
        .unwrap(),
        PreparationLimits::default(),
    )
    .unwrap()
    .execute(&compiled)
    .unwrap();
    assert_eq!(
        original.data.as_values().unwrap(),
        with_base.data.as_values().unwrap()
    );
    assert_ne!(
        original.receipt.extraction_sha256,
        with_base.receipt.extraction_sha256
    );
    assert!(
        !canonical_json(&with_base.receipt)
            .unwrap()
            .contains("sentinel")
    );
    let mut reordered = plan;
    reordered.exclude = vec![".first".into(), ".second".into()];
    let first = CompiledPlan::compile(&reordered).unwrap();
    reordered.exclude.reverse();
    assert_ne!(
        first.plan_sha256(),
        CompiledPlan::compile(&reordered).unwrap().plan_sha256()
    );
}

#[test]
fn t05_generated_plan_schema_accepts_defaulted_example() {
    let schema = schema("htmlcut.extraction.plan").unwrap();
    let required = schema["required"].as_array().unwrap();
    assert!(required.contains(&serde_json::json!("strategy")));
    assert!(!required.contains(&serde_json::json!("limits")));
    assert!(!required.contains(&serde_json::json!("projection")));
    assert!(!required.contains(&serde_json::json!("selection")));
}

#[test]
fn t26_independent_length_framed_golden_vectors() {
    let vector: serde_json::Value = serde_json::from_str(include_str!(
        "../../tests/fixtures/extraction-contract/identity.json"
    ))
    .unwrap();
    let plan: ExtractionPlan = serde_json::from_value(vector["plan"].clone()).unwrap();
    let compiled = CompiledPlan::compile(&plan).unwrap();
    let source = PreparedDocument::new(
        SourceSnapshot::new(
            vector["source"].as_str().unwrap(),
            serde_json::from_value(vector["metadata"].clone()).unwrap(),
        )
        .unwrap(),
        serde_json::from_value(vector["preparation_limits"].clone()).unwrap(),
    )
    .unwrap();
    assert_eq!(
        source.snapshot().source_sha256(),
        vector["source_sha256"].as_str().unwrap()
    );
    assert_eq!(
        source.prepared_sha256(),
        vector["prepared_sha256"].as_str().unwrap()
    );
    assert_eq!(
        compiled.plan_sha256(),
        vector["plan_sha256"].as_str().unwrap()
    );
    assert_eq!(
        source.execute(&compiled).unwrap().receipt.extraction_sha256,
        vector["extraction_sha256"].as_str().unwrap()
    );
}

#[test]
fn t05_schema_and_runtime_agree_on_closed_role_version_and_defaults() {
    let schema = schema("htmlcut.extraction.plan").unwrap();
    let location = "https://schemas.htmlcut.invalid/extraction-plan/1.json";
    let mut schemas = boon::Schemas::new();
    let mut compiler = boon::Compiler::new();
    compiler.add_resource(location, schema).unwrap();
    let index = compiler.compile(location, &mut schemas).unwrap();
    let minimal = serde_json::json!({"schema":"htmlcut.extraction.plan","version":5,"strategy":{"kind":"css","selector":"p"}});
    assert!(schemas.validate(&minimal, index).is_ok());
    assert!(ExtractionPlan::from_json(&serde_json::to_vec(&minimal).unwrap()).is_ok());
    for invalid in [
        serde_json::json!({"schema":"htmlcut.extraction.plan","version":1,"strategy":{"kind":"css","selector":"p"}}),
        serde_json::json!({"schema":"htmlcut.extraction.plan","version":5,"strategy":{"kind":"css","selector":"p"},"unknown":1}),
        serde_json::json!({"schema":"htmlcut.plan","version":5,"strategy":{"kind":"css","selector":"p"}}),
    ] {
        assert!(schemas.validate(&invalid, index).is_err(), "{invalid}");
        assert!(ExtractionPlan::from_json(&serde_json::to_vec(&invalid).unwrap()).is_err());
    }
}

#[test]
fn t22_serializer_faults_are_typed_and_canonical_escaping_is_bounded() {
    struct Fault;
    impl serde::Serialize for Fault {
        fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
            Err(serde::ser::Error::custom("synthetic serializer fault"))
        }
    }
    assert_eq!(
        canonical_json(&Fault).unwrap_err().code,
        ErrorCode::InternalInvariant
    );
    assert_eq!(
        crate::identity::canonical_json_bounded(&Fault, 128)
            .unwrap_err()
            .code,
        ErrorCode::InternalInvariant
    );
    let data = serde_json::json!({"z":["é",0],"a":{"quoted":"\"\n"}});
    let expected = "{\"a\":{\"quoted\":\"\\\"\\n\"},\"z\":[\"é\",0]}";
    assert_eq!(canonical_json(&data).unwrap(), expected);
    for (maximum, valid) in [
        (expected.len() - 1, false),
        (expected.len(), true),
        (expected.len() + 1, true),
    ] {
        assert_eq!(
            crate::identity::canonical_json_bounded(&data, maximum).is_ok(),
            valid
        );
    }
    assert_eq!(
        schema("htmlcut.unsupported").unwrap_err().code,
        ErrorCode::InvalidSchema
    );
    assert_eq!(ErrorCode::ResourceLimit.exit_class(), 4);
    assert_eq!(ErrorCode::InternalInvariant.exit_class(), 6);
}

#[test]
fn t05_every_named_schema_is_retrievable_and_has_its_expected_public_shape() {
    for (name, title, required_property) in [
        ("htmlcut.extraction.plan", "ExtractionPlan", "strategy"),
        (
            "htmlcut.extraction.receipt",
            "ExecutionReceipt",
            "data_kind",
        ),
        ("htmlcut.extraction.error", "ExtractionError", "code"),
        ("htmlcut.inspection", "InspectionResult", "count"),
    ] {
        let schema = crate::schema(name).unwrap();
        assert_eq!(schema["title"], title);
        assert!(schema["properties"].get(required_property).is_some());
    }
    assert_eq!(
        crate::schema("htmlcut.unknown").unwrap_err().code,
        ErrorCode::InvalidSchema
    );
}
