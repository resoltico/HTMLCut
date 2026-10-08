// SPDX-License-Identifier: MPL-2.0
//! Bounded normalized query encoding.
use crate::{ErrorCode, ExtractionError};
use serde::Serialize;
/// Serializes canonical JSON with lexicographically ordered object keys and preserved arrays.
pub fn canonical_json(value: &impl Serialize) -> Result<String, ExtractionError> {
    // Downstream consumers may enable serde_json/preserve_order through Cargo feature
    // unification. Explicit sorting keeps ordering independent of that feature.
    serde_json::to_value(value)
        .and_then(|mut value| {
            value.sort_all_objects();
            serde_json::to_string(&value)
        })
        .map_err(|_| {
            ExtractionError::new(
                ErrorCode::InternalInvariant,
                "serialization",
                "The validated contract could not be serialized.",
            )
        })
}

struct Buffer {
    value: Vec<u8>,
    maximum: usize,
}
impl std::io::Write for Buffer {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > self.maximum.saturating_sub(self.value.len()) {
            return Err(std::io::Error::other("Canonical JSON size limit."));
        }
        self.value.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

// Query structs declare a fixed member order and fields use BTreeMap; direct typed
// serialization avoids map feature unification and a second normalization grammar.
pub(crate) fn query_bytes(plan: &crate::ExtractionPlan) -> Result<String, ExtractionError> {
    let mut buffer = Buffer {
        value: Vec::new(),
        maximum: crate::MAX_PLAN_BYTES,
    };
    serde_json::to_writer(&mut buffer, plan).map_err(|_| {
        ExtractionError::resource("plan", "query_bytes", crate::MAX_PLAN_BYTES as u64)
    })?;
    Ok(String::from_utf8(buffer.value).expect("serde_json produces UTF-8"))
}
