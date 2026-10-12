// SPDX-License-Identifier: MPL-2.0
use super::*;
use serde_json::json;

#[test]
fn exclusions_are_root_or_field_members_and_never_reading_wrappers() {
    let valid = json!({"version":8,"select":"p","read":"literal"});
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
