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
