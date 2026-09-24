//! V2 standalone-document identity envelopes and preflight decoding.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use thiserror::Error;

use crate::ContractValueError;

/// Required identity envelope for every standalone v2 wire document.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct WireEnvelope {
    schema_name: String,
    schema_version: u32,
    wire_profile: String,
}

/// Error returned while preflighting or decoding a standalone v2 wire document.
#[derive(Debug, Error)]
pub enum WireDocumentDecodeError {
    /// The input was not valid JSON, so its identity could not be inspected.
    #[error("wire document is not valid JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),
    /// The document was not authored for this exact v2 schema identity.
    #[error("wire document identity envelope is not current")]
    IncompatibleEnvelope,
}

#[derive(Deserialize)]
pub(super) struct WireEnvelopePreflight {
    #[serde(default)]
    schema_name: Option<String>,
    #[serde(default)]
    schema_version: Option<u32>,
    #[serde(default)]
    wire_profile: Option<String>,
}

/// Validates a standalone document's identity before its body is deserialized.
///
/// This deliberately reads only the identity envelope and ignores every body field. Callers that
/// load persisted JSON must perform this preflight before attempting full typed deserialization.
pub fn preflight_document_json(
    json: &str,
    schema_name: &str,
    schema_version: u32,
) -> Result<(), WireDocumentDecodeError> {
    let envelope: WireEnvelopePreflight = serde_json::from_str(json)?;
    if envelope.schema_name.as_deref() == Some(schema_name)
        && envelope.schema_version == Some(schema_version)
        && envelope.wire_profile.as_deref() == Some(crate::HTMLCUT_JSON_SCHEMA_PROFILE)
    {
        Ok(())
    } else {
        Err(WireDocumentDecodeError::IncompatibleEnvelope)
    }
}

/// Preflights and then fully decodes one standalone v2 wire document.
///
/// The separate preflight keeps an old or cross-schema document from being interpreted under a
/// current contract merely because some of its body fields happen to have compatible shapes.
pub fn decode_document_json<T>(
    json: &str,
    schema_name: &str,
    schema_version: u32,
) -> Result<T, WireDocumentDecodeError>
where
    T: DeserializeOwned,
{
    preflight_document_json(json, schema_name, schema_version)?;
    Ok(serde_json::from_str(json)?)
}

impl WireEnvelope {
    pub(super) fn new(schema_name: &str, schema_version: u32) -> Self {
        Self {
            schema_name: schema_name.to_owned(),
            schema_version,
            wire_profile: crate::HTMLCUT_JSON_SCHEMA_PROFILE.to_owned(),
        }
    }

    pub(super) fn validate(
        &self,
        schema_name: &str,
        schema_version: u32,
    ) -> Result<(), ContractValueError> {
        validate_wire_identity(
            &self.schema_name,
            self.schema_version,
            &self.wire_profile,
            schema_name,
            schema_version,
        )
    }
}

pub(super) fn validate_wire_identity(
    actual_schema_name: &str,
    actual_schema_version: u32,
    actual_wire_profile: &str,
    expected_schema_name: &str,
    expected_schema_version: u32,
) -> Result<(), ContractValueError> {
    if actual_schema_name == expected_schema_name
        && actual_schema_version == expected_schema_version
        && actual_wire_profile == crate::HTMLCUT_JSON_SCHEMA_PROFILE
    {
        Ok(())
    } else {
        Err(ContractValueError::WireEnvelope)
    }
}
