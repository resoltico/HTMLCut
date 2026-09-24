//! Custom error types for diagnostics
//! Includes re-exported error types from dependencies

mod utils;

use std::{error::Error, fmt::Display};

use cssparser::{BasicParseErrorKind, ParseErrorKind, SourceLocation, Token};
use selectors::parser::SelectorParseErrorKind;

/// Error type that is returned when calling `Selector::parse`
#[derive(Debug, Clone)]
pub enum SelectorErrorKind<'a> {
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
    ExpectedColonOnPseudoElement(Token<'a>),

    /// Expected an identity for a pseudoelement
    ExpectedIdentityOnPseudoElement(Token<'a>),

    /// A `SelectorParseErrorKind` error that isn't really supposed to happen did
    UnexpectedSelectorParseError(SelectorParseErrorKind<'a>),
}

/// A CSS selector parse failure together with its source location.
///
/// [`Selector::parse_with_location`](crate::Selector::parse_with_location) preserves this
/// location before converting the parser's error kind into [`SelectorErrorKind`].
#[derive(Debug, Clone)]
pub struct SelectorParseError<'a> {
    kind: SelectorErrorKind<'a>,
    location: SourceLocation,
}

impl<'a> SelectorParseError<'a> {
    /// Returns the classified selector parse error.
    pub const fn kind(&self) -> &SelectorErrorKind<'a> {
        &self.kind
    }

    /// Returns the parser location: a zero-based line and a one-based UTF-16 column.
    pub const fn location(&self) -> SourceLocation {
        self.location
    }

    /// Returns the classified selector parse error, discarding its location.
    pub fn into_kind(self) -> SelectorErrorKind<'a> {
        self.kind
    }
}

impl Display for SelectorParseError<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.kind.fmt(formatter)
    }
}

impl Error for SelectorParseError<'_> {}

impl<'a> SelectorParseError<'a> {
    pub(crate) fn from_parser(
        original: cssparser::ParseError<SelectorParseErrorKind<'a>>,
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

impl<'a> From<cssparser::ParseError<SelectorParseErrorKind<'a>>> for SelectorErrorKind<'a> {
    fn from(original: cssparser::ParseError<SelectorParseErrorKind<'a>>) -> Self {
        Self::from_parse_error_kind(original.kind)
    }
}

impl<'a> SelectorErrorKind<'a> {
    fn from_parse_error_kind(error: ParseErrorKind<SelectorParseErrorKind<'a>>) -> Self {
        match error {
            ParseErrorKind::Basic(err) => SelectorErrorKind::from(err),
            ParseErrorKind::Custom(err) => SelectorErrorKind::from(err),
        }
    }
}

impl<'a> From<BasicParseErrorKind> for SelectorErrorKind<'a> {
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

impl<'a> From<SelectorParseErrorKind<'a>> for SelectorErrorKind<'a> {
    fn from(err: SelectorParseErrorKind<'a>) -> Self {
        match err {
            SelectorParseErrorKind::PseudoElementExpectedColon(token) => {
                Self::ExpectedColonOnPseudoElement(token)
            }
            SelectorParseErrorKind::PseudoElementExpectedIdent(token) => {
                Self::ExpectedIdentityOnPseudoElement(token)
            }
            other => Self::UnexpectedSelectorParseError(other),
        }
    }
}

impl Display for SelectorErrorKind<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::UnexpectedToken => "Token was not expected".to_string(),
                Self::EndOfLine => "Unexpected EOL".to_string(),
                Self::InvalidAtRule => "Invalid @-rule".to_string(),
                Self::InvalidAtRuleBody => "The body of an @-rule was invalid".to_string(),
                Self::QualRuleInvalid => "The qualified name was invalid".to_string(),
                Self::ExpectedColonOnPseudoElement(token) => format!(
                    "Expected a ':' token for pseudoelement, got {:?} instead",
                    utils::render_token(token)
                ),
                Self::ExpectedIdentityOnPseudoElement(token) => format!(
                    "Expected identity for pseudoelement, got {:?} instead",
                    utils::render_token(token)
                ),
                Self::UnexpectedSelectorParseError(err) => format!(
                    "Unexpected error occurred. Please report this to the developer\n{err:#?}"
                ),
            }
        )
    }
}

impl Error for SelectorErrorKind<'_> {
    fn description(&self) -> &str {
        match self {
            Self::UnexpectedToken => "Token was not expected",
            Self::EndOfLine => "Unexpected EOL",
            Self::InvalidAtRule => "Invalid @-rule",
            Self::InvalidAtRuleBody => "The body of an @-rule was invalid",
            Self::QualRuleInvalid => "The qualified name was invalid",
            Self::ExpectedColonOnPseudoElement(_) => "Missing colon character on pseudoelement",
            Self::ExpectedIdentityOnPseudoElement(_) => "Missing pseudoelement identity",
            Self::UnexpectedSelectorParseError(_) => "Unexpected error",
        }
    }
}
