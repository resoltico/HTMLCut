// SPDX-License-Identifier: MPL-2.0
//! Repairable failures and their frozen facts.
use serde::Serialize;

/// A stable failure class.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ErrorCode {
    /// The command-line request is malformed.
    InvalidArguments,
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
    /// Operator policy was exceeded.
    LimitExceeded,
    /// Input could not be read.
    InputUnreadable,
    /// Input bytes are malformed in their encoding.
    UndecodableInput,
    /// Output could not be published.
    OutputUnwritable,
    /// An internal invariant failed.
    Internal,
}

impl ErrorCode {
    /// The command-line exit class for this failure.
    pub fn exit_status(self) -> i32 {
        match self {
            Self::InvalidArguments
            | Self::InvalidContract
            | Self::InvalidSelector
            | Self::InvalidPattern
            | Self::InvalidEncoding
            | Self::InvalidBaseUrl => 2,
            Self::CountMismatch
            | Self::ValueMismatch
            | Self::ExpectationFailed
            | Self::MissingAttribute
            | Self::UnresolvableUrl => 3,
            Self::LimitExceeded => 4,
            Self::InputUnreadable | Self::UndecodableInput | Self::OutputUnwritable => 5,
            Self::Internal => 6,
        }
    }
}

/// One failure, with only the facts known at the point of refusal.
#[derive(Clone, Debug, Serialize, thiserror::Error)]
#[error("{message}")]
#[non_exhaustive]
pub struct Error {
    /// Stable failure code.
    pub error: ErrorCode,
    /// Human-readable facts and repair context.
    pub message: String,
    /// Known diagnostic facts, kept off the error's stack representation.
    #[serde(flatten)]
    pub facts: Box<ErrorFacts>,
}

/// Optional facts carried by a refusal.
#[derive(Clone, Debug, Default, Serialize)]
#[non_exhaustive]
pub struct ErrorFacts {
    /// Contract location.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Selector supplied at this location.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub select: Option<String>,
    /// One-based indexes of enclosing all matches.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub row: Option<Vec<usize>>,
    /// Observed cardinality.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub found: Option<usize>,
    /// Accepted cardinality range.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected: Option<String>,
    /// At most 120 characters of the observed value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// Exceeded operator limit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<String>,
    /// Configured bound, in the limit's units.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bound: Option<serde_json::Value>,
    /// WHATWG encoding name, in lower case.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encoding: Option<String>,
    /// Byte offset of the first malformed sequence.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<usize>,
    /// Input or output path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    /// Operating-system failure detail.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl Error {
    /// Constructs a failure without inventing unavailable facts.
    pub fn new(error: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            error,
            message: message.into(),
            facts: Box::default(),
        }
    }

    pub(crate) fn at(mut self, path: &str) -> Self {
        self.path = Some(path.into());
        self
    }
    pub(crate) fn observed(mut self, value: &str) -> Self {
        self.value = Some(value.chars().take(120).collect());
        self
    }
    /// Constructs an operator-policy refusal.
    pub fn limit(name: &str, bound: impl Into<serde_json::Value>) -> Self {
        let bound = bound.into();
        let mut error = Self::new(
            ErrorCode::LimitExceeded,
            format!("The {name} limit of {bound} was exceeded."),
        );
        error.limit = Some(name.into());
        error.bound = Some(bound);
        error
    }
    /// Compact one-line JSON, including the final LF.
    pub fn to_json_line(&self) -> Vec<u8> {
        let mut bytes = serde_json::to_vec(self).expect("Error facts are JSON serializable");
        bytes.push(b'\n');
        bytes
    }
}
impl std::ops::Deref for Error {
    type Target = ErrorFacts;
    fn deref(&self) -> &Self::Target {
        &self.facts
    }
}
impl std::ops::DerefMut for Error {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.facts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_code_has_its_declared_exit_class() {
        for (codes, expected) in [
            (
                vec![
                    ErrorCode::InvalidArguments,
                    ErrorCode::InvalidContract,
                    ErrorCode::InvalidSelector,
                    ErrorCode::InvalidPattern,
                    ErrorCode::InvalidEncoding,
                    ErrorCode::InvalidBaseUrl,
                ],
                2,
            ),
            (
                vec![
                    ErrorCode::CountMismatch,
                    ErrorCode::ValueMismatch,
                    ErrorCode::ExpectationFailed,
                    ErrorCode::MissingAttribute,
                    ErrorCode::UnresolvableUrl,
                ],
                3,
            ),
            (vec![ErrorCode::LimitExceeded], 4),
            (
                vec![
                    ErrorCode::InputUnreadable,
                    ErrorCode::UndecodableInput,
                    ErrorCode::OutputUnwritable,
                ],
                5,
            ),
            (vec![ErrorCode::Internal], 6),
        ] {
            for code in codes {
                assert_eq!(code.exit_status(), expected);
            }
        }
    }
    #[test]
    fn unknown_facts_are_absent_and_messages_stay_on_one_line() {
        assert_eq!(
            Error::new(ErrorCode::InvalidContract, "Bad\ncontract.").to_json_line(),
            b"{\"error\":\"invalid_contract\",\"message\":\"Bad\\ncontract.\"}\n"
        );
    }
    #[test]
    fn previews_count_characters_not_bytes() {
        let e = Error::new(ErrorCode::ValueMismatch, "Wrong value.")
            .at("$.fields.x")
            .observed(&"🦀".repeat(121));
        assert_eq!(e.value.as_deref().unwrap(), "🦀".repeat(120));
    }
}
