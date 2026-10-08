// SPDX-License-Identifier: MPL-2.0 AND ISC
//! CSS grammar and cooperative accounting through selectors public Element callbacks.
use crate::budget::WorkBudget;
use crate::dom::{ElementRef, Html};
use cssparser::ToCss;
use html5ever::{LocalName, Namespace};
use precomputed_hash::PrecomputedHash;
use selectors::{
    OpaqueElement,
    attr::{AttrSelectorOperation, CaseSensitivity, NamespaceConstraint},
    bloom::BloomFilter,
    matching::{self, SelectorCaches},
    parser::{self, ParseRelative, SelectorList, SelectorParseErrorKind},
};
use std::fmt;
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Selector {
    selectors: SelectorList<StaticSelectors>,
}
#[derive(Clone, Copy)]
struct Parser;
impl<'i> parser::Parser<'i> for Parser {
    type Impl = StaticSelectors;
    type Error = SelectorParseErrorKind;
    fn parse_is_and_where(&self) -> bool {
        true
    }
    fn parse_has(&self) -> bool {
        true
    }
}
#[derive(Debug, Clone, Copy)]
pub(crate) enum SelectorMatchError {
    WorkLimitExceeded,
    DocumentMismatch,
}
pub(crate) struct BudgetedMatcher<'s, 'd> {
    selector: &'s Selector,
    document: &'d Html,
    scope: Option<ElementRef<'d>>,
    budget: &'s WorkBudget,
    caches: SelectorCaches,
}
impl Selector {
    pub fn parse(value: &str) -> Result<Self, String> {
        let mut parser = cssparser::Parser::new(value);
        SelectorList::parse(&Parser, &mut parser, ParseRelative::No)
            .map(|selectors| Self { selectors })
            .map_err(|e| format!("{e:?}"))
    }
    pub fn budgeted<'s, 'd>(
        &'s self,
        document: &'d Html,
        scope: Option<ElementRef<'d>>,
        budget: &'s WorkBudget,
    ) -> Result<BudgetedMatcher<'s, 'd>, SelectorMatchError> {
        if scope.is_some_and(|s| !std::ptr::eq(s.tree(), &document.tree)) {
            return Err(SelectorMatchError::DocumentMismatch);
        }
        Ok(BudgetedMatcher {
            selector: self,
            document,
            scope,
            budget,
            caches: Default::default(),
        })
    }
}
impl<'d> BudgetedMatcher<'_, 'd> {
    pub fn matches(&mut self, element: &ElementRef<'d>) -> Result<bool, SelectorMatchError> {
        if !std::ptr::eq(element.tree(), &self.document.tree) {
            return Err(SelectorMatchError::DocumentMismatch);
        }
        if !self.budget.consume() {
            return Err(SelectorMatchError::WorkLimitExceeded);
        }
        let mut context = matching::MatchingContext::new(
            matching::MatchingMode::Normal,
            None,
            &mut self.caches,
            matching::QuirksMode::NoQuirks,
            matching::NeedsSelectorFlags::No,
            matching::MatchingForInvalidation::No,
        );
        context.scope_element = self.scope.map(|e| OpaqueElement::new(e.node.value()));
        let element = BudgetedElement {
            element: *element,
            budget: self.budget,
        };
        let matched = self
            .selector
            .selectors
            .slice()
            .iter()
            .any(|s| matching::matches_selector(s, 0, None, &element, &mut context));
        if self.budget.exhausted() {
            Err(SelectorMatchError::WorkLimitExceeded)
        } else {
            Ok(matched)
        }
    }
}
#[derive(Clone, Copy, Debug)]
struct BudgetedElement<'d, 'b> {
    element: ElementRef<'d>,
    budget: &'b WorkBudget,
}
impl<'d, 'b> BudgetedElement<'d, 'b> {
    fn step(&self) -> bool {
        self.budget.consume()
    }
    fn bytes(&self, bytes: usize) -> bool {
        (0..bytes.div_ceil(64)).all(|_| self.step())
    }
    fn wrap(&self, e: ElementRef<'d>) -> Self {
        Self {
            element: e,
            budget: self.budget,
        }
    }
    fn navigate(
        &self,
        mut nodes: impl Iterator<Item = ego_tree::NodeRef<'d, crate::dom::Node>>,
    ) -> Option<Self> {
        {
            for n in nodes.by_ref() {
                if !self.step() {
                    return None;
                }
                if let Some(e) = ElementRef::wrap(n) {
                    return Some(self.wrap(e));
                }
            }
            None
        }
    }
}
impl selectors::Element for BudgetedElement<'_, '_> {
    type Impl = StaticSelectors;
    fn opaque(&self) -> OpaqueElement {
        OpaqueElement::new(self.element.node.value())
    }
    fn parent_element(&self) -> Option<Self> {
        if !self.step() {
            return None;
        }
        self.element
            .parent()
            .and_then(ElementRef::wrap)
            .map(|e| self.wrap(e))
    }
    fn prev_sibling_element(&self) -> Option<Self> {
        self.navigate(self.element.prev_siblings())
    }
    fn next_sibling_element(&self) -> Option<Self> {
        self.navigate(self.element.next_siblings())
    }
    fn first_element_child(&self) -> Option<Self> {
        self.navigate(self.element.children())
    }
    fn parent_node_is_shadow_root(&self) -> bool {
        false
    }
    fn containing_shadow_host(&self) -> Option<Self> {
        None
    }
    fn is_pseudo_element(&self) -> bool {
        false
    }
    fn is_part(&self, _name: &CssLocalName) -> bool {
        false
    }
    fn imported_part(&self, _name: &CssLocalName) -> Option<CssLocalName> {
        None
    }
    fn is_same_type(&self, other: &Self) -> bool {
        self.step() && self.element.value().name == other.element.value().name
    }
    fn is_html_element_in_html_document(&self) -> bool {
        self.step() && self.element.value().name.ns == html5ever::ns!(html)
    }
    fn has_local_name(&self, name: &CssLocalName) -> bool {
        self.step() && self.element.value().name.local == name.0
    }
    fn has_namespace(&self, name: &Namespace) -> bool {
        self.step() && &self.element.value().name.ns == name
    }
    fn attr_matches(
        &self,
        ns: &NamespaceConstraint<&Namespace>,
        name: &CssLocalName,
        op: &AttrSelectorOperation<&CssString>,
    ) -> bool {
        for (key, value) in &self.element.value().attrs {
            if !self.step() {
                return false;
            }
            if !matches!(*ns,NamespaceConstraint::Specific(url) if *url!=key.ns)
                && name.0 == key.local
            {
                if !self.bytes(value.len()) {
                    return false;
                }
                if op.eval_str(value) {
                    return true;
                }
            }
        }
        false
    }
    fn match_non_ts_pseudo_class(
        &self,
        _pc: &NonTSPseudoClass,
        _ctx: &mut matching::MatchingContext<'_, StaticSelectors>,
    ) -> bool {
        false
    }
    fn match_pseudo_element(
        &self,
        _pe: &PseudoElement,
        _ctx: &mut matching::MatchingContext<StaticSelectors>,
    ) -> bool {
        false
    }
    fn is_link(&self) -> bool {
        self.step() && self.element.value().name() == "link"
    }
    fn is_html_slot_element(&self) -> bool {
        true
    }
    fn has_id(&self, id: &CssLocalName, case: CaseSensitivity) -> bool {
        for (n, v) in &self.element.value().attrs {
            if !self.step() {
                return false;
            }
            if &*n.local == "id" {
                return self.bytes(v.len()) && case.eq(id.0.as_bytes(), v.as_bytes());
            }
        }
        false
    }
    fn has_class(&self, name: &CssLocalName, case: CaseSensitivity) -> bool {
        for (n, v) in &self.element.value().attrs {
            if !self.step() {
                return false;
            }
            if &*n.local == "class" {
                if !self.bytes(v.len()) {
                    return false;
                }
                for token in v.split_ascii_whitespace() {
                    if !self.step() {
                        return false;
                    }
                    if case.eq(token.as_bytes(), name.0.as_bytes()) {
                        return true;
                    }
                }
            }
        }
        false
    }
    fn has_custom_state(&self, _name: &CssLocalName) -> bool {
        false
    }
    fn is_empty(&self) -> bool {
        for n in self.element.children() {
            if !self.step() {
                return false;
            }
            if n.value().is_element() || n.value().is_text() {
                return false;
            }
        }
        true
    }
    fn is_root(&self) -> bool {
        self.step()
            && self
                .element
                .parent()
                .is_some_and(|n| n.value().is_document())
    }
    fn apply_selector_flags(&self, _flags: matching::ElementSelectorFlags) {}
    fn add_element_unique_hashes(&self, _filter: &mut BloomFilter) -> bool {
        false
    }
}
/// A simple implementation of `SelectorImpl` with no pseudo-classes or pseudo-elements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaticSelectors;

impl parser::SelectorImpl for StaticSelectors {
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
pub struct CssString(pub String);

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
pub struct CssLocalName(pub LocalName);

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
pub enum NonTSPseudoClass {}

impl parser::NonTSPseudoClass for NonTSPseudoClass {
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
pub enum PseudoElement {}

impl parser::PseudoElement for PseudoElement {}

impl ToCss for PseudoElement {
    fn to_css<W>(&self, dest: &mut W) -> fmt::Result
    where
        W: fmt::Write,
    {
        dest.write_str("")
    }
}
