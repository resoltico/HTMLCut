// SPDX-License-Identifier: MPL-2.0 AND ISC
// Selector trait binding adapted from scraper; see LICENSE-ISC.
use cssparser::ToCss;
use html5ever::{LocalName, Namespace};
use precomputed_hash::PrecomputedHash;
use selectors::parser::{self, SelectorParseErrorKind};
use std::fmt;
/// An implementation of `Parser` for `selectors`
#[derive(Clone, Copy, Debug)]
pub(crate) struct Parser;
impl<'i> parser::Parser<'i> for Parser {
    type Impl = Simple;
    type Error = SelectorParseErrorKind<'i>;

    fn parse_is_and_where(&self) -> bool {
        true
    }

    fn parse_has(&self) -> bool {
        true
    }
}

/// A simple implementation of `SelectorImpl` with no pseudo-classes or pseudo-elements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Simple;

impl parser::SelectorImpl for Simple {
    type AttrValue = CssString;
    type Identifier = CssLocalName;
    type LocalName = CssLocalName;
    type NamespacePrefix = CssLocalName;
    type NamespaceUrl = Namespace;
    type BorrowedNamespaceUrl = Namespace;
    type BorrowedLocalName = CssLocalName;

    type NonTSPseudoClass = NonTSPseudoClass;
    type PseudoElement = PseudoElement;

    // see: https://github.com/servo/servo/pull/19747#issuecomment-357106065
    type ExtraMatchingData<'a> = ();
}

/// Wraps [`String`] so that it can be used with [`selectors`]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CssString(pub String);

impl<'a> From<&'a str> for CssString {
    fn from(val: &'a str) -> Self {
        Self(val.to_owned())
    }
}

impl AsRef<str> for CssString {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl ToCss for CssString {
    fn to_css<W>(&self, dest: &mut W) -> fmt::Result
    where
        W: fmt::Write,
    {
        cssparser::serialize_string(&self.0, dest)
    }
}

/// Wraps [`LocalName`] so that it can be used with [`selectors`]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct CssLocalName(pub LocalName);

impl<'a> From<&'a str> for CssLocalName {
    fn from(val: &'a str) -> Self {
        Self(val.into())
    }
}

impl ToCss for CssLocalName {
    fn to_css<W>(&self, dest: &mut W) -> fmt::Result
    where
        W: fmt::Write,
    {
        dest.write_str(&self.0)
    }
}

impl PrecomputedHash for CssLocalName {
    fn precomputed_hash(&self) -> u32 {
        self.0.precomputed_hash()
    }
}

/// Non Tree-Structural Pseudo-Class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NonTSPseudoClass {}

impl parser::NonTSPseudoClass for NonTSPseudoClass {
    type Impl = Simple;

    fn is_active_or_hover(&self) -> bool {
        false
    }

    fn is_user_action_state(&self) -> bool {
        false
    }
}

impl ToCss for NonTSPseudoClass {
    fn to_css<W>(&self, dest: &mut W) -> fmt::Result
    where
        W: fmt::Write,
    {
        dest.write_str("")
    }
}

/// CSS Pseudo-Element
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PseudoElement {}

impl parser::PseudoElement for PseudoElement {
    type Impl = Simple;
}

impl ToCss for PseudoElement {
    fn to_css<W>(&self, dest: &mut W) -> fmt::Result
    where
        W: fmt::Write,
    {
        dest.write_str("")
    }
}

#[cfg(test)]
mod tests {
    use crate::test_support::{DocumentFixture, TestOptions, parse_fixture};
    use crate::{Contract, ErrorCode, Value};

    #[test]
    fn static_selector_contract_refuses_state_and_generated_content_with_original_facts() {
        for selector in [
            "p:hover",
            "p:active",
            "input:focus",
            "input:checked",
            "input:disabled",
            "a:link",
            "a:visited",
            "p:lang(en)",
            "p::before",
            "p::after",
            "p[data-note='original spelling']:hover",
        ] {
            let input = serde_json::json!({"select": selector}).to_string();
            let error = Contract::from_json(&input, &crate::stop::Clock::new(None).unwrap())
                .err()
                .unwrap();
            assert_eq!(error.code(), ErrorCode::InvalidSelector, "{selector}");
            assert_eq!(
                serde_json::to_value(&error).unwrap()["path"].as_str(),
                Some("$.select")
            );
            assert_eq!(
                serde_json::to_value(&error).unwrap()["select"].as_str(),
                Some(selector)
            );
            assert!(
                error.to_string().contains("unsupported pseudo"),
                "{error:?}"
            );
        }

        // The documented attribute alternatives inspect snapshot state without
        // inventing a live browser state or pseudo-element selection surface.
        let document = parse_fixture(
            DocumentFixture::new(
                b"<a href='/'>Follow</a><input checked value='Check'><input disabled value='Off'>",
            ),
            &TestOptions::default().preparation(),
            &crate::stop::Clock::new(TestOptions::default().time_limit).unwrap(),
        )
        .unwrap();
        for (selector, read, expected) in [
            ("a[href]", "text", "Follow"),
            ("[checked]", "attr:value", "Check"),
            ("[disabled]", "attr:value", "Off"),
        ] {
            let input = serde_json::json!({"select": selector, "read": read}).to_string();
            let contract =
                Contract::from_json(&input, &crate::stop::Clock::new(None).unwrap()).unwrap();
            assert_eq!(
                document
                    .extract(
                        &contract,
                        crate::ExportFormat::Json,
                        &crate::test_support::TestOptions::default().execution(),
                        &crate::stop::Clock::new(
                            (crate::test_support::TestOptions::default()).time_limit
                        )
                        .unwrap()
                    )
                    .unwrap(),
                Value::String(expected.into())
            );
        }
    }
}
