//! Validation of the serialized selector-parse detail object.

use serde_json::Value;
use std::num::NonZeroU64;

use super::{SelectorParseDetail, SelectorParseErrorClass};

/// Closed reasons a serialized `selector_parse` object is rejected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SelectorParseDetailsViolation {
    Missing,
    Malformed,
    NonObject,
    ZeroPosition,
    UnknownClass,
}

/// Validates and normalizes the closed public `selector_parse` detail object.
pub(crate) fn validate_selector_parse_details(
    value: &Value,
) -> Result<SelectorParseDetail, SelectorParseDetailsViolation> {
    let details = value
        .as_object()
        .ok_or(SelectorParseDetailsViolation::Malformed)?;
    let selector_parse = details
        .get("selector_parse")
        .ok_or(SelectorParseDetailsViolation::Missing)?;
    let selector_parse = selector_parse
        .as_object()
        .ok_or(SelectorParseDetailsViolation::NonObject)?;

    const REQUIRED_FIELDS: [&str; 3] = ["line", "column_utf16", "parse_error_class"];
    if selector_parse.len() != REQUIRED_FIELDS.len()
        || REQUIRED_FIELDS
            .iter()
            .any(|field| !selector_parse.contains_key(*field))
    {
        return Err(SelectorParseDetailsViolation::Malformed);
    }

    let line = positive_position(selector_parse.get("line"))?;
    let column_utf16 = positive_position(selector_parse.get("column_utf16"))?;
    let class_name = selector_parse
        .get("parse_error_class")
        .and_then(Value::as_str)
        .ok_or(SelectorParseDetailsViolation::Malformed)?;
    let parse_error_class = SelectorParseErrorClass::parse(class_name)
        .ok_or(SelectorParseDetailsViolation::UnknownClass)?;

    Ok(SelectorParseDetail {
        line: NonZeroU64::new(line).expect("positive selector parse line"),
        column_utf16: NonZeroU64::new(column_utf16).expect("positive selector parse column"),
        parse_error_class,
    })
}

fn positive_position(value: Option<&Value>) -> Result<u64, SelectorParseDetailsViolation> {
    match value.and_then(Value::as_u64) {
        Some(0) => Err(SelectorParseDetailsViolation::ZeroPosition),
        Some(value) => Ok(value),
        None => Err(SelectorParseDetailsViolation::Malformed),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn detail_validation_rejects_zero_and_non_integer_positions() {
        for value in [
            json!({"selector_parse":{"line":0,"column_utf16":1,"parse_error_class":"unexpected_token"}}),
            json!({"selector_parse":{"line":1,"column_utf16":0,"parse_error_class":"unexpected_token"}}),
            json!({"selector_parse":{"line":"one","column_utf16":1,"parse_error_class":"unexpected_token"}}),
        ] {
            assert!(validate_selector_parse_details(&value).is_err());
        }
    }
}
