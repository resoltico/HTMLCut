use super::*;

#[test]
fn t26_t28_normalized_defaults_metadata_array_order_and_navigation() {
    let plan = ExtractionPlan::css("#amount").unwrap();
    let minimal = r##"{"version":1,"strategy":{"selector":"#amount","kind":"css"},"schema":"htmlcut.extraction.plan"}"##;
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
    assert_eq!(original.values, navigation.values);
    assert_ne!(original.source_sha256, navigation.source_sha256);
    assert_ne!(original.extraction_sha256, navigation.extraction_sha256);
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
    assert_eq!(original.values, with_base.values);
    assert_ne!(original.extraction_sha256, with_base.extraction_sha256);
    assert!(!canonical_json(&with_base).unwrap().contains("sentinel"));
    let mut reordered = plan;
    reordered.projection = Projection::DocumentText;
    reordered.transforms = vec![Transform::NormalizeWhitespace, Transform::ResolveUrls];
    let first = CompiledPlan::compile(&reordered).unwrap();
    reordered.transforms.reverse();
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
        source.execute(&compiled).unwrap().extraction_sha256,
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
    let minimal = serde_json::json!({"schema":"htmlcut.extraction.plan","version":1,"strategy":{"kind":"css","selector":"p"}});
    assert!(schemas.validate(&minimal, index).is_ok());
    assert!(ExtractionPlan::from_json(&serde_json::to_vec(&minimal).unwrap()).is_ok());
    for invalid in [
        serde_json::json!({"schema":"htmlcut.extraction.plan","version":2,"strategy":{"kind":"css","selector":"p"}}),
        serde_json::json!({"schema":"htmlcut.extraction.plan","version":1,"strategy":{"kind":"css","selector":"p"},"unknown":1}),
        serde_json::json!({"schema":"htmlcut.plan","version":1,"strategy":{"kind":"css","selector":"p"}}),
    ] {
        assert!(schemas.validate(&invalid, index).is_err(), "{invalid}");
        assert!(ExtractionPlan::from_json(&serde_json::to_vec(&invalid).unwrap()).is_err());
    }
}
