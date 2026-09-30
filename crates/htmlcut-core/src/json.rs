//! Closed JSON parsing before any duplicate-erasing map conversion.

use std::cell::Cell;
use std::fmt;

use serde::de::{DeserializeSeed, Error, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::{Map, Number, Value};

use crate::limits::{MAX_JSON_DEPTH, MAX_PLAN_BYTES};

/// Reads one size-bounded JSON document, rejecting nested duplicate keys before map creation.
/// Adapter-owned documents add their own closed fields and semantic validation afterward.
pub fn parse_closed_json(bytes: &[u8]) -> Result<Value, crate::ExtractionError> {
    if bytes.len() > MAX_PLAN_BYTES {
        return Err(crate::ExtractionError::limit("json"));
    }
    serde_json::from_slice::<ClosedValue>(bytes)
        .map(|value| value.0)
        .map_err(|error| {
            if error.to_string().starts_with("Closed JSON exceeds") {
                return crate::ExtractionError::limit("json");
            }
            crate::ExtractionError::new(
                crate::ErrorCode::InvalidJson,
                "json",
                "JSON is invalid, too deeply nested, or contains duplicate object keys.",
            )
        })
}

pub(crate) struct ClosedValue(pub Value);

impl<'de> Deserialize<'de> for ClosedValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let budget = Cell::new(MAX_PLAN_BYTES);
        Seed {
            depth: 0,
            budget: &budget,
        }
        .deserialize(deserializer)
        .map(Self)
    }
}

struct Seed<'a> {
    depth: usize,
    budget: &'a Cell<usize>,
}

impl Seed<'_> {
    fn charge<E: Error>(&self, count: usize) -> Result<(), E> {
        let remaining = self
            .budget
            .get()
            .checked_sub(count)
            .ok_or_else(|| E::custom("Closed JSON exceeds its size budget."))?;
        self.budget.set(remaining);
        Ok(())
    }
    fn child(&self) -> Seed<'_> {
        Seed {
            depth: self.depth + 1,
            budget: self.budget,
        }
    }
}

impl<'de> DeserializeSeed<'de> for Seed<'_> {
    type Value = Value;
    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Value, D::Error> {
        if self.depth > MAX_JSON_DEPTH {
            return Err(D::Error::custom("Closed JSON exceeds its nesting budget."));
        }
        self.charge::<D::Error>(1)?;
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for Seed<'_> {
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
        self.charge::<E>(value.len())?;
        Ok(Value::String(value.into()))
    }
    fn visit_string<E: Error>(self, value: String) -> Result<Value, E> {
        self.charge::<E>(value.len())?;
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
            self.charge::<A::Error>(key.len())?;
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
#[path = "tests/json_deserializer.rs"]
mod deserializer_tests;
