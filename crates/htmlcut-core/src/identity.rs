// SPDX-License-Identifier: MPL-2.0
//! Domain-separated, length-framed deterministic identities.

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{ErrorCode, ExtractionError};

pub(crate) fn sha256(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

struct JsonBuffer<'a> {
    bytes: Vec<u8>,
    maximum: usize,
    budget: &'a selectors::work_budget::SelectorWorkBudget,
    failure: Option<ExtractionError>,
}
impl std::io::Write for JsonBuffer<'_> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > self.maximum.saturating_sub(self.bytes.len()) {
            if self.failure.is_none() {
                self.failure = Some(ExtractionError::resource(
                    "serialization",
                    "encoded_bytes",
                    self.maximum as u64,
                ));
            }
            return Err(std::io::Error::other("Encoded JSON bound exceeded."));
        }
        let size = self.bytes.len() + bytes.len();
        if let Err(error) = crate::execution::charge(
            self.budget,
            size.div_ceil(64) - self.bytes.len().div_ceil(64),
        ) {
            if self.failure.is_none() {
                self.failure = Some(error);
            }
            return Err(std::io::Error::other("Encoded JSON work exhausted."));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
/// Serializes directly ordered types once, charging and admitting every encoded byte.
pub(crate) fn encoded(
    value: &impl Serialize,
    maximum: usize,
    budget: &selectors::work_budget::SelectorWorkBudget,
) -> Result<Vec<u8>, ExtractionError> {
    let mut writer = JsonBuffer {
        bytes: Vec::new(),
        maximum,
        budget,
        failure: None,
    };
    serde_json::to_writer(&mut writer, value).map_err(|_| {
        writer.failure.take().unwrap_or_else(|| {
            ExtractionError::new(
                ErrorCode::InternalInvariant,
                "serialization",
                "Validated result encoding failed.",
            )
        })
    })?;
    Ok(writer.bytes)
}
pub(crate) fn framed(domain: &str, fields: &[&[u8]]) -> String {
    let mut hash = Sha256::new();
    for value in std::iter::once(domain.as_bytes()).chain(fields.iter().copied()) {
        hash.update((value.len() as u64).to_be_bytes());
        hash.update(value);
    }
    hex(&hash.finalize())
}

/// Charges the actual domain and component frames before hashing the same bounded inputs.
pub(crate) fn budgeted_framed(
    domain: &str,
    fields: &[&[u8]],
    budget: &selectors::work_budget::SelectorWorkBudget,
) -> Result<String, ExtractionError> {
    let bytes = std::iter::once(domain.as_bytes())
        .chain(fields.iter().copied())
        .map(|value| std::mem::size_of::<u64>() + value.len())
        .sum::<usize>();
    crate::execution::charge(budget, bytes.div_ceil(64))?;
    Ok(framed(domain, fields))
}

fn hex(bytes: &[u8]) -> String {
    let digits = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(digits[(byte >> 4) as usize] as char);
        output.push(digits[(byte & 15) as usize] as char);
    }
    output
}

/// Serializes canonical JSON with lexicographically ordered object keys and preserved arrays.
pub fn canonical_json(value: &impl Serialize) -> Result<String, ExtractionError> {
    // Downstream consumers may enable serde_json/preserve_order through Cargo feature
    // unification. Explicit sorting keeps the identity independent of that feature.
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

#[cfg(test)]
#[path = "tests/identity_writer.rs"]
mod writer_tests;

// Query structs declare a fixed member order and fields use BTreeMap; direct typed
// serialization avoids map feature unification and a second identity grammar.
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
