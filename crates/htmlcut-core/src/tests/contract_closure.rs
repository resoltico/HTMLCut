// SPDX-License-Identifier: MPL-2.0
use super::*;
use serde_json::json;

#[test]
fn current_query_fields_and_expectations_reject_unknown_members() {
    let query = json!({"version":6,"select":"p","fields":{"text":{"select":":scope"}},"expect":[{"select":"p"}]});
    ExtractionPlan::from_json(&serde_json::to_vec(&query).unwrap()).unwrap();
    for location in ["root", "field", "expect", "limits"] {
        let mut invalid = query.clone();
        let object = match location {
            "root" => invalid.as_object_mut().unwrap(),
            "field" => invalid["fields"]["text"].as_object_mut().unwrap(),
            "expect" => invalid["expect"][0].as_object_mut().unwrap(),
            _ => {
                invalid["limits"] = json!({});
                invalid["limits"].as_object_mut().unwrap()
            }
        };
        object.insert("unknown_instruction".into(), json!(true));
        assert!(ExtractionPlan::from_json(&serde_json::to_vec(&invalid).unwrap()).is_err());
        assert!(serde_json::from_value::<ExtractionPlan>(invalid).is_err());
    }
    for reading in [
        Reading::Text,
        Reading::Literal,
        Reading::Markdown,
        Reading::ResolvedMarkdown,
        Reading::InnerHtml,
        Reading::OuterHtml,
        Reading::Attribute("href".into()),
        Reading::Url("href".into()),
    ] {
        let wire = serde_json::to_value(&reading).unwrap();
        assert_eq!(
            serde_json::from_value::<Reading>(wire.clone()).unwrap(),
            reading
        );
        assert!(serde_json::from_value::<Reading>(json!({"read":wire})).is_err());
    }
    assert!(serde_json::from_value::<Match>(json!("optional")).is_err());
    assert_eq!(
        serde_json::from_value::<FieldMatch>(json!("optional")).unwrap(),
        FieldMatch::Optional
    );
}
#[test]
fn exclusions_are_root_or_field_members_and_never_reading_wrappers() {
    let valid = json!({"version":6,"select":"p","read":"literal"});
    let mut invalid = valid.clone();
    invalid["projection"] = json!({"exclude":[".private"]});
    assert!(ExtractionPlan::from_json(&serde_json::to_vec(&invalid).unwrap()).is_err());
    assert!(serde_json::from_value::<ExtractionPlan>(invalid).is_err());
    let mut allowed = valid;
    allowed["exclude"] = json!([".private"]);
    let plan = ExtractionPlan::from_json(&serde_json::to_vec(&allowed).unwrap()).unwrap();
    assert_eq!(
        prepared("<p>A<span class='private'>SECRET</span>B</p>")
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap()
            .data()
            .as_values()
            .unwrap(),
        ["AB"]
    );
}
