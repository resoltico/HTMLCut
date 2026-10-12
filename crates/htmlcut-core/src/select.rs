// SPDX-License-Identifier: MPL-2.0
//! Admitted upstream selectors with scope-local shared matching caches.
use crate::limits::{MAX_PATTERN_BYTES, MAX_PATTERN_DEPTH};
use crate::{Code, Error};
use selectors::{
    matching,
    parser::{ParseRelative, SelectorList},
};
#[path = "dom/selector_depth.rs"]
mod depth;

pub(crate) struct Selector {
    pub(crate) text: String,
    list: SelectorList<scraper::selector::Simple>,
}
impl Selector {
    pub(crate) fn compile(text: &str, path: &str) -> Result<Self, Error> {
        let failure = |message: String| {
            let mut e = Error::new(Code::InvalidSelector, message).at(path);
            e.select = Some(text.into());
            e
        };
        if text.len() > MAX_PATTERN_BYTES {
            return Err(failure(
                "The selector exceeds the 8192-byte grammar bound.".into(),
            ));
        }
        if !syntax_admitted(text) {
            return Err(failure(
                "The selector exceeds the syntax nesting bound of 64.".into(),
            ));
        }
        let mut input = cssparser::ParserInput::new(text);
        let mut parser = cssparser::Parser::new(&mut input);
        let list = SelectorList::parse(&scraper::selector::Parser, &mut parser, ParseRelative::No)
            .map_err(|e| failure(format!("Invalid CSS selector {text:?}: {e:?}.")))?;
        depth::admit(&list)
            .map_err(|()| failure("The selector exceeds the matching-depth bound of 64.".into()))?;
        Ok(Self {
            text: text.into(),
            list,
        })
    }
    pub(crate) fn matches(
        &self,
        element: scraper::ElementRef<'_>,
        scope: scraper::ElementRef<'_>,
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
