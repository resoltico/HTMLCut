// SPDX-License-Identifier: MPL-2.0
#![cfg_attr(not(test), cfg_attr(feature = "fuzzing", no_main))]
#[cfg(all(feature = "fuzzing", not(test)))]
use libfuzzer_sys::fuzz_target;
#[cfg(all(feature = "fuzzing", not(test)))]
fn refuses_extra<T: serde::de::DeserializeOwned>(valid: &str, extra: &serde_json::Value) {
    assert!(serde_json::from_str::<T>(valid).is_ok());
    let mut wire: serde_json::Value = serde_json::from_str(valid).unwrap();
    wire.as_object_mut()
        .unwrap()
        .insert("unexpected_field".into(), extra.clone());
    assert!(serde_json::from_value::<T>(wire).is_err());
}
#[cfg(all(feature = "fuzzing", not(test)))]
fuzz_target!(|data: &[u8]| {
    use htmlcut_core::{
        ConfigurationProblem, ConfigurationRole, ExtractionPlan, FailureCause, Guard, Reading,
        RecordField,
    };
    let raw = &data[..data.len().min(4096)];
    let extra = serde_json::from_slice(raw).unwrap_or(serde_json::Value::Null);
    let _ = htmlcut_core::parse_closed_json(raw, htmlcut_core::MAX_PLAN_BYTES);
    let _ = ExtractionPlan::from_json(raw);
    let _ = serde_json::from_slice::<ExtractionPlan>(raw);
    for (wire, path, problem) in [
        (
            serde_json::json!({"select":"p"}),
            "$.version",
            ConfigurationProblem::MissingRequired,
        ),
        (
            serde_json::json!({"version":null,"select":"p"}),
            "$.version",
            ConfigurationProblem::InvalidValue,
        ),
        (
            serde_json::json!({"version":"private","select":"p"}),
            "$.version",
            ConfigurationProblem::InvalidValue,
        ),
        (
            serde_json::json!({"version":5,"select":"p"}),
            "$.version",
            ConfigurationProblem::UnsupportedVersion,
        ),
        (
            serde_json::json!({"version":7}),
            "$.select",
            ConfigurationProblem::MissingRequired,
        ),
    ] {
        let error = ExtractionPlan::from_json(&serde_json::to_vec(&wire).unwrap()).unwrap_err();
        assert_eq!(error.plan_path.as_deref(), Some(path));
        assert_eq!(
            error.cause,
            Some(FailureCause::Configuration {
                role: ConfigurationRole::Plan,
                problem,
            })
        );
    }
    refuses_extra::<ExtractionPlan>(r#"{"version":7,"select":"p"}"#, &extra);
    refuses_extra::<RecordField>(r#"{"select":"p"}"#, &extra);
    refuses_extra::<Guard>(r#"{"select":"p"}"#, &extra);
    for duplicate in [
        br#"{"version":7,"select":"p","select":"p"}"#.as_slice(),
        br#"{"version":7,"select":"p","fields":{"a":{"select":"p"},"a":{"select":"p"}}}"#.as_slice(),
        br#"{"version":7,"select":"p","fields":{"a":{"select":"p","read":"text","read":"literal"}}}"#.as_slice(),
        br#"{"version":7,"select":"p","expect":[{"select":"p","min":0,"min":1}]}"#.as_slice(),
        br#"{"version":7,"select":"p","limits":{"max_work":1,"max_work":2}}"#.as_slice(),
    ] { assert!(ExtractionPlan::from_json(duplicate).is_err()); }
    for read in [
        "text",
        "literal",
        "markdown",
        "resolved-markdown",
        "inner-html",
        "outer-html",
        "attr:href",
        "url:href",
    ] {
        assert!(read.parse::<Reading>().is_ok());
    }
    for member in [
        "read",
        "fields",
        "min",
        "max",
        "index",
        "exclude",
        "following_siblings",
    ] {
        let mut wire = serde_json::json!({"version":7,"select":"p"});
        wire[member] = serde_json::Value::Null;
        assert!(serde_json::from_value::<ExtractionPlan>(wire).is_err());
    }
});
#[cfg(any(test, not(feature = "fuzzing")))]
fn main() {}
