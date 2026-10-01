//! Domain-separated, length-framed deterministic identities.

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{ErrorCode, ExtractionError};

pub(crate) fn sha256(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}

pub(crate) fn framed(domain: &str, fields: &[&[u8]]) -> String {
    let mut hash = Sha256::new();
    for value in std::iter::once(domain.as_bytes()).chain(fields.iter().copied()) {
        hash.update((value.len() as u64).to_be_bytes());
        hash.update(value);
    }
    hex(&hash.finalize())
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

pub(crate) fn canonical_json_bounded(
    value: &impl Serialize,
    maximum: usize,
) -> Result<String, ExtractionError> {
    let mut value = serde_json::to_value(value).map_err(|_| {
        ExtractionError::new(
            ErrorCode::InternalInvariant,
            "serialization",
            "The validated contract could not be serialized.",
        )
    })?;
    value.sort_all_objects();
    let mut buffer = Buffer {
        value: Vec::new(),
        maximum,
    };
    serde_json::to_writer(&mut buffer, &value).map_err(|_| ExtractionError::limit("plan"))?;
    // serde_json writes UTF-8 JSON into this owned buffer, which is never externally mutated.
    Ok(String::from_utf8(buffer.value).expect("serde_json produces UTF-8"))
}

#[cfg(test)]
#[path = "tests/identity_writer.rs"]
mod writer_tests;
