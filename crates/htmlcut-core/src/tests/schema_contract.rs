// SPDX-License-Identifier: MPL-2.0
use super::*;

#[test]
fn t05_generated_plan_schema_accepts_defaulted_example() {
    let schema = schema("htmlcut.extraction.plan").unwrap();
    let required = schema["required"].as_array().unwrap();
    assert!(required.contains(&serde_json::json!("select")));
    assert!(!required.contains(&serde_json::json!("limits")));
    assert!(!required.contains(&serde_json::json!("projection")));
    assert!(!required.contains(&serde_json::json!("selection")));
}

#[test]
fn t05_schema_and_runtime_agree_on_closed_role_version_and_defaults() {
    let schema = schema("htmlcut.extraction.plan").unwrap();
    let location = "https://schemas.htmlcut.invalid/extraction-plan/1.json";
    let mut schemas = boon::Schemas::new();
    let mut compiler = boon::Compiler::new();
    compiler.add_resource(location, schema).unwrap();
    let index = compiler.compile(location, &mut schemas).unwrap();
    let minimal = serde_json::json!({"version":7,"select":"p","match":"one","read":"literal"});
    assert!(schemas.validate(&minimal, index).is_ok());
    assert!(ExtractionPlan::from_json(&serde_json::to_vec(&minimal).unwrap()).is_ok());
    for invalid in [
        serde_json::json!({"version":5,"select":"p"}),
        serde_json::json!({"version":7,"select":"p","unknown":1}),
        serde_json::json!({"version":7}),
    ] {
        assert!(schemas.validate(&invalid, index).is_err(), "{invalid}");
        assert!(ExtractionPlan::from_json(&serde_json::to_vec(&invalid).unwrap()).is_err());
    }
}

#[test]
fn t05_every_named_schema_is_retrievable_and_has_its_expected_public_shape() {
    for (name, title, required_property) in [
        ("htmlcut.extraction.plan", "ExtractionPlan", "select"),
        ("htmlcut.extraction.error", "ExtractionError", "code"),
        ("htmlcut.inspection", "InspectionResult", "count"),
        ("htmlcut.survey", "SurveyResult", "groups"),
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
