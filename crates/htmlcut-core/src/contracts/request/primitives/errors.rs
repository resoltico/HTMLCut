use std::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Error returned when a contract value fails eager validation.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum ContractValueError {
    /// A required string was blank.
    #[error("{field} must not be empty")]
    Empty {
        /// Human-readable field label used in the validation message.
        field: &'static str,
    },
    /// A field contains whitespace where the contract forbids it.
    #[error("{field} must not contain whitespace")]
    ContainsWhitespace {
        /// Human-readable field label used in the validation message.
        field: &'static str,
    },
    /// A URL string could not be parsed or violated HTMLCut's URL contract.
    #[error("{field} is invalid: {message}")]
    InvalidUrl {
        /// Human-readable field label used in the validation message.
        field: &'static str,
        /// Specific parse or validation error detail.
        message: String,
    },
    /// A URL used a scheme outside HTMLCut's supported HTTP(S) contract.
    #[error("{field} must use http or https, got {scheme}")]
    UnsupportedUrlScheme {
        /// Human-readable field label used in the validation message.
        field: &'static str,
        /// Unsupported URL scheme.
        scheme: String,
    },
    /// URL userinfo is forbidden because HTMLCut never stores or reports credential-bearing URLs.
    #[error("{field} must not include URL userinfo")]
    UrlUserInfoUnsupported {
        /// Human-readable field label used in the validation message.
        field: &'static str,
    },
    /// Query strings are forbidden for persisted replayable URL artifacts.
    #[error("{field} must not include a query string")]
    UrlQueryUnsupported {
        /// Human-readable field label used in the validation message.
        field: &'static str,
    },
    /// Public display URL artifacts may only carry the explicit redacted query marker.
    #[error("{field} must not include an unredacted query string")]
    UrlUnredactedQueryUnsupported {
        /// Human-readable field label used in the validation message.
        field: &'static str,
    },
    /// Fragments are forbidden for persisted or display-only URL artifacts.
    #[error("{field} must not include a fragment")]
    UrlFragmentUnsupported {
        /// Human-readable field label used in the validation message.
        field: &'static str,
    },
    /// A numeric contract field must be greater than zero.
    #[error("{field} must be greater than zero")]
    NonPositive {
        /// Human-readable field label used in the validation message.
        field: &'static str,
    },
}

macro_rules! non_empty_string_type {
    ($name:ident, $field:literal, $doc:literal) => {
        #[doc = $doc]
        #[derive(
            Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord, Hash,
        )]
        #[serde(try_from = "String")]
        #[schemars(with = "String")]
        pub struct $name(String);

        impl $name {
            /// Validates and stores a non-empty string value.
            pub fn new(value: impl Into<String>) -> Result<Self, ContractValueError> {
                let value = value.into();
                if value.trim().is_empty() {
                    return Err(ContractValueError::Empty { field: $field });
                }

                Ok(Self(value))
            }

            /// Returns the stored string value.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(self.as_str())
            }
        }

        impl TryFrom<String> for $name {
            type Error = ContractValueError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }
    };
}

non_empty_string_type!(
    SelectorQuery,
    "selector",
    "Validated CSS selector text used by selector extraction."
);
non_empty_string_type!(
    SliceBoundary,
    "slice boundary",
    "Validated boundary pattern used by slice extraction."
);
