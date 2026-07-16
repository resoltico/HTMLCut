use scraper::error::{SelectorErrorKind, SelectorParseError};
use selectors::parser::SelectorParseErrorKind;
use serde_json::{Map, Value, json};

/// Closed machine-readable classes for CSS selector parse failures.
///
/// This inventory is deliberately internal implementation machinery. The interop profile
/// publishes its stable string representations in `selector_parse.parse_error_class` instead of
/// exposing the vendored parser's error types or diagnostics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SelectorParseErrorClass {
    UnexpectedToken,
    EndOfInput,
    InvalidAtRule,
    InvalidAtRuleBody,
    InvalidQualifiedRule,
    PseudoElementExpectedColon,
    PseudoElementExpectedIdent,
    InvalidAttributeSelector,
    EmptySelector,
    DanglingCombinator,
    NonCompoundSelector,
    NonPseudoElementAfterSlotted,
    InvalidPseudoElementAfterSlotted,
    InvalidPseudoElementInsideWhere,
    InvalidState,
    UnexpectedTokenInAttributeSelector,
    NoIdentForPseudo,
    UnsupportedPseudoClassOrElement,
    UnexpectedIdent,
    ExpectedNamespace,
    ExpectedBarInAttributeSelector,
    InvalidAttributeValue,
    InvalidQualifiedNameInAttributeSelector,
    ExplicitNamespaceUnexpectedToken,
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
            SelectorErrorKind::UnexpectedToken(_) => Self::UnexpectedToken,
            SelectorErrorKind::EndOfLine => Self::EndOfInput,
            SelectorErrorKind::InvalidAtRule(_) => Self::InvalidAtRule,
            SelectorErrorKind::InvalidAtRuleBody => Self::InvalidAtRuleBody,
            SelectorErrorKind::QualRuleInvalid => Self::InvalidQualifiedRule,
            SelectorErrorKind::ExpectedColonOnPseudoElement(_) => Self::PseudoElementExpectedColon,
            SelectorErrorKind::ExpectedIdentityOnPseudoElement(_) => {
                Self::PseudoElementExpectedIdent
            }
            SelectorErrorKind::UnexpectedSelectorParseError(error) => match error {
                SelectorParseErrorKind::NoQualifiedNameInAttributeSelector(_) => {
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
                SelectorParseErrorKind::BadValueInAttr(_) => Self::InvalidAttributeValue,
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SelectorParse {
    line: u64,
    column_utf16: u64,
    class: SelectorParseErrorClass,
}

impl SelectorParse {
    fn from_error(error: &SelectorParseError<'_>) -> Self {
        let location = error.location();
        Self {
            line: u64::from(location.line) + 1,
            column_utf16: u64::from(location.column),
            class: SelectorParseErrorClass::from_error_kind(error.kind()),
        }
    }

    fn as_value(self) -> Value {
        json!({
            "line": self.line,
            "column_utf16": self.column_utf16,
            "parse_error_class": self.class.as_str(),
        })
    }
}

/// Builds the public structured selector parse detail from the preserved parser error.
pub(crate) fn selector_parse_details(error: &SelectorParseError<'_>) -> Value {
    let mut details = Map::new();
    details.insert(
        "selector_parse".to_owned(),
        SelectorParse::from_error(error).as_value(),
    );
    Value::Object(details)
}

/// Validates and normalizes the closed public `selector_parse` detail object.
pub(crate) fn validate_selector_parse_details(
    value: &Value,
) -> Result<SelectorParse, &'static str> {
    let details = value.as_object().ok_or("details must be an object")?;
    let selector_parse = details
        .get("selector_parse")
        .ok_or("selector_parse is required")?;
    let selector_parse = selector_parse
        .as_object()
        .ok_or("selector_parse must be an object")?;

    const REQUIRED_FIELDS: [&str; 3] = ["line", "column_utf16", "parse_error_class"];
    if selector_parse.len() != REQUIRED_FIELDS.len()
        || REQUIRED_FIELDS
            .iter()
            .any(|field| !selector_parse.contains_key(*field))
    {
        return Err(
            "selector_parse must contain exactly line, column_utf16, and parse_error_class",
        );
    }

    let line = selector_parse
        .get("line")
        .and_then(Value::as_u64)
        .filter(|line| *line > 0)
        .ok_or("selector_parse.line must be a positive integer")?;
    let column_utf16 = selector_parse
        .get("column_utf16")
        .and_then(Value::as_u64)
        .filter(|column| *column > 0)
        .ok_or("selector_parse.column_utf16 must be a positive integer")?;
    let class = selector_parse
        .get("parse_error_class")
        .and_then(Value::as_str)
        .and_then(SelectorParseErrorClass::parse)
        .ok_or("selector_parse.parse_error_class is unknown")?;

    Ok(SelectorParse {
        line,
        column_utf16,
        class,
    })
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
                SelectorErrorKind::UnexpectedToken(token.clone()),
                SelectorParseErrorClass::UnexpectedToken,
            ),
            (
                SelectorErrorKind::EndOfLine,
                SelectorParseErrorClass::EndOfInput,
            ),
            (
                SelectorErrorKind::InvalidAtRule("rule".to_owned()),
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
                    SelectorParseErrorKind::NoQualifiedNameInAttributeSelector(token.clone()),
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
                    SelectorParseErrorKind::BadValueInAttr(token.clone()),
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
}
