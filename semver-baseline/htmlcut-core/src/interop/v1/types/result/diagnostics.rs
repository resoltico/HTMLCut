//! Interop diagnostic vocabulary and validation.

use std::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::super::shared::{ContractError, validate_message_bytes};

macro_rules! interop_diagnostic_codes {
    (
        $(
            $(#[$meta:meta])*
            $variant:ident => $code:literal,
        )+
    ) => {
        /// Stable diagnostic-code identifiers published by `htmlcut-v1`.
        #[derive(
            Clone,
            Copy,
            Debug,
            Serialize,
            Deserialize,
            JsonSchema,
            PartialEq,
            Eq,
            PartialOrd,
            Ord,
            Hash,
        )]
        pub enum InteropDiagnosticCode {
            $(
                $(#[$meta])*
                #[serde(rename = $code)]
                $variant,
            )+
        }

        impl InteropDiagnosticCode {
            /// Returns the complete stable diagnostic-code inventory.
            pub const ALL: &'static [Self] = &[
                $(
                    Self::$variant,
                )+
            ];

            /// Returns the stable string form of this diagnostic code.
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(
                        Self::$variant => $code,
                    )+
                }
            }
        }

        impl fmt::Display for InteropDiagnosticCode {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(self.as_str())
            }
        }

        impl PartialEq<&str> for InteropDiagnosticCode {
            fn eq(&self, other: &&str) -> bool {
                self.as_str() == *other
            }
        }

        impl PartialEq<InteropDiagnosticCode> for &str {
            fn eq(&self, other: &InteropDiagnosticCode) -> bool {
                *self == other.as_str()
            }
        }
    };
}

interop_diagnostic_codes! {
    /// The source could not be loaded or decoded.
    SourceLoadFailed => "SOURCE_LOAD_FAILED",
    /// The request spec version is unsupported.
    UnsupportedSpecVersion => "UNSUPPORTED_SPEC_VERSION",
    /// The CSS selector is invalid.
    InvalidSelector => "INVALID_SELECTOR",
    /// The slice pattern or regex flags are invalid.
    InvalidSlicePattern => "INVALID_SLICE_PATTERN",
    /// The requested value type is not valid for the chosen extraction strategy.
    UnsupportedValueType => "UNSUPPORTED_VALUE_TYPE",
    /// No candidates matched the request.
    NoMatch => "NO_MATCH",
    /// Exact-one selection found multiple candidates.
    AmbiguousMatch => "AMBIGUOUS_MATCH",
    /// The requested match index is outside the candidate set.
    MatchIndexOutOfRange => "MATCH_INDEX_OUT_OF_RANGE",
    /// The selected HTML is missing the requested attribute.
    MissingAttribute => "MISSING_ATTRIBUTE",
    /// More than one candidate matched while first-match mode was active.
    MultipleMatches => "MULTIPLE_MATCHES",
    /// URL rewriting depended on an unresolved effective base URL.
    EffectiveBaseUrlUnresolved => "EFFECTIVE_BASE_URL_UNRESOLVED",
    /// Slice selection appears to start or end inside HTML markup.
    SliceSplitsMarkup => "SLICE_SPLITS_MARKUP",
}

impl From<crate::DiagnosticCode> for InteropDiagnosticCode {
    fn from(value: crate::DiagnosticCode) -> Self {
        match value {
            crate::DiagnosticCode::SourceLoadFailed => Self::SourceLoadFailed,
            crate::DiagnosticCode::UnsupportedSpecVersion => Self::UnsupportedSpecVersion,
            crate::DiagnosticCode::InvalidSelector => Self::InvalidSelector,
            crate::DiagnosticCode::InvalidSlicePattern => Self::InvalidSlicePattern,
            crate::DiagnosticCode::UnsupportedValueType => Self::UnsupportedValueType,
            crate::DiagnosticCode::NoMatch => Self::NoMatch,
            crate::DiagnosticCode::AmbiguousMatch => Self::AmbiguousMatch,
            crate::DiagnosticCode::MatchIndexOutOfRange => Self::MatchIndexOutOfRange,
            crate::DiagnosticCode::MissingAttribute => Self::MissingAttribute,
            crate::DiagnosticCode::MultipleMatches => Self::MultipleMatches,
            crate::DiagnosticCode::EffectiveBaseUrlUnresolved => Self::EffectiveBaseUrlUnresolved,
            crate::DiagnosticCode::SliceSplitsMarkup => Self::SliceSplitsMarkup,
        }
    }
}

/// Severity level for published interop diagnostics.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum InteropDiagnosticLevel {
    /// The operation failed.
    Error,
    /// The operation succeeded but with a risk or fallback.
    Warning,
    /// Supplemental informational context.
    Info,
}

impl From<crate::DiagnosticLevel> for InteropDiagnosticLevel {
    fn from(value: crate::DiagnosticLevel) -> Self {
        match value {
            crate::DiagnosticLevel::Error => Self::Error,
            crate::DiagnosticLevel::Warning => Self::Warning,
            crate::DiagnosticLevel::Info => Self::Info,
        }
    }
}

/// Machine-readable diagnostic published by htmlcut-v1.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct InteropDiagnostic {
    /// Severity level for the diagnostic.
    pub level: InteropDiagnosticLevel,
    /// Stable interop diagnostic code.
    pub code: InteropDiagnosticCode,
    /// Human-readable diagnostic message.
    #[schemars(length(max = 1024))]
    pub message: String,
    /// Optional structured details for automation and debugging.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

impl From<&crate::Diagnostic> for InteropDiagnostic {
    fn from(value: &crate::Diagnostic) -> Self {
        Self {
            level: value.level.into(),
            code: value.code.into(),
            message: value.message.clone(),
            details: value.details.clone(),
        }
    }
}

impl InteropDiagnostic {
    pub(super) fn validate_body(&self) -> Result<(), ContractError> {
        validate_message_bytes("diagnostic.message", &self.message)
    }
}
