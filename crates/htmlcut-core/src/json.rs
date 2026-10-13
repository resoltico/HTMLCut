// SPDX-License-Identifier: MPL-2.0
//! Duplicate-rejecting bounded JSON before map conversion.
use crate::limits::MAX_JSON_DEPTH;
use serde::Deserializer;
use serde::de::{DeserializeSeed, Error, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number, Value};
use std::fmt;
pub(crate) fn read(bytes: &[u8]) -> Result<Value, serde_json::Error> {
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    Seed { depth: 0 }
        .deserialize(&mut deserializer)
        .and_then(|value| {
            deserializer.end()?;
            Ok(value)
        })
}

struct Seed {
    depth: usize,
}

impl Seed {
    fn child(&self) -> Seed {
        Seed {
            depth: self.depth + 1,
        }
    }
}

impl<'de> DeserializeSeed<'de> for Seed {
    type Value = Value;
    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Value, D::Error> {
        if self.depth > MAX_JSON_DEPTH {
            return Err(D::Error::custom("Closed JSON exceeds its nesting budget."));
        }
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for Seed {
    type Value = Value;
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("bounded JSON without duplicate object keys")
    }
    fn visit_unit<E: Error>(self) -> Result<Value, E> {
        // serde_json::Value's default is JSON null, the exact unit representation.
        Ok(Default::default())
    }
    fn visit_bool<E: Error>(self, value: bool) -> Result<Value, E> {
        Ok(Value::Bool(value))
    }
    fn visit_i64<E: Error>(self, value: i64) -> Result<Value, E> {
        Ok(Value::Number(value.into()))
    }
    fn visit_u64<E: Error>(self, value: u64) -> Result<Value, E> {
        Ok(Value::Number(value.into()))
    }
    fn visit_f64<E: Error>(self, value: f64) -> Result<Value, E> {
        Number::from_f64(value)
            .map(Value::Number)
            .ok_or_else(|| E::custom("Nonfinite JSON number."))
    }
    fn visit_str<E: Error>(self, value: &str) -> Result<Value, E> {
        Ok(Value::String(value.into()))
    }
    fn visit_string<E: Error>(self, value: String) -> Result<Value, E> {
        Ok(Value::String(value))
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Value, A::Error> {
        let mut values = Vec::new();
        while let Some(value) = seq.next_element_seed(self.child())? {
            values.push(value);
        }
        Ok(Value::Array(values))
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Value, A::Error> {
        let mut values = Map::new();
        while let Some(key) = map.next_key::<String>()? {
            if values.contains_key(&key) {
                return Err(A::Error::custom("Duplicate JSON object key."));
            }
            let value = map.next_value_seed(self.child())?;
            values.insert(key, value);
        }
        Ok(Value::Object(values))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nesting_and_duplicate_key_boundaries_are_preserved() {
        let nested = format!("{}0{}", "[".repeat(64), "]".repeat(64));
        assert!(read(nested.as_bytes()).is_ok());
        let deeper = format!("[{nested}]");
        assert!(
            read(deeper.as_bytes())
                .unwrap_err()
                .to_string()
                .contains("nesting")
        );
        assert!(
            read(br#"{"x":1,"x":2}"#)
                .unwrap_err()
                .to_string()
                .contains("Duplicate")
        );
        assert!(read(b"1e999").is_err());
    }
}
