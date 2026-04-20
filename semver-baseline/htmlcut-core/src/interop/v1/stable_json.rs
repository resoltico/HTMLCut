use std::fmt::Write as _;

use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::ContractError;

/// Serializes one value with the frozen `stable_json_v1` profile.
pub fn stable_json_v1<T: Serialize>(value: &T) -> Result<String, ContractError> {
    let value = serde_json::to_value(value)?;
    let mut output = String::new();
    write_stable_json_value(&value, &mut output)?;
    Ok(output)
}

pub(super) fn digest_stable_json<T: Serialize>(value: &T) -> Result<String, ContractError> {
    let stable_json = stable_json_v1(value)?;
    Ok(sha256_hex(stable_json.as_bytes()))
}

pub(super) fn digest_stable_json_omitting_field<T: Serialize>(
    value: &T,
    field: &str,
) -> Result<String, ContractError> {
    let mut value = serde_json::to_value(value)?;
    if let Value::Object(map) = &mut value {
        map.remove(field);
    }

    let mut output = String::new();
    write_stable_json_value(&value, &mut output)?;
    Ok(sha256_hex(output.as_bytes()))
}

#[cfg(test)]
pub(crate) fn digest_stable_json_omitting_field_for_tests<T: Serialize>(
    value: &T,
    field: &str,
) -> Result<String, ContractError> {
    digest_stable_json_omitting_field(value, field)
}

fn write_stable_json_value(value: &Value, output: &mut String) -> Result<(), ContractError> {
    match value {
        Value::Null => output.push_str("null"),
        Value::Bool(value) => output.push_str(if *value { "true" } else { "false" }),
        Value::Number(value) => output.push_str(&value.to_string()),
        Value::String(value) => output.push_str(&serde_json::to_string(value)?),
        Value::Array(values) => {
            output.push('[');
            for (index, value) in values.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                write_stable_json_value(value, output)?;
            }
            output.push(']');
        }
        Value::Object(map) => {
            output.push('{');
            let mut entries = map.iter().collect::<Vec<_>>();
            entries.sort_unstable_by_key(|(key, _)| *key);
            for (index, (key, value)) in entries.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                output.push_str(&serde_json::to_string(key)?);
                output.push(':');
                write_stable_json_value(value, output)?;
            }
            output.push('}');
        }
    }

    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(digest.len() * 2);
    for byte in digest {
        let _ = write!(output, "{byte:02x}");
    }
    output
}
