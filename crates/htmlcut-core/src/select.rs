// SPDX-License-Identifier: MPL-2.0
//! Admitted upstream selectors and scope-aware matching caches.
use crate::Error;
use crate::limits::{MAX_PATTERN_BYTES, MAX_PATTERN_DEPTH};
use selectors::{
    matching,
    parser::{ParseRelative, SelectorList},
};
#[path = "select_depth.rs"]
mod depth;

pub(crate) struct Selector {
    pub(crate) text: String,
    list: SelectorList<crate::css::Simple>,
}
impl Selector {
    #[cfg(test)]
    pub(crate) fn parse(text: &str) -> Result<Self, Error> {
        Self::compile(text, "$")
    }
    pub(crate) fn compile(text: &str, path: &str) -> Result<Self, Error> {
        if text.len() > MAX_PATTERN_BYTES {
            return Err(Error::InvalidSelector {
                path: Some(path.into()),
                select: None,
                reason: format!("The selector exceeds the {MAX_PATTERN_BYTES}-byte grammar bound."),
            });
        }
        let failure = |reason: String| Error::InvalidSelector {
            path: Some(path.into()),
            select: Some(text.into()),
            reason,
        };
        if !syntax_admitted(text) {
            return Err(failure(
                "The selector exceeds the syntax nesting bound of 64.".into(),
            ));
        }
        let mut input = cssparser::ParserInput::new(text);
        let mut parser = cssparser::Parser::new(&mut input);
        let list = SelectorList::parse(&crate::css::Parser, &mut parser, ParseRelative::No)
            .map_err(|e| {
                use cssparser::{BasicParseErrorKind, ParseErrorKind};
                use selectors::parser::SelectorParseErrorKind;
                let reason = match e.kind {
                    ParseErrorKind::Basic(BasicParseErrorKind::EndOfInput) => {
                        "unexpected end of selector".into()
                    }
                    ParseErrorKind::Custom(
                        SelectorParseErrorKind::UnsupportedPseudoClassOrElement(name),
                    ) => format!(
                        "unsupported pseudo-class or pseudo-element {}",
                        serde_json::to_string(name.as_ref()).unwrap()
                    ),
                    ParseErrorKind::Custom(SelectorParseErrorKind::EmptySelector) => {
                        "the selector is empty".into()
                    }
                    ParseErrorKind::Custom(SelectorParseErrorKind::DanglingCombinator) => {
                        "a combinator has no following selector".into()
                    }
                    _ => "selector syntax error".into(),
                };
                failure(format!(
                    "Invalid CSS selector {} at column {}: {reason}.",
                    serde_json::to_string(text).unwrap(),
                    e.location.column
                ))
            })?;
        depth::admit(&list).map_err(|message| failure(message.into()))?;
        Ok(Self {
            text: text.into(),
            list,
        })
    }
    pub(crate) fn only_scope(&self) -> bool {
        let list = self.list.slice();
        if list.len() != 1 {
            return false;
        }
        let mut components = list[0].iter_raw_match_order();
        matches!(components.next(), Some(selectors::parser::Component::Scope))
            && components.next().is_none()
    }
    pub(crate) fn matches(
        &self,
        element: crate::dom::ElementRef<'_>,
        scope: crate::dom::ElementRef<'_>,
        caches: &mut matching::SelectorCaches,
    ) -> bool {
        let mut context = matching::MatchingContext::new(
            matching::MatchingMode::Normal,
            None,
            caches,
            matching::QuirksMode::NoQuirks,
            matching::NeedsSelectorFlags::No,
            matching::MatchingForInvalidation::No,
        );
        context.scope_element = Some(selectors::Element::opaque(&scope));
        self.list
            .slice()
            .iter()
            .any(|s| matching::matches_selector(s, 0, None, &element, &mut context))
    }
}

fn syntax_admitted(text: &str) -> bool {
    let mut depth = 0usize;
    let mut quote = None;
    let mut escaped = false;
    let mut comment = false;
    let mut characters = text.chars().peekable();
    while let Some(c) = characters.next() {
        if comment {
            if c == '*' && characters.peek() == Some(&'/') {
                characters.next();
                comment = false;
            }
        } else if escaped {
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if let Some(q) = quote {
            if c == q {
                quote = None;
            }
        } else if c == '/' && characters.peek() == Some(&'*') {
            characters.next();
            comment = true;
        } else if c == '\'' || c == '"' {
            quote = Some(c);
        } else if c == '(' || c == '[' {
            depth += 1;
            if depth > MAX_PATTERN_DEPTH as usize {
                return false;
            }
        } else if c == ')' || c == ']' {
            depth = depth.saturating_sub(1);
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn syntax_nesting_counts_only_unquoted_unescaped_noncomment_delimiters() {
        // Lexical admission precedes CSS parsing. A balanced 64-level boundary
        // exercises the guard itself; parsed matching depth is a separate bound.
        assert!(syntax_admitted(&format!(
            "{}p{}",
            ":is(".repeat(64),
            ")".repeat(64)
        )));
        assert!(!syntax_admitted(&format!(
            "{}p{}",
            ":is(".repeat(65),
            ")".repeat(65)
        )));
        for ignored in [
            format!("[x='{}']", "([".repeat(100)),
            format!("[x=\"{}\"]", "([".repeat(100)),
            format!(".{}", r"\(\[".repeat(100)),
            format!("/* {} */p", "([".repeat(100)),
            format!("/* * / {} */p", "([".repeat(100)),
            format!("[x='a\\\'{}']", "([".repeat(100)),
            format!("[x=\"a\\\"{}\"]", "([".repeat(100)),
        ] {
            assert!(syntax_admitted(&ignored), "{ignored}");
            let over = format!("{ignored}{}p{}", ":is(".repeat(65), ")".repeat(65));
            assert!(
                !syntax_admitted(&over),
                "ignored text must not hide following nesting: {over}"
            );
        }
        for close in [")", "]"] {
            assert!(syntax_admitted(&format!(
                "{}{close}{}",
                "(".repeat(64),
                ")".repeat(63)
            )));
        }
        for selector in [":is(p)".repeat(65), "[x]".repeat(65)] {
            assert!(
                syntax_admitted(&selector),
                "sequential scopes do not accumulate depth"
            );
        }
        for prefix in ["/", "*"] {
            assert!(!syntax_admitted(&format!(
                "{prefix}{}p{}",
                ":is(".repeat(65),
                ")".repeat(65)
            )));
        }
    }
    #[test]
    fn selector_parse_failures_describe_the_actual_syntax() {
        for (selector, reason) in [
            ("[", "unexpected end of selector"),
            ("", "the selector is empty"),
            (",", "the selector is empty"),
            ("p >", "a combinator has no following selector"),
        ] {
            let error = Selector::compile(selector, "$").err().unwrap();
            assert!(error.to_string().contains(reason), "{}", error.to_string());
            assert_eq!(
                serde_json::to_value(&error).unwrap()["select"].as_str(),
                Some(selector)
            );
            assert_eq!(
                serde_json::to_value(&error).unwrap()["path"].as_str(),
                Some("$")
            );
        }
    }
}
