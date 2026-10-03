//! Bare requested data; empty arrays retain their kind in typed execution and receipts.

use std::{borrow::Cow, collections::BTreeMap};

use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Serialize};

/// One record field: a required string, explicitly absent node, or all-valued array.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum FieldValue {
    /// One projected node, including a present empty string.
    Text(String),
    /// An optional selector matched zero nodes.
    Absent,
    /// An all-valued selector; may be empty only when explicitly allowed.
    Many(Vec<String>),
}

/// Requested data in source order, without an evidence envelope.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum ExtractionData {
    /// Flat strings in document/source order.
    Values(Vec<String>),
    /// Rows in document order, with deterministic object-key serialization.
    Records(Vec<BTreeMap<String, FieldValue>>),
}

impl ExtractionData {
    /// Borrows flat values when this execution selected the flat representation.
    pub fn as_values(&self) -> Option<&[String]> {
        match self {
            Self::Values(values) => Some(values),
            Self::Records(_) => None,
        }
    }
    /// Borrows rows when this execution selected the records representation.
    pub fn as_records(&self) -> Option<&[BTreeMap<String, FieldValue>]> {
        match self {
            Self::Records(rows) => Some(rows),
            Self::Values(_) => None,
        }
    }
}

impl JsonSchema for ExtractionData {
    fn schema_name() -> Cow<'static, str> {
        "ExtractionData".into()
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        // Both alternatives accept []; an exclusive oneOf would reject valid empty data.
        let values = generator.subschema_for::<Vec<String>>();
        let records = generator.subschema_for::<Vec<BTreeMap<String, FieldValue>>>();
        schemars::json_schema!({"anyOf": [values, records]})
    }
}
