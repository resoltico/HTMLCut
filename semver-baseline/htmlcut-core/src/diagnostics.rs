use serde_json::{Value, json};

use crate::contracts::{Diagnostic, DiagnosticLevel};

macro_rules! diagnostic_codes {
    (
        $(
            $(#[$meta:meta])*
            $variant:ident => $code:literal,
        )+
    ) => {
        /// Stable diagnostic-code identifiers emitted by `htmlcut-core`.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum DiagnosticCode {
            $(
                $(#[$meta])*
                $variant,
            )+
        }

        /// Error returned when parsing an unknown diagnostic code.
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub struct DiagnosticCodeParseError;

        impl DiagnosticCode {
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

        impl fmt::Display for DiagnosticCode {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(self.as_str())
            }
        }

        impl fmt::Display for DiagnosticCodeParseError {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("unknown HTMLCut diagnostic code")
            }
        }

        impl std::error::Error for DiagnosticCodeParseError {}

        impl std::str::FromStr for DiagnosticCode {
            type Err = DiagnosticCodeParseError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                match value {
                    $(
                        $code => Ok(Self::$variant),
                    )+
                    _ => Err(DiagnosticCodeParseError),
                }
            }
        }
    };
}

use std::fmt;

diagnostic_codes! {
    /// The source could not be loaded or decoded.
    SourceLoadFailed => "SOURCE_LOAD_FAILED",
    /// The request spec version is unsupported.
    UnsupportedSpecVersion => "UNSUPPORTED_SPEC_VERSION",
    /// The request shape is invalid.
    InvalidRequest => "INVALID_REQUEST",
    /// The CSS selector is invalid.
    InvalidSelector => "INVALID_SELECTOR",
    /// The slice pattern or regex flags are invalid.
    InvalidSlicePattern => "INVALID_SLICE_PATTERN",
    /// No candidates matched the request.
    NoMatch => "NO_MATCH",
    /// Exact-one selection found multiple candidates.
    AmbiguousMatch => "AMBIGUOUS_MATCH",
    /// The requested match index is outside the candidate set.
    MatchIndexOutOfRange => "MATCH_INDEX_OUT_OF_RANGE",
    /// The selected HTML is missing the requested attribute.
    MissingAttribute => "MISSING_ATTRIBUTE",
    /// Parsing failed before extraction could complete.
    ParseFailed => "PARSE_FAILED",
    /// More than one candidate matched while first-match mode was active.
    MultipleMatches => "MULTIPLE_MATCHES",
    /// URL rewriting depended on an unresolved effective base URL.
    EffectiveBaseUrlUnresolved => "EFFECTIVE_BASE_URL_UNRESOLVED",
    /// Slice selection appears to start or end inside HTML markup.
    SliceSplitsMarkup => "SLICE_SPLITS_MARKUP",
}

pub(crate) fn has_errors(diagnostics: &[Diagnostic]) -> bool {
    diagnostics
        .iter()
        .any(|diagnostic| diagnostic.level == DiagnosticLevel::Error)
}

pub(crate) fn error_diagnostic(
    code: DiagnosticCode,
    message: impl Into<String>,
    details: Option<Value>,
) -> Diagnostic {
    Diagnostic {
        level: DiagnosticLevel::Error,
        code: code.to_string(),
        message: message.into(),
        details,
    }
}

pub(crate) fn warning_diagnostic(
    code: DiagnosticCode,
    message: impl Into<String>,
    details: Option<Value>,
) -> Diagnostic {
    Diagnostic {
        level: DiagnosticLevel::Warning,
        code: code.to_string(),
        message: message.into(),
        details,
    }
}

/// Reports that an effective base URL could not be determined for a request that depends on it.
pub(crate) fn unresolved_effective_base_diagnostic(
    document_base_href: Option<&str>,
    rewrite_requested: bool,
) -> Diagnostic {
    warning_diagnostic(
        DiagnosticCode::EffectiveBaseUrlUnresolved,
        if rewrite_requested {
            "URL rewriting was requested, but no effective base URL could be resolved. Relative URLs are left unchanged."
        } else {
            "The document declares <base href>, but no effective base URL could be resolved for this input."
        },
        Some(json!({
            "documentBaseHref": document_base_href,
            "rewriteRequested": rewrite_requested,
        })),
    )
}

/// Reports that a slice selection appears to start or end inside HTML markup.
pub(crate) fn slice_splits_markup_diagnostic(
    affected_matches: &[Value],
    first_range_summary: &str,
) -> Diagnostic {
    warning_diagnostic(
        DiagnosticCode::SliceSplitsMarkup,
        format!(
            "Selected slice boundaries appear to cut through HTML markup near {first_range_summary}. Choose stricter boundaries or include the matched tags when the markup should remain intact."
        ),
        Some(json!({
            "affectedMatches": affected_matches,
        })),
    )
}
