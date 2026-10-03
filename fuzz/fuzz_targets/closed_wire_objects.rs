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
    use htmlcut_core::{ExtractionPlan, GuardRead, Projection, Selection, Transform};
    let raw = &data[..data.len().min(4096)];
    let extra = serde_json::from_slice(raw).unwrap_or(serde_json::Value::Null);
    let _ = htmlcut_core::parse_closed_json(raw, htmlcut_core::MAX_PLAN_BYTES);
    let _ = serde_json::from_slice::<ExtractionPlan>(raw);
    for projection in ["dom_text", "markdown", "inner_html", "outer_html", "source"] {
        refuses_extra::<Projection>(&format!("{{\"kind\":\"{projection}\"}}"), &extra);
    }
    refuses_extra::<Selection>(r#"{"kind":"single"}"#, &extra);
    refuses_extra::<GuardRead>(r#"{"kind":"dom_text"}"#, &extra);
    for transform in ["normalize_whitespace", "resolve_urls"] {
        refuses_extra::<Transform>(&format!("{{\"kind\":\"{transform}\"}}"), &extra);
    }
    for duplicate in [
        br#"{"kind":"single","kind":"single"}"#.as_slice(),
        br#"{"version":2,"version":2}"#.as_slice(),
    ] {
        assert!(htmlcut_core::parse_closed_json(duplicate, htmlcut_core::MAX_PLAN_BYTES).is_err());
    }
    let mut plan: serde_json::Value =
        serde_json::to_value(ExtractionPlan::css("p").unwrap()).unwrap();
    assert!(serde_json::from_value::<ExtractionPlan>(plan.clone()).is_ok());
    plan["projection"]["unexpected_field"] = extra.clone();
    assert!(serde_json::from_value::<ExtractionPlan>(plan).is_err());
});
#[cfg(any(test, not(feature = "fuzzing")))]
fn main() {}
