use schemars::{Schema, json_schema};
use serde_json::json;
use url::Url;

use super::{ContractValueError, DisplayedHttpUrl, HttpUrl, PersistedHttpUrl};

pub(super) fn validate_http_url(
    field: &'static str,
    value: Url,
) -> Result<HttpUrl, ContractValueError> {
    validate_http_url_components(field, &value)?;

    Ok(HttpUrl {
        display: redacted_display_url(&value),
        raw: value,
    })
}

pub(super) fn validate_persisted_http_url(
    field: &'static str,
    value: Url,
) -> Result<PersistedHttpUrl, ContractValueError> {
    let url = validate_http_url(field, value)?;
    if url.raw.query().is_some() {
        return Err(ContractValueError::UrlQueryUnsupported { field });
    }
    if url.raw.fragment().is_some() {
        return Err(ContractValueError::UrlFragmentUnsupported { field });
    }

    Ok(PersistedHttpUrl(url))
}

pub(super) fn validate_displayed_http_url(
    field: &'static str,
    value: Url,
) -> Result<DisplayedHttpUrl, ContractValueError> {
    validate_http_url_components(field, &value)?;
    if value.query().is_some_and(|query| query != "[redacted]") {
        return Err(ContractValueError::UrlUnredactedQueryUnsupported { field });
    }
    if value.fragment().is_some() {
        return Err(ContractValueError::UrlFragmentUnsupported { field });
    }

    Ok(DisplayedHttpUrl(value.to_string()))
}

fn validate_http_url_components(
    field: &'static str,
    value: &Url,
) -> Result<(), ContractValueError> {
    if !matches!(value.scheme(), "http" | "https") {
        return Err(ContractValueError::UnsupportedUrlScheme {
            field,
            scheme: value.scheme().to_owned(),
        });
    }
    if !value.username().is_empty() || value.password().is_some() {
        return Err(ContractValueError::UrlUserInfoUnsupported { field });
    }

    Ok(())
}

fn redacted_display_url(value: &Url) -> String {
    let mut redacted = value.clone();
    if value.query().is_some() {
        redacted.set_query(Some("[redacted]"));
    }
    redacted.set_fragment(None);
    redacted.to_string()
}

pub(super) enum QueryPolicy {
    AllowAny,
    ForbidAny,
    AllowRedactedOnly,
}

pub(super) fn http_url_schema(
    description: &'static str,
    query_policy: QueryPolicy,
    forbid_fragment: bool,
) -> Schema {
    let mut all_of = vec![
        json!({ "pattern": "^https?://" }),
        json!({ "not": { "pattern": "^https?://[^/?#]*@" } }),
    ];
    match query_policy {
        QueryPolicy::AllowAny => {}
        QueryPolicy::ForbidAny => {
            all_of.push(json!({ "not": { "pattern": "\\?" } }));
        }
        QueryPolicy::AllowRedactedOnly => {
            all_of.push(json!({
                "anyOf": [
                    { "not": { "pattern": "\\?" } },
                    { "pattern": "\\?\\[redacted\\]$" }
                ]
            }));
        }
    }
    if forbid_fragment {
        all_of.push(json!({ "not": { "pattern": "#" } }));
    }

    json_schema!({
        "type": "string",
        "format": "uri",
        "description": description,
        "allOf": all_of,
    })
}
