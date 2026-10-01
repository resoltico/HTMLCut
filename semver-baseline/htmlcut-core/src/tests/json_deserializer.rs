use super::*;
use serde::de::value::{BytesDeserializer, Error as ValueError, F64Deserializer};

#[test]
fn generic_deserialization_rejects_non_json_types_and_nonfinite_numbers() {
    let error = ClosedValue::deserialize(BytesDeserializer::<ValueError>::new(b"private data"))
        .err()
        .unwrap();
    assert!(
        error
            .to_string()
            .contains("bounded JSON without duplicate object keys")
    );
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let error = ClosedValue::deserialize(F64Deserializer::<ValueError>::new(value))
            .err()
            .unwrap();
        assert!(error.to_string().contains("Nonfinite JSON number"));
    }
}

#[test]
fn generic_deserializers_cannot_bypass_materialized_byte_and_depth_limits() {
    use serde::de::IntoDeserializer;
    for (length, accepted) in [
        (MAX_PLAN_BYTES - 1, true),
        (MAX_PLAN_BYTES, false),
        (MAX_PLAN_BYTES + 1, false),
    ] {
        let value = serde_json::Value::String("x".repeat(length));
        assert_eq!(
            ClosedValue::deserialize(value.into_deserializer()).is_ok(),
            accepted
        );
    }
    for (depth, accepted) in [
        (MAX_JSON_DEPTH, true),
        (MAX_JSON_DEPTH + 1, false),
        (MAX_JSON_DEPTH + 2, false),
    ] {
        let mut value = serde_json::Value::Null;
        for _ in 0..depth {
            value = serde_json::json!([value]);
        }
        assert_eq!(
            ClosedValue::deserialize(value.into_deserializer()).is_ok(),
            accepted
        );
    }
}
