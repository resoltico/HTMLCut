// SPDX-License-Identifier: MPL-2.0
use super::*;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::json;
use std::collections::BTreeSet;

fn closed_variants<T: Serialize + DeserializeOwned + schemars::JsonSchema>(values: Vec<T>) {
    let schema = serde_json::to_value(schemars::schema_for!(T)).unwrap();
    fn kinds(value: &serde_json::Value, root: &serde_json::Value, result: &mut BTreeSet<String>) {
        if let Some(name) = value
            .pointer("/properties/kind/const")
            .and_then(serde_json::Value::as_str)
        {
            result.insert(name.into());
        }
        if let Some(reference) = value.get("$ref").and_then(serde_json::Value::as_str) {
            kinds(
                root.pointer(reference.strip_prefix('#').unwrap()).unwrap(),
                root,
                result,
            );
        }
        for key in ["oneOf", "anyOf"] {
            if let Some(items) = value.get(key).and_then(serde_json::Value::as_array) {
                for item in items {
                    kinds(item, root, result);
                }
            }
        }
    }
    let mut declared = BTreeSet::new();
    kinds(&schema, &schema, &mut declared);
    let covered: BTreeSet<_> = values
        .iter()
        .map(|v| {
            serde_json::to_value(v).unwrap()["kind"]
                .as_str()
                .unwrap()
                .to_owned()
        })
        .collect();
    assert_eq!(
        declared, covered,
        "Every declared tagged variant needs a valid and rejection control"
    );
    let location = "https://schemas.htmlcut.invalid/closed-variants.json";
    let mut schemas = boon::Schemas::new();
    let mut compiler = boon::Compiler::new();
    compiler.add_resource(location, schema).unwrap();
    let index = compiler.compile(location, &mut schemas).unwrap();
    for value in values {
        let mut wire = serde_json::to_value(value).unwrap();
        assert!(schemas.validate(&wire, index).is_ok(), "{wire}");
        assert!(serde_json::from_value::<T>(wire.clone()).is_ok());
        wire.as_object_mut()
            .unwrap()
            .insert("unknown_instruction".into(), json!(true));
        assert!(schemas.validate(&wire, index).is_err(), "{wire}");
        assert!(serde_json::from_value::<T>(wire).is_err());
    }
}

#[test]
fn every_tagged_plan_variant_rejects_unknown_members() {
    closed_variants(vec![
        Projection::Records {
            following_siblings: 0,
            fields: vec![RecordField {
                name: "text".into(),
                selector: "p".into(),
                selection: FieldSelection::default(),
                projection: ValueProjection::default(),
                exclude: vec![],
                transforms: vec![],
            }],
        },
        Projection::Value(ValueProjection::DomText {}),
        Projection::Value(ValueProjection::Markdown {}),
        Projection::Value(ValueProjection::InnerHtml {}),
        Projection::Value(ValueProjection::OuterHtml {}),
        Projection::Source {},
        Projection::Value(ValueProjection::Attribute {
            name: "href".into(),
        }),
    ]);
    closed_variants(vec![
        Selection::Single {},
        Selection::All { min: 1, max: None },
        Selection::Nth { index: 1 },
    ]);
    closed_variants(vec![
        GuardRead::DomText {},
        GuardRead::Attribute {
            name: "title".into(),
        },
    ]);
    closed_variants(vec![
        Transform::NormalizeWhitespace {},
        Transform::ResolveUrls {},
    ]);
    closed_variants(vec![
        Strategy::Css {
            selector: "p".into(),
        },
        Strategy::Slice {
            start: Boundary::Literal { value: "A".into() },
            end: Boundary::Literal { value: "B".into() },
            include_start: false,
            include_end: false,
        },
    ]);
    closed_variants(vec![
        Boundary::Literal { value: "A".into() },
        Boundary::Regex {
            pattern: "A".into(),
            flags: String::new(),
        },
    ]);
    closed_variants(vec![
        Predicate::Exact { value: "A".into() },
        Predicate::Regex {
            pattern: "A".into(),
            flags: String::new(),
        },
    ]);
}

#[test]
fn nested_configuration_is_rejected_before_compilation_or_output() {
    let valid = json!({"schema":"htmlcut.extraction.plan","version":crate::SCHEMA_VERSION,"strategy":{"kind":"css","selector":"p"},"projection":{"kind":"dom_text"}});
    assert!(ExtractionPlan::from_json(&serde_json::to_vec(&valid).unwrap()).is_ok());
    let mut invalid = valid.clone();
    invalid["projection"]["exclude"] = json!([".private"]);
    assert!(ExtractionPlan::from_json(&serde_json::to_vec(&invalid).unwrap()).is_err());
    assert!(serde_json::from_value::<ExtractionPlan>(invalid).is_err());
    let mut allowed = valid;
    allowed["exclude"] = json!([".private"]);
    let plan = ExtractionPlan::from_json(&serde_json::to_vec(&allowed).unwrap()).unwrap();
    assert_eq!(
        prepared("<p>A<span class='private'>SECRET</span>B</p>")
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap()
            .data
            .as_values()
            .unwrap(),
        ["AB"]
    );
}
