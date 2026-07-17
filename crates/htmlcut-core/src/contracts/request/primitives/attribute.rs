use std::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::ContractValueError;

/// Validated attribute name used by attribute extraction.
#[derive(
    Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
#[serde(try_from = "String")]
#[schemars(with = "String")]
pub struct AttributeName(String);

impl AttributeName {
    /// Validates and stores one attribute name.
    pub fn new(value: impl Into<String>) -> Result<Self, ContractValueError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(ContractValueError::Empty {
                field: "attribute name",
            });
        }
        if value.chars().any(char::is_whitespace) {
            return Err(ContractValueError::ContainsWhitespace {
                field: "attribute name",
            });
        }

        Ok(Self(value.to_ascii_lowercase()))
    }

    /// Returns the stored string value.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for AttributeName {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for AttributeName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl TryFrom<String> for AttributeName {
    type Error = ContractValueError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
