// SPDX-License-Identifier: MPL-2.0
//! Repairable failures and their structured facts.
use serde::Serialize;

/// A stable diagnostic code.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    /// The operation request is malformed.
    InvalidRequest,
    /// The contract is malformed or exceeds a grammar bound.
    InvalidContract,
    /// CSS cannot be compiled.
    InvalidSelector,
    /// A regular expression cannot be compiled.
    InvalidPattern,
    /// An encoding label is unknown.
    InvalidEncoding,
    /// The caller's base URL is invalid.
    InvalidBaseUrl,
    /// Selected cardinality is outside the declared range.
    CountMismatch,
    /// A selected value fails its declared shape.
    ValueMismatch,
    /// An expectation does not hold.
    ExpectationFailed,
    /// A required attribute is absent.
    MissingAttribute,
    /// A URL cannot be resolved.
    UnresolvableUrl,
    /// The requested representation cannot preserve the result shape.
    UnrepresentableOutput,
    /// Operator policy was exceeded.
    LimitExceeded,
    /// Input could not be read.
    InputUnreadable,
    /// Input bytes are malformed in their encoding.
    UndecodableInput,
    /// Output could not be published.
    OutputUnwritable,
    /// The caller cancelled the operation.
    Cancelled,
    /// An internal invariant failed.
    Internal,
}

/// Diagnostic classes shared by hosts.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ErrorClass {
    /// Malformed request or contract.
    Request,
    /// A contract or output shape did not hold.
    Refusal,
    /// A configured resource limit was exceeded.
    Limit,
    /// Input or output failed.
    Io,
    /// An internal invariant failed.
    Internal,
    /// The caller cancelled the operation.
    Cancelled,
}
impl ErrorCode {
    /// Classification independent of any frontend's exit conventions.
    pub fn class(self) -> ErrorClass {
        match self {
            Self::InvalidRequest
            | Self::InvalidContract
            | Self::InvalidSelector
            | Self::InvalidPattern
            | Self::InvalidEncoding
            | Self::InvalidBaseUrl => ErrorClass::Request,
            Self::CountMismatch
            | Self::ValueMismatch
            | Self::ExpectationFailed
            | Self::MissingAttribute
            | Self::UnresolvableUrl
            | Self::UnrepresentableOutput => ErrorClass::Refusal,
            Self::LimitExceeded => ErrorClass::Limit,
            Self::InputUnreadable | Self::UndecodableInput | Self::OutputUnwritable => {
                ErrorClass::Io
            }
            Self::Internal => ErrorClass::Internal,
            Self::Cancelled => ErrorClass::Cancelled,
        }
    }
}
/// Accepted cardinality, with an optional inclusive upper bound.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CountRange {
    /// Inclusive lower bound.
    pub min: usize,
    /// Inclusive upper bound; absent means unbounded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<usize>,
}
impl std::fmt::Display for CountRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.max {
            Some(max) if max == self.min => write!(f, "{}", self.min),
            Some(max) => write!(f, "{}..{max}", self.min),
            None => write!(f, "{}..", self.min),
        }
    }
}
/// The reason an expectation failed.
#[derive(Clone, Debug)]
pub enum ExpectationFailure {
    /// Cardinality outside the declared range.
    Count {
        /// Observed matches.
        found: usize,
        /// Accepted range.
        expected: CountRange,
    },
    /// The reading did not satisfy a value check.
    Value {
        /// Preview of the reading.
        value: String,
    },
    /// A required attribute is absent.
    MissingAttribute,
}
/// Resource whose configured limit was exceeded.
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Limit {
    /// Source bytes before decoding.
    InputBytes,
    /// Retained tree nodes.
    Nodes,
    /// Tree depth.
    Depth,
    /// Transformed reading bytes.
    ReadingBytes,
    /// Emitted output bytes.
    OutputBytes,
    /// Cooperative elapsed seconds.
    Time,
}
impl std::fmt::Display for Limit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InputBytes => "input_bytes",
            Self::Nodes => "nodes",
            Self::Depth => "depth",
            Self::ReadingBytes => "reading_bytes",
            Self::OutputBytes => "output_bytes",
            Self::Time => "time",
        })
    }
}
/// A failure carrying only the facts appropriate to its kind.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum Error {
    /// Malformed request.
    InvalidRequest {
        /// Human-readable failure reason.
        reason: String,
    },
    /// Malformed contract.
    InvalidContract {
        /// Contract location.
        path: Option<String>,
        /// Human-readable failure reason.
        reason: String,
    },
    /// Invalid selector.
    InvalidSelector {
        /// Contract location.
        path: Option<String>,
        /// Selector when available.
        select: Option<String>,
        /// Human-readable failure reason.
        reason: String,
    },
    /// Invalid regular expression.
    InvalidPattern {
        /// Contract location.
        path: String,
        /// Human-readable failure reason.
        reason: String,
    },
    /// Unknown encoding label.
    InvalidEncoding {
        /// Caller-supplied encoding label.
        label: String,
    },
    /// Invalid base URL.
    InvalidBaseUrl {
        /// Input value or bounded preview.
        value: String,
        /// Human-readable failure reason.
        reason: String,
    },
    /// Selected cardinality is outside the declared range.
    CountMismatch {
        /// Contract location.
        path: String,
        /// Selector when available.
        select: Option<String>,
        /// One-based row context.
        row: Vec<usize>,
        /// Observed cardinality.
        found: usize,
        /// Accepted cardinality.
        expected: CountRange,
    },
    /// A selected value fails its check.
    ValueMismatch {
        /// Contract location.
        path: String,
        /// One-based row context.
        row: Vec<usize>,
        /// Input value or bounded preview.
        value: String,
    },
    /// An expectation does not hold.
    ExpectationFailed {
        /// Contract location.
        path: String,
        /// Selector when available.
        select: Option<String>,
        /// One-based row context.
        row: Vec<usize>,
        /// Expectation failure facts.
        failure: ExpectationFailure,
    },
    /// A required attribute is absent.
    MissingAttribute {
        /// Contract location.
        path: String,
        /// One-based row context.
        row: Vec<usize>,
    },
    /// A URL cannot be resolved.
    UnresolvableUrl {
        /// Contract location.
        path: String,
        /// One-based row context.
        row: Vec<usize>,
        /// Input value or bounded preview.
        value: String,
    },
    /// The output format cannot preserve the result.
    UnrepresentableOutput {
        /// Contract location.
        path: String,
        /// One-based row context.
        row: Option<usize>,
        /// Human-readable failure reason.
        reason: String,
    },
    /// A resource limit was exceeded.
    LimitExceeded {
        /// Exceeded resource.
        limit: Limit,
        /// Configured bound in the resource unit.
        bound: serde_json::Number,
    },
    /// Input could not be read.
    InputUnreadable {
        /// Input or output path.
        file: String,
        /// Operating system failure detail.
        detail: String,
    },
    /// Input contains malformed bytes.
    UndecodableInput {
        /// Encoding name.
        encoding: &'static str,
        /// Malformed input byte offset.
        offset: usize,
    },
    /// Output could not be written.
    OutputUnwritable {
        /// Input or output path.
        file: String,
        /// Operating system failure detail.
        detail: String,
    },
    /// The caller cancelled the operation.
    Cancelled,
    /// An internal invariant failed.
    Internal {
        /// Human-readable failure reason.
        reason: String,
    },
}
impl std::error::Error for Error {}
impl Error {
    /// Stable diagnostic code.
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::InvalidRequest { .. } => ErrorCode::InvalidRequest,
            Self::InvalidContract { .. } => ErrorCode::InvalidContract,
            Self::InvalidSelector { .. } => ErrorCode::InvalidSelector,
            Self::InvalidPattern { .. } => ErrorCode::InvalidPattern,
            Self::InvalidEncoding { .. } => ErrorCode::InvalidEncoding,
            Self::InvalidBaseUrl { .. } => ErrorCode::InvalidBaseUrl,
            Self::CountMismatch { .. } => ErrorCode::CountMismatch,
            Self::ValueMismatch { .. } => ErrorCode::ValueMismatch,
            Self::ExpectationFailed { .. } => ErrorCode::ExpectationFailed,
            Self::MissingAttribute { .. } => ErrorCode::MissingAttribute,
            Self::UnresolvableUrl { .. } => ErrorCode::UnresolvableUrl,
            Self::UnrepresentableOutput { .. } => ErrorCode::UnrepresentableOutput,
            Self::LimitExceeded { .. } => ErrorCode::LimitExceeded,
            Self::InputUnreadable { .. } => ErrorCode::InputUnreadable,
            Self::UndecodableInput { .. } => ErrorCode::UndecodableInput,
            Self::OutputUnwritable { .. } => ErrorCode::OutputUnwritable,
            Self::Cancelled => ErrorCode::Cancelled,
            Self::Internal { .. } => ErrorCode::Internal,
        }
    }
    /// Compact flat JSON diagnostic followed by LF.
    pub fn to_json_line(&self) -> Vec<u8> {
        let mut bytes = serde_json::to_vec(self).expect("diagnostic facts are JSON serializable");
        bytes.push(b'\n');
        bytes
    }
}
pub(crate) fn preview(value: &str) -> String {
    value.chars().take(120).collect()
}
fn selected(select: Option<&str>) -> String {
    select
        .map(|s| format!("selector {}", serde_json::to_string(s).unwrap()))
        .unwrap_or_else(|| "the match itself".into())
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidRequest { reason } | Self::Internal { reason } => f.write_str(reason),
            Self::InvalidContract { path, reason } | Self::InvalidSelector { path, reason, .. } => {
                if let Some(path) = path {
                    write!(f, "{path}: ")?;
                }
                f.write_str(reason)
            }
            Self::InvalidPattern { path, reason } => write!(f, "{path}: {reason}"),
            Self::InvalidEncoding { label } => write!(f, "Unknown encoding label {label:?}."),
            Self::InvalidBaseUrl { value, reason } => {
                write!(f, "Invalid base URL {value:?}: {reason}.")
            }
            Self::CountMismatch {
                path,
                select,
                found,
                expected,
                ..
            } => write!(
                f,
                "{path}: {} found {found}; expected {expected}.",
                selected(select.as_deref())
            ),
            Self::ValueMismatch { path, value, .. } => write!(
                f,
                "{path}: observed value {:?} does not satisfy its declared shape.",
                preview(value)
            ),
            Self::ExpectationFailed {
                path,
                select,
                failure,
                ..
            } => {
                write!(f, "{path}: {} ", selected(select.as_deref()))?;
                match failure {
                    ExpectationFailure::Count { found, expected } => {
                        write!(f, "found {found}; expected {expected}.")
                    }
                    ExpectationFailure::Value { value } => write!(
                        f,
                        "read {}, which fails the expectation.",
                        serde_json::to_string(&preview(value)).unwrap()
                    ),
                    ExpectationFailure::MissingAttribute => {
                        f.write_str("lacks a required attribute.")
                    }
                }
            }
            Self::MissingAttribute { path, .. } => write!(
                f,
                "{path}: the selected element lacks a required attribute."
            ),
            Self::UnresolvableUrl { path, value, .. } => {
                let reason = match url::Url::parse(value) {
                    Err(url::ParseError::RelativeUrlWithoutBase) => {
                        "relative URL requires a usable document base URL".into()
                    }
                    Err(cause) => format!("malformed URL ({cause})"),
                    Ok(_) => "the document base cannot resolve this URL".into(),
                };
                write!(
                    f,
                    "{path}: URL {:?} cannot be resolved: {reason}.",
                    preview(value)
                )
            }
            Self::UnrepresentableOutput { path, reason, .. } => write!(f, "{path}: {reason}"),
            Self::LimitExceeded { limit, bound } => {
                write!(f, "The {limit} limit of {bound} was exceeded.")
            }
            Self::InputUnreadable { file, detail } | Self::OutputUnwritable { file, detail } => {
                write!(f, "File {file:?}: {detail}.")
            }
            Self::UndecodableInput { encoding, offset } => write!(
                f,
                "Input is not valid {} at byte {offset}.",
                encoding.to_ascii_lowercase()
            ),
            Self::Cancelled => f.write_str("The operation was cancelled."),
        }
    }
}
impl Serialize for Error {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(None)?;
        map.serialize_entry("error", &self.code())?;
        map.serialize_entry("message", &self.to_string())?;
        match self {
            Self::InvalidContract { path, .. } => {
                if let Some(path) = path {
                    map.serialize_entry("path", path)?;
                }
            }
            Self::InvalidSelector { path, select, .. } => {
                if let Some(path) = path {
                    map.serialize_entry("path", path)?;
                }
                if let Some(select) = select.as_ref().filter(|s| s.len() <= 8192) {
                    map.serialize_entry("select", select)?;
                }
            }
            Self::InvalidPattern { path, .. } => map.serialize_entry("path", path)?,
            Self::InvalidEncoding { label } => map.serialize_entry("encoding", label)?,
            Self::InvalidBaseUrl { value, .. } => map.serialize_entry("value", value)?,
            Self::CountMismatch {
                path,
                select,
                row,
                found,
                expected,
            } => {
                map.serialize_entry("path", path)?;
                if let Some(select) = select {
                    map.serialize_entry("select", select)?;
                }
                if !row.is_empty() {
                    map.serialize_entry("row", row)?;
                }
                map.serialize_entry("found", found)?;
                map.serialize_entry("expected", expected)?;
            }
            Self::ValueMismatch { path, row, value }
            | Self::UnresolvableUrl { path, row, value } => {
                map.serialize_entry("path", path)?;
                if !row.is_empty() {
                    map.serialize_entry("row", row)?;
                }
                map.serialize_entry("value", &preview(value))?;
            }
            Self::MissingAttribute { path, row } => {
                map.serialize_entry("path", path)?;
                if !row.is_empty() {
                    map.serialize_entry("row", row)?;
                }
            }
            Self::ExpectationFailed {
                path,
                select,
                row,
                failure,
            } => {
                map.serialize_entry("path", path)?;
                if let Some(select) = select {
                    map.serialize_entry("select", select)?;
                }
                if !row.is_empty() {
                    map.serialize_entry("row", row)?;
                }
                match failure {
                    ExpectationFailure::Count { found, expected } => {
                        map.serialize_entry("found", found)?;
                        map.serialize_entry("expected", expected)?;
                    }
                    ExpectationFailure::Value { value } => {
                        map.serialize_entry("value", &preview(value))?
                    }
                    ExpectationFailure::MissingAttribute => (),
                }
            }
            Self::UnrepresentableOutput { path, row, .. } => {
                map.serialize_entry("path", path)?;
                if let Some(row) = row {
                    map.serialize_entry("row", &[*row])?;
                }
            }
            Self::LimitExceeded { limit, bound } => {
                map.serialize_entry("limit", limit)?;
                map.serialize_entry("bound", bound)?;
            }
            Self::InputUnreadable { file, detail } | Self::OutputUnwritable { file, detail } => {
                map.serialize_entry("file", file)?;
                map.serialize_entry("detail", detail)?;
            }
            Self::UndecodableInput { encoding, offset } => {
                map.serialize_entry("encoding", &encoding.to_ascii_lowercase())?;
                map.serialize_entry("offset", offset)?;
            }
            Self::InvalidRequest { .. } | Self::Internal { .. } | Self::Cancelled => (),
        }
        map.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_variant_has_an_exact_flat_diagnostic_and_class() {
        use ErrorClass::{Cancelled, Internal, Io, Limit as Limited, Refusal, Request};
        let cases = [
            (
                Error::InvalidRequest {
                    reason: "bad\nrequest".into(),
                },
                Request,
                r#"{"error":"invalid_request","message":"bad\nrequest"}"#,
            ),
            (
                Error::InvalidContract {
                    path: Some("$.min".into()),
                    reason: "bad count".into(),
                },
                Request,
                r#"{"error":"invalid_contract","message":"$.min: bad count","path":"$.min"}"#,
            ),
            (
                Error::InvalidSelector {
                    path: Some("$.select".into()),
                    select: Some("[".into()),
                    reason: "bad CSS".into(),
                },
                Request,
                r#"{"error":"invalid_selector","message":"$.select: bad CSS","path":"$.select","select":"["}"#,
            ),
            (
                Error::InvalidPattern {
                    path: "$.pattern".into(),
                    reason: "bad regex".into(),
                },
                Request,
                r#"{"error":"invalid_pattern","message":"$.pattern: bad regex","path":"$.pattern"}"#,
            ),
            (
                Error::InvalidEncoding {
                    label: "unknown".into(),
                },
                Request,
                r#"{"error":"invalid_encoding","message":"Unknown encoding label \"unknown\".","encoding":"unknown"}"#,
            ),
            (
                Error::InvalidBaseUrl {
                    value: "[".into(),
                    reason: "invalid".into(),
                },
                Request,
                r#"{"error":"invalid_base_url","message":"Invalid base URL \"[\": invalid.","value":"["}"#,
            ),
            (
                Error::CountMismatch {
                    path: "$".into(),
                    select: None,
                    row: vec![],
                    found: 0,
                    expected: CountRange { min: 1, max: None },
                },
                Refusal,
                r#"{"error":"count_mismatch","message":"$: the match itself found 0; expected 1...","path":"$","found":0,"expected":{"min":1}}"#,
            ),
            (
                Error::ValueMismatch {
                    path: "$".into(),
                    row: vec![2],
                    value: "x".into(),
                },
                Refusal,
                r#"{"error":"value_mismatch","message":"$: observed value \"x\" does not satisfy its declared shape.","path":"$","row":[2],"value":"x"}"#,
            ),
            (
                Error::ExpectationFailed {
                    path: "$.expect[0]".into(),
                    select: Some("p".into()),
                    row: vec![],
                    failure: ExpectationFailure::Count {
                        found: 2,
                        expected: CountRange {
                            min: 0,
                            max: Some(1),
                        },
                    },
                },
                Refusal,
                r#"{"error":"expectation_failed","message":"$.expect[0]: selector \"p\" found 2; expected 0..1.","path":"$.expect[0]","select":"p","found":2,"expected":{"min":0,"max":1}}"#,
            ),
            (
                Error::ExpectationFailed {
                    path: "$".into(),
                    select: None,
                    row: vec![1],
                    failure: ExpectationFailure::Value { value: "x".into() },
                },
                Refusal,
                r#"{"error":"expectation_failed","message":"$: the match itself read \"x\", which fails the expectation.","path":"$","row":[1],"value":"x"}"#,
            ),
            (
                Error::ExpectationFailed {
                    path: "$".into(),
                    select: None,
                    row: vec![],
                    failure: ExpectationFailure::MissingAttribute,
                },
                Refusal,
                r#"{"error":"expectation_failed","message":"$: the match itself lacks a required attribute.","path":"$"}"#,
            ),
            (
                Error::MissingAttribute {
                    path: "$".into(),
                    row: vec![],
                },
                Refusal,
                r#"{"error":"missing_attribute","message":"$: the selected element lacks a required attribute.","path":"$"}"#,
            ),
            (
                Error::UnresolvableUrl {
                    path: "$".into(),
                    row: vec![],
                    value: "x".into(),
                },
                Refusal,
                r#"{"error":"unresolvable_url","message":"$: URL \"x\" cannot be resolved: relative URL requires a usable document base URL.","path":"$","value":"x"}"#,
            ),
            (
                Error::UnrepresentableOutput {
                    path: "$".into(),
                    row: Some(2),
                    reason: "LF in item".into(),
                },
                Refusal,
                r#"{"error":"unrepresentable_output","message":"$: LF in item","path":"$","row":[2]}"#,
            ),
            (
                Error::LimitExceeded {
                    limit: Limit::OutputBytes,
                    bound: 7.into(),
                },
                Limited,
                r#"{"error":"limit_exceeded","message":"The output_bytes limit of 7 was exceeded.","limit":"output_bytes","bound":7}"#,
            ),
            (
                Error::InputUnreadable {
                    file: "a".into(),
                    detail: "absent".into(),
                },
                Io,
                r#"{"error":"input_unreadable","message":"File \"a\": absent.","file":"a","detail":"absent"}"#,
            ),
            (
                Error::UndecodableInput {
                    encoding: "UTF-8",
                    offset: 3,
                },
                Io,
                r#"{"error":"undecodable_input","message":"Input is not valid utf-8 at byte 3.","encoding":"utf-8","offset":3}"#,
            ),
            (
                Error::OutputUnwritable {
                    file: "a".into(),
                    detail: "denied".into(),
                },
                Io,
                r#"{"error":"output_unwritable","message":"File \"a\": denied.","file":"a","detail":"denied"}"#,
            ),
            (
                Error::Cancelled,
                Cancelled,
                r#"{"error":"cancelled","message":"The operation was cancelled."}"#,
            ),
            (
                Error::Internal {
                    reason: "bug".into(),
                },
                Internal,
                r#"{"error":"internal","message":"bug"}"#,
            ),
        ];
        for (error, class, expected) in cases {
            assert_eq!(error.code().class(), class);
            assert_eq!(error.to_json_line(), format!("{expected}\n").as_bytes());
            let facts = serde_json::to_value(&error).unwrap();
            if let Some(path) = facts.get("path").and_then(serde_json::Value::as_str) {
                assert!(error.to_string().contains(path));
            }
        }
    }
    #[test]
    fn previews_count_characters_and_oversized_selectors_are_omitted() {
        let error = Error::ValueMismatch {
            path: "$".into(),
            row: vec![],
            value: "🦀".repeat(121),
        };
        let facts = serde_json::to_value(&error).unwrap();
        assert_eq!(facts["value"], "🦀".repeat(120));
        assert!(!error.to_string().contains(&"🦀".repeat(121)));
        let error = Error::InvalidSelector {
            path: None,
            select: Some("x".repeat(8193)),
            reason: "too long".into(),
        };
        assert!(serde_json::to_value(error).unwrap().get("select").is_none());
    }
}
