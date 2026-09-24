use schemars::JsonSchema;
use scraper::error::{SelectorErrorKind, SelectorParseError};
use selectors::parser::SelectorParseErrorKind;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::num::NonZeroU64;

mod validation;

#[cfg(test)]
pub(crate) use validation::SelectorParseDetailsViolation;
pub(crate) use validation::validate_selector_parse_details;

/// Closed machine-readable classes for CSS selector parse failures.
///
/// This inventory is deliberately internal implementation machinery. The interop profile
/// publishes its stable string representations in `selector_parse.parse_error_class` instead of
/// exposing the vendored parser's error types or diagnostics.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectorParseErrorClass {
    /// The parser encountered an unexpected token.
    UnexpectedToken,
    /// The selector ended before a required token appeared.
    EndOfInput,
    /// An at-rule was not valid in selector grammar.
    InvalidAtRule,
    /// An at-rule body was not valid in selector grammar.
    InvalidAtRuleBody,
    /// A qualified selector rule was not valid.
    InvalidQualifiedRule,
    /// A pseudo-element was missing its required colon.
    PseudoElementExpectedColon,
    /// A pseudo-element was missing its identifier.
    PseudoElementExpectedIdent,
    /// An attribute-selector name was not valid.
    InvalidAttributeSelector,
    /// The selector contained no selector component.
    EmptySelector,
    /// The selector ended with a combinator.
    DanglingCombinator,
    /// A required compound selector was absent.
    NonCompoundSelector,
    /// A non-pseudo-element followed `::slotted`.
    NonPseudoElementAfterSlotted,
    /// A pseudo-element after `::slotted` was invalid.
    InvalidPseudoElementAfterSlotted,
    /// A pseudo-element inside `:where` was invalid.
    InvalidPseudoElementInsideWhere,
    /// The selector parser entered an invalid internal grammar state.
    InvalidState,
    /// An attribute selector contained an unexpected token.
    UnexpectedTokenInAttributeSelector,
    /// A pseudo-class or pseudo-element was missing an identifier.
    NoIdentForPseudo,
    /// The requested pseudo-class or pseudo-element is unsupported.
    UnsupportedPseudoClassOrElement,
    /// The parser encountered an unexpected identifier.
    UnexpectedIdent,
    /// A namespace identifier was required but absent.
    ExpectedNamespace,
    /// An attribute selector was missing its namespace separator.
    ExpectedBarInAttributeSelector,
    /// An attribute selector carried an invalid value.
    InvalidAttributeValue,
    /// An attribute selector carried an invalid qualified name.
    InvalidQualifiedNameInAttributeSelector,
    /// An explicit namespace contained an unexpected token.
    ExplicitNamespaceUnexpectedToken,
    /// A class selector was missing its identifier.
    ClassNeedsIdent,
}

impl SelectorParseErrorClass {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::UnexpectedToken => "unexpected_token",
            Self::EndOfInput => "end_of_input",
            Self::InvalidAtRule => "invalid_at_rule",
            Self::InvalidAtRuleBody => "invalid_at_rule_body",
            Self::InvalidQualifiedRule => "invalid_qualified_rule",
            Self::PseudoElementExpectedColon => "pseudo_element_expected_colon",
            Self::PseudoElementExpectedIdent => "pseudo_element_expected_ident",
            Self::InvalidAttributeSelector => "invalid_attribute_selector",
            Self::EmptySelector => "empty_selector",
            Self::DanglingCombinator => "dangling_combinator",
            Self::NonCompoundSelector => "non_compound_selector",
            Self::NonPseudoElementAfterSlotted => "non_pseudo_element_after_slotted",
            Self::InvalidPseudoElementAfterSlotted => "invalid_pseudo_element_after_slotted",
            Self::InvalidPseudoElementInsideWhere => "invalid_pseudo_element_inside_where",
            Self::InvalidState => "invalid_state",
            Self::UnexpectedTokenInAttributeSelector => "unexpected_token_in_attribute_selector",
            Self::NoIdentForPseudo => "no_ident_for_pseudo",
            Self::UnsupportedPseudoClassOrElement => "unsupported_pseudo_class_or_element",
            Self::UnexpectedIdent => "unexpected_ident",
            Self::ExpectedNamespace => "expected_namespace",
            Self::ExpectedBarInAttributeSelector => "expected_bar_in_attribute_selector",
            Self::InvalidAttributeValue => "invalid_attribute_value",
            Self::InvalidQualifiedNameInAttributeSelector => {
                "invalid_qualified_name_in_attribute_selector"
            }
            Self::ExplicitNamespaceUnexpectedToken => "explicit_namespace_unexpected_token",
            Self::ClassNeedsIdent => "class_needs_ident",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "unexpected_token" => Self::UnexpectedToken,
            "end_of_input" => Self::EndOfInput,
            "invalid_at_rule" => Self::InvalidAtRule,
            "invalid_at_rule_body" => Self::InvalidAtRuleBody,
            "invalid_qualified_rule" => Self::InvalidQualifiedRule,
            "pseudo_element_expected_colon" => Self::PseudoElementExpectedColon,
            "pseudo_element_expected_ident" => Self::PseudoElementExpectedIdent,
            "invalid_attribute_selector" => Self::InvalidAttributeSelector,
            "empty_selector" => Self::EmptySelector,
            "dangling_combinator" => Self::DanglingCombinator,
            "non_compound_selector" => Self::NonCompoundSelector,
            "non_pseudo_element_after_slotted" => Self::NonPseudoElementAfterSlotted,
            "invalid_pseudo_element_after_slotted" => Self::InvalidPseudoElementAfterSlotted,
            "invalid_pseudo_element_inside_where" => Self::InvalidPseudoElementInsideWhere,
            "invalid_state" => Self::InvalidState,
            "unexpected_token_in_attribute_selector" => Self::UnexpectedTokenInAttributeSelector,
            "no_ident_for_pseudo" => Self::NoIdentForPseudo,
            "unsupported_pseudo_class_or_element" => Self::UnsupportedPseudoClassOrElement,
            "unexpected_ident" => Self::UnexpectedIdent,
            "expected_namespace" => Self::ExpectedNamespace,
            "expected_bar_in_attribute_selector" => Self::ExpectedBarInAttributeSelector,
            "invalid_attribute_value" => Self::InvalidAttributeValue,
            "invalid_qualified_name_in_attribute_selector" => {
                Self::InvalidQualifiedNameInAttributeSelector
            }
            "explicit_namespace_unexpected_token" => Self::ExplicitNamespaceUnexpectedToken,
            "class_needs_ident" => Self::ClassNeedsIdent,
            _ => return None,
        })
    }

    fn from_error_kind(error: &SelectorErrorKind<'_>) -> Self {
        match error {
            SelectorErrorKind::UnexpectedToken => Self::UnexpectedToken,
            SelectorErrorKind::EndOfLine => Self::EndOfInput,
            SelectorErrorKind::InvalidAtRule => Self::InvalidAtRule,
            SelectorErrorKind::InvalidAtRuleBody => Self::InvalidAtRuleBody,
            SelectorErrorKind::QualRuleInvalid => Self::InvalidQualifiedRule,
            SelectorErrorKind::ExpectedColonOnPseudoElement(_) => Self::PseudoElementExpectedColon,
            SelectorErrorKind::ExpectedIdentityOnPseudoElement(_) => {
                Self::PseudoElementExpectedIdent
            }
            SelectorErrorKind::UnexpectedSelectorParseError(error) => match error {
                SelectorParseErrorKind::NoQualifiedNameInAttributeSelector { .. } => {
                    Self::InvalidAttributeSelector
                }
                SelectorParseErrorKind::EmptySelector => Self::EmptySelector,
                SelectorParseErrorKind::DanglingCombinator => Self::DanglingCombinator,
                SelectorParseErrorKind::NonCompoundSelector => Self::NonCompoundSelector,
                SelectorParseErrorKind::NonPseudoElementAfterSlotted => {
                    Self::NonPseudoElementAfterSlotted
                }
                SelectorParseErrorKind::InvalidPseudoElementAfterSlotted => {
                    Self::InvalidPseudoElementAfterSlotted
                }
                SelectorParseErrorKind::InvalidPseudoElementInsideWhere => {
                    Self::InvalidPseudoElementInsideWhere
                }
                SelectorParseErrorKind::InvalidState => Self::InvalidState,
                SelectorParseErrorKind::UnexpectedTokenInAttributeSelector(_) => {
                    Self::UnexpectedTokenInAttributeSelector
                }
                SelectorParseErrorKind::PseudoElementExpectedColon(_) => {
                    Self::PseudoElementExpectedColon
                }
                SelectorParseErrorKind::PseudoElementExpectedIdent(_) => {
                    Self::PseudoElementExpectedIdent
                }
                SelectorParseErrorKind::NoIdentForPseudo(_) => Self::NoIdentForPseudo,
                SelectorParseErrorKind::UnsupportedPseudoClassOrElement(_) => {
                    Self::UnsupportedPseudoClassOrElement
                }
                SelectorParseErrorKind::UnexpectedIdent(_) => Self::UnexpectedIdent,
                SelectorParseErrorKind::ExpectedNamespace(_) => Self::ExpectedNamespace,
                SelectorParseErrorKind::ExpectedBarInAttr(_) => {
                    Self::ExpectedBarInAttributeSelector
                }
                SelectorParseErrorKind::BadValueInAttr => Self::InvalidAttributeValue,
                SelectorParseErrorKind::InvalidQualNameInAttr(_) => {
                    Self::InvalidQualifiedNameInAttributeSelector
                }
                SelectorParseErrorKind::ExplicitNamespaceUnexpectedToken(_) => {
                    Self::ExplicitNamespaceUnexpectedToken
                }
                SelectorParseErrorKind::ClassNeedsIdent(_) => Self::ClassNeedsIdent,
            },
        }
    }
}

/// One normalized selector parse position and classification.
#[derive(Clone, Copy, Debug, Deserialize, JsonSchema, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SelectorParseDetail {
    /// One-based source line that contains the parser rejection.
    pub line: NonZeroU64,
    /// One-based UTF-16 column within [`Self::line`].
    pub column_utf16: NonZeroU64,
    /// Closed HTMLCut-owned class of the parser rejection.
    pub parse_error_class: SelectorParseErrorClass,
}

impl SelectorParseDetail {
    fn from_error(error: &SelectorParseError<'_>) -> Self {
        let location = error.location();
        Self {
            line: NonZeroU64::new(u64::from(location.line) + 1)
                .expect("CSS parser line positions are one-based"),
            column_utf16: NonZeroU64::new(u64::from(location.column))
                .expect("CSS parser column positions are one-based"),
            parse_error_class: SelectorParseErrorClass::from_error_kind(error.kind()),
        }
    }

    fn as_value(self) -> Value {
        json!({
            "line": self.line.get(),
            "column_utf16": self.column_utf16.get(),
            "parse_error_class": self.parse_error_class.as_str(),
        })
    }
}

/// Builds the public structured selector parse detail from the preserved parser error.
pub(crate) fn selector_parse_details(error: &SelectorParseError<'_>) -> Value {
    let mut details = Map::new();
    details.insert(
        "selector_parse".to_owned(),
        SelectorParseDetail::from_error(error).as_value(),
    );
    Value::Object(details)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cssparser::{CowRcStr, Token};

    #[test]
    fn selector_parse_error_classes_are_closed_and_exhaustively_mapped() {
        let token = Token::Ident(CowRcStr::from("htmlcut"));
        let identifier = CowRcStr::from("htmlcut");

        let cases = [
            (
                SelectorErrorKind::UnexpectedToken,
                SelectorParseErrorClass::UnexpectedToken,
            ),
            (
                SelectorErrorKind::EndOfLine,
                SelectorParseErrorClass::EndOfInput,
            ),
            (
                SelectorErrorKind::InvalidAtRule,
                SelectorParseErrorClass::InvalidAtRule,
            ),
            (
                SelectorErrorKind::InvalidAtRuleBody,
                SelectorParseErrorClass::InvalidAtRuleBody,
            ),
            (
                SelectorErrorKind::QualRuleInvalid,
                SelectorParseErrorClass::InvalidQualifiedRule,
            ),
            (
                SelectorErrorKind::ExpectedColonOnPseudoElement(token.clone()),
                SelectorParseErrorClass::PseudoElementExpectedColon,
            ),
            (
                SelectorErrorKind::ExpectedIdentityOnPseudoElement(token.clone()),
                SelectorParseErrorClass::PseudoElementExpectedIdent,
            ),
            (
                SelectorErrorKind::UnexpectedSelectorParseError(
                    SelectorParseErrorKind::NoQualifiedNameInAttributeSelector {
                        token: token.clone(),
                        location: cssparser::SourceLocation { line: 0, column: 1 },
                    },
                ),
                SelectorParseErrorClass::InvalidAttributeSelector,
            ),
            (
                SelectorErrorKind::UnexpectedSelectorParseError(
                    SelectorParseErrorKind::EmptySelector,
                ),
                SelectorParseErrorClass::EmptySelector,
            ),
            (
                SelectorErrorKind::UnexpectedSelectorParseError(
                    SelectorParseErrorKind::DanglingCombinator,
                ),
                SelectorParseErrorClass::DanglingCombinator,
            ),
            (
                SelectorErrorKind::UnexpectedSelectorParseError(
                    SelectorParseErrorKind::NonCompoundSelector,
                ),
                SelectorParseErrorClass::NonCompoundSelector,
            ),
            (
                SelectorErrorKind::UnexpectedSelectorParseError(
                    SelectorParseErrorKind::NonPseudoElementAfterSlotted,
                ),
                SelectorParseErrorClass::NonPseudoElementAfterSlotted,
            ),
            (
                SelectorErrorKind::UnexpectedSelectorParseError(
                    SelectorParseErrorKind::InvalidPseudoElementAfterSlotted,
                ),
                SelectorParseErrorClass::InvalidPseudoElementAfterSlotted,
            ),
            (
                SelectorErrorKind::UnexpectedSelectorParseError(
                    SelectorParseErrorKind::InvalidPseudoElementInsideWhere,
                ),
                SelectorParseErrorClass::InvalidPseudoElementInsideWhere,
            ),
            (
                SelectorErrorKind::UnexpectedSelectorParseError(
                    SelectorParseErrorKind::InvalidState,
                ),
                SelectorParseErrorClass::InvalidState,
            ),
            (
                SelectorErrorKind::UnexpectedSelectorParseError(
                    SelectorParseErrorKind::UnexpectedTokenInAttributeSelector(token.clone()),
                ),
                SelectorParseErrorClass::UnexpectedTokenInAttributeSelector,
            ),
            (
                SelectorErrorKind::UnexpectedSelectorParseError(
                    SelectorParseErrorKind::PseudoElementExpectedColon(token.clone()),
                ),
                SelectorParseErrorClass::PseudoElementExpectedColon,
            ),
            (
                SelectorErrorKind::UnexpectedSelectorParseError(
                    SelectorParseErrorKind::PseudoElementExpectedIdent(token.clone()),
                ),
                SelectorParseErrorClass::PseudoElementExpectedIdent,
            ),
            (
                SelectorErrorKind::UnexpectedSelectorParseError(
                    SelectorParseErrorKind::NoIdentForPseudo(token.clone()),
                ),
                SelectorParseErrorClass::NoIdentForPseudo,
            ),
            (
                SelectorErrorKind::UnexpectedSelectorParseError(
                    SelectorParseErrorKind::UnsupportedPseudoClassOrElement(identifier.clone()),
                ),
                SelectorParseErrorClass::UnsupportedPseudoClassOrElement,
            ),
            (
                SelectorErrorKind::UnexpectedSelectorParseError(
                    SelectorParseErrorKind::UnexpectedIdent(identifier.clone()),
                ),
                SelectorParseErrorClass::UnexpectedIdent,
            ),
            (
                SelectorErrorKind::UnexpectedSelectorParseError(
                    SelectorParseErrorKind::ExpectedNamespace(identifier.clone()),
                ),
                SelectorParseErrorClass::ExpectedNamespace,
            ),
            (
                SelectorErrorKind::UnexpectedSelectorParseError(
                    SelectorParseErrorKind::ExpectedBarInAttr(token.clone()),
                ),
                SelectorParseErrorClass::ExpectedBarInAttributeSelector,
            ),
            (
                SelectorErrorKind::UnexpectedSelectorParseError(
                    SelectorParseErrorKind::BadValueInAttr,
                ),
                SelectorParseErrorClass::InvalidAttributeValue,
            ),
            (
                SelectorErrorKind::UnexpectedSelectorParseError(
                    SelectorParseErrorKind::InvalidQualNameInAttr(token.clone()),
                ),
                SelectorParseErrorClass::InvalidQualifiedNameInAttributeSelector,
            ),
            (
                SelectorErrorKind::UnexpectedSelectorParseError(
                    SelectorParseErrorKind::ExplicitNamespaceUnexpectedToken(token.clone()),
                ),
                SelectorParseErrorClass::ExplicitNamespaceUnexpectedToken,
            ),
            (
                SelectorErrorKind::UnexpectedSelectorParseError(
                    SelectorParseErrorKind::ClassNeedsIdent(token),
                ),
                SelectorParseErrorClass::ClassNeedsIdent,
            ),
        ];

        for (error, expected) in cases {
            assert_eq!(SelectorParseErrorClass::from_error_kind(&error), expected);
        }

        for class in [
            SelectorParseErrorClass::UnexpectedToken,
            SelectorParseErrorClass::EndOfInput,
            SelectorParseErrorClass::InvalidAtRule,
            SelectorParseErrorClass::InvalidAtRuleBody,
            SelectorParseErrorClass::InvalidQualifiedRule,
            SelectorParseErrorClass::PseudoElementExpectedColon,
            SelectorParseErrorClass::PseudoElementExpectedIdent,
            SelectorParseErrorClass::InvalidAttributeSelector,
            SelectorParseErrorClass::EmptySelector,
            SelectorParseErrorClass::DanglingCombinator,
            SelectorParseErrorClass::NonCompoundSelector,
            SelectorParseErrorClass::NonPseudoElementAfterSlotted,
            SelectorParseErrorClass::InvalidPseudoElementAfterSlotted,
            SelectorParseErrorClass::InvalidPseudoElementInsideWhere,
            SelectorParseErrorClass::InvalidState,
            SelectorParseErrorClass::UnexpectedTokenInAttributeSelector,
            SelectorParseErrorClass::NoIdentForPseudo,
            SelectorParseErrorClass::UnsupportedPseudoClassOrElement,
            SelectorParseErrorClass::UnexpectedIdent,
            SelectorParseErrorClass::ExpectedNamespace,
            SelectorParseErrorClass::ExpectedBarInAttributeSelector,
            SelectorParseErrorClass::InvalidAttributeValue,
            SelectorParseErrorClass::InvalidQualifiedNameInAttributeSelector,
            SelectorParseErrorClass::ExplicitNamespaceUnexpectedToken,
            SelectorParseErrorClass::ClassNeedsIdent,
        ] {
            assert_eq!(SelectorParseErrorClass::parse(class.as_str()), Some(class));
        }
        assert_eq!(
            SelectorParseErrorClass::parse("not_a_selector_parse_error"),
            None
        );
    }

    #[test]
    fn selector_parse_details_require_exactly_the_closed_field_set() {
        let extra = serde_json::json!({
            "selector_parse": {
                "line": 1,
                "column_utf16": 1,
                "parse_error_class": "unexpected-token",
                "extra": true
            }
        });
        assert_eq!(
            validate_selector_parse_details(&extra),
            Err(SelectorParseDetailsViolation::Malformed)
        );

        let substituted = serde_json::json!({
            "selector_parse": {
                "line": 1,
                "column_utf16": 1,
                "extra": true
            }
        });
        assert_eq!(
            validate_selector_parse_details(&substituted),
            Err(SelectorParseDetailsViolation::Malformed)
        );
    }
}
