#![cfg_attr(not(test), cfg_attr(feature = "fuzzing", no_main))]
#[cfg(all(feature = "fuzzing", not(test)))]
use libfuzzer_sys::fuzz_target;
#[cfg(all(feature = "fuzzing", not(test)))]
#[path = "../../crates/htmlcut-cli/src/input.rs"]
// Only owning deserializers are exercised; this target never performs acquisition I/O.
#[allow(dead_code)]
mod input;
#[cfg(all(feature = "fuzzing", not(test)))]
#[path = "../../crates/htmlcut-cli/src/input/http/media_type.rs"]
mod media_type;

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
    let _ = htmlcut_core::parse_closed_json(raw);
    let _ = serde_json::from_slice::<ExtractionPlan>(raw);
    let _ = serde_json::from_slice::<input::RunSpec>(raw);
    for projection in [
        "dom_text",
        "document_text",
        "inner_html",
        "outer_html",
        "source",
    ] {
        refuses_extra::<Projection>(&format!("{{\"kind\":\"{projection}\"}}"), &extra);
    }
    refuses_extra::<Selection>(r#"{"kind":"single"}"#, &extra);
    refuses_extra::<GuardRead>(r#"{"kind":"dom_text"}"#, &extra);
    for transform in ["normalize_whitespace", "resolve_urls"] {
        refuses_extra::<Transform>(&format!("{{\"kind\":\"{transform}\"}}"), &extra);
    }
    for source in [
        r#"{"kind":"stdin"}"#,
        r#"{"kind":"file","path":"fixture.html"}"#,
        r#"{"kind":"http","url":"https://fixture.invalid/"}"#,
    ] {
        refuses_extra::<input::SourceSpec>(source, &extra);
    }
    for duplicate in [
        br#"{"kind":"single","kind":"single"}"#.as_slice(),
        br#"{"version":2,"version":2}"#.as_slice(),
    ] {
        assert!(htmlcut_core::parse_closed_json(duplicate).is_err());
    }
    let mut plan: serde_json::Value =
        serde_json::to_value(ExtractionPlan::css("p").unwrap()).unwrap();
    assert!(serde_json::from_value::<ExtractionPlan>(plan.clone()).is_ok());
    plan["projection"]["unexpected_field"] = extra.clone();
    assert!(serde_json::from_value::<ExtractionPlan>(plan).is_err());
    let plan = serde_json::to_value(ExtractionPlan::css("p").unwrap()).unwrap();
    let mut run = serde_json::json!({"schema":"htmlcut.run","version":htmlcut_core::SCHEMA_VERSION,"source":{"kind":"stdin"},"plan":plan});
    assert!(serde_json::from_value::<input::RunSpec>(run.clone()).is_ok());
    run["source"]["unexpected_field"] = extra;
    assert!(serde_json::from_value::<input::RunSpec>(run).is_err());
});
#[cfg(any(test, not(feature = "fuzzing")))]
fn main() {}
