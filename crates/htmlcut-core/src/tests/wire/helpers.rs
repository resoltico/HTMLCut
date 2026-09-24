//! Shared construction and tampering helpers for wire-contract scenarios.

use serde_json::Value;

use crate::{
    CORE_REQUEST_SCHEMA_VERSION, EXTRACTION_REQUEST_SCHEMA_NAME, HTMLCUT_JSON_SCHEMA_PROFILE,
    SOURCE_REQUEST_SCHEMA_NAME,
};

pub(super) fn tamper_wire_profile<T>(document: T) -> T
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    let mut value = serde_json::to_value(document).expect("wire document JSON");
    value["wire_profile"] = Value::String("htmlcut-json-schema-invalid".to_owned());
    serde_json::from_value(value).expect("wire document shape")
}

pub(super) fn tamper_wire_identity_field<T>(document: T, field: &str, value: Value) -> T
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    let mut document = serde_json::to_value(document).expect("wire document JSON");
    document[field] = value;
    serde_json::from_value(document).expect("wire document shape")
}

pub(super) fn v2_document(schema_name: &str, schema_version: u32, body: Value) -> Value {
    let mut object = body
        .as_object()
        .cloned()
        .expect("wire document body must be an object");
    object.insert(
        "schema_name".to_owned(),
        Value::String(schema_name.to_owned()),
    );
    object.insert("schema_version".to_owned(), Value::from(schema_version));
    object.insert(
        "wire_profile".to_owned(),
        Value::String(HTMLCUT_JSON_SCHEMA_PROFILE.to_owned()),
    );
    Value::Object(object)
}

pub(super) fn extraction_request_document(body: Value) -> Value {
    let mut body = body
        .as_object()
        .cloned()
        .expect("extraction request body must be an object");
    let source = body
        .remove("source")
        .expect("extraction request must contain source");
    body.insert(
        "source".to_owned(),
        v2_document(
            SOURCE_REQUEST_SCHEMA_NAME,
            CORE_REQUEST_SCHEMA_VERSION,
            source,
        ),
    );
    v2_document(
        EXTRACTION_REQUEST_SCHEMA_NAME,
        CORE_REQUEST_SCHEMA_VERSION,
        Value::Object(body),
    )
}
