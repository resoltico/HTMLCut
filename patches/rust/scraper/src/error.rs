//! Custom error types for diagnostics
//! Includes re-exported error types from dependencies

use std::{error::Error, fmt::Display};

use cssparser::{BasicParseErrorKind, ParseErrorKind, SourceLocation};
use selectors::parser::SelectorParseErrorKind;

/// Error type that is returned when calling `Selector::parse`
#[derive(Debug, Clone)]
pub enum SelectorErrorKind {
    /// A `Token` was not expected
    UnexpectedToken,

    /// End-Of-Line was unexpected
    EndOfLine,

    /// `@` rule is invalid
    InvalidAtRule,

    /// The body of an `@` rule is invalid
    InvalidAtRuleBody,

    /// The qualified rule is invalid
    QualRuleInvalid,

    /// Expected a `::` for a pseudoelement
    ExpectedColonOnPseudoElement,

    /// Expected an identity for a pseudoelement
    ExpectedIdentityOnPseudoElement,

    /// A `SelectorParseErrorKind` error that isn't really supposed to happen did
    UnexpectedSelectorParseError(SelectorParseErrorKind),
}

/// A CSS selector parse failure together with its source location.
///
/// [`Selector::parse_with_location`](crate::Selector::parse_with_location) preserves this
/// location before converting the parser's error kind into [`SelectorErrorKind`].
#[derive(Debug, Clone)]
pub struct SelectorParseError {
    kind: SelectorErrorKind,
    location: SourceLocation,
}

impl SelectorParseError {
    /// Returns the classified selector parse error.
    pub const fn kind(&self) -> &SelectorErrorKind {
        &self.kind
    }

    /// Returns the parser location: a zero-based line and a one-based UTF-16 column.
    pub const fn location(&self) -> SourceLocation {
        self.location
    }

    /// Returns the classified selector parse error, discarding its location.
    pub fn into_kind(self) -> SelectorErrorKind {
        self.kind
    }
}

impl Display for SelectorParseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.kind.fmt(formatter)
    }
}

impl Error for SelectorParseError {}

impl SelectorParseError {
    pub(crate) fn from_parser(
        original: cssparser::ParseError<SelectorParseErrorKind>,
        fallback_location: SourceLocation,
    ) -> Self {
        let location = match &original.kind {
            ParseErrorKind::Custom(
                SelectorParseErrorKind::NoQualifiedNameInAttributeSelector { location, .. },
            ) => *location,
            _ => fallback_location,
        };
        Self {
            kind: SelectorErrorKind::from_parse_error_kind(original.kind),
            location,
        }
    }
}

impl From<cssparser::ParseError<SelectorParseErrorKind>> for SelectorErrorKind {
    fn from(original: cssparser::ParseError<SelectorParseErrorKind>) -> Self {
        Self::from_parse_error_kind(original.kind)
    }
}

impl SelectorErrorKind {
    fn from_parse_error_kind(error: ParseErrorKind<SelectorParseErrorKind>) -> Self {
        match error {
            ParseErrorKind::Basic(err) => SelectorErrorKind::from(err),
            ParseErrorKind::Custom(err) => SelectorErrorKind::from(err),
        }
    }
}

impl From<BasicParseErrorKind> for SelectorErrorKind {
    fn from(err: BasicParseErrorKind) -> Self {
        match err {
            BasicParseErrorKind::UnexpectedToken => Self::UnexpectedToken,
            BasicParseErrorKind::EndOfInput => Self::EndOfLine,
            BasicParseErrorKind::AtRuleInvalid => Self::InvalidAtRule,
            BasicParseErrorKind::AtRuleBodyInvalid => Self::InvalidAtRuleBody,
            BasicParseErrorKind::QualifiedRuleInvalid => Self::QualRuleInvalid,
            BasicParseErrorKind::TooManyNestedBlocks => {
                Self::UnexpectedSelectorParseError(SelectorParseErrorKind::InvalidState)
            }
        }
    }
}

impl From<SelectorParseErrorKind> for SelectorErrorKind {
    fn from(err: SelectorParseErrorKind) -> Self {
        match err {
            SelectorParseErrorKind::PseudoElementExpectedColon => {
                Self::ExpectedColonOnPseudoElement
            }
            SelectorParseErrorKind::PseudoElementExpectedIdent => {
                Self::ExpectedIdentityOnPseudoElement
            }
            other => Self::UnexpectedSelectorParseError(other),
        }
    }
}

impl Display for SelectorErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::UnexpectedToken => "Token was not expected".to_string(),
                Self::EndOfLine => "Unexpected EOL".to_string(),
                Self::InvalidAtRule => "Invalid @-rule".to_string(),
                Self::InvalidAtRuleBody => "The body of an @-rule was invalid".to_string(),
                Self::QualRuleInvalid => "The qualified name was invalid".to_string(),
                Self::ExpectedColonOnPseudoElement =>
                    "Expected a colon for pseudoelement".to_string(),
                Self::ExpectedIdentityOnPseudoElement =>
                    "Expected identity for pseudoelement".to_string(),
                Self::UnexpectedSelectorParseError(err) => format!(
                    "Unexpected error occurred. Please report this to the developer\n{err:#?}"
                ),
            }
        )
    }
}

impl Error for SelectorErrorKind {
    fn description(&self) -> &str {
        match self {
            Self::UnexpectedToken => "Token was not expected",
            Self::EndOfLine => "Unexpected EOL",
            Self::InvalidAtRule => "Invalid @-rule",
            Self::InvalidAtRuleBody => "The body of an @-rule was invalid",
            Self::QualRuleInvalid => "The qualified name was invalid",
            Self::ExpectedColonOnPseudoElement => "Missing colon character on pseudoelement",
            Self::ExpectedIdentityOnPseudoElement => "Missing pseudoelement identity",
            Self::UnexpectedSelectorParseError(_) => "Unexpected error",
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn regression_test_issue212() {
        let err = crate::Selector::parse("div138293@!#@!!@#").unwrap_err();
        assert_eq!(err.to_string(), "Token was not expected");
    }
}
