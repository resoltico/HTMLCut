// SPDX-License-Identifier: MPL-2.0 AND ISC
// Selector element binding adapted from scraper; see LICENSE-ISC.
use html5ever::Namespace;
use html5ever::ns;
use selectors::{
    Element, OpaqueElement,
    attr::{AttrSelectorOperation, CaseSensitivity, NamespaceConstraint},
    bloom::BloomFilter,
    matching,
};

use super::ElementRef;
use crate::css::{CssLocalName, CssString, NonTSPseudoClass, PseudoElement, Simple};

/// Note: will never match against non-tree-structure pseudo-classes.
impl Element for ElementRef<'_> {
    type Impl = Simple;

    fn opaque(&self) -> OpaqueElement {
        OpaqueElement::new(self.0.value())
    }

    fn parent_element(&self) -> Option<Self> {
        self.parent().and_then(ElementRef::wrap)
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

    fn is_same_type(&self, other: &Self) -> bool {
        self.value().name.local == other.value().name.local
            && self.value().name.ns == other.value().name.ns
    }

    fn imported_part(&self, _: &CssLocalName) -> Option<CssLocalName> {
        None
    }

    fn prev_sibling_element(&self) -> Option<Self> {
        self.prev_siblings()
            .find(|sibling| sibling.value().is_element())
            .map(ElementRef)
    }

    fn next_sibling_element(&self) -> Option<Self> {
        self.next_siblings()
            .find(|sibling| sibling.value().is_element())
            .map(ElementRef)
    }

    fn first_element_child(&self) -> Option<Self> {
        self.children()
            .find(|child| child.value().is_element())
            .map(ElementRef)
    }

    fn is_html_element_in_html_document(&self) -> bool {
        // FIXME: Is there more to this?
        self.value().name.ns == ns!(html)
    }

    fn has_local_name(&self, name: &CssLocalName) -> bool {
        self.value().name.local == name.0
    }

    fn has_namespace(&self, namespace: &Namespace) -> bool {
        &self.value().name.ns == namespace
    }

    fn attr_matches(
        &self,
        ns: &NamespaceConstraint<&Namespace>,
        local_name: &CssLocalName,
        operation: &AttrSelectorOperation<&CssString>,
    ) -> bool {
        self.value().attrs.iter().any(|(key, value)| {
            !matches!(*ns, NamespaceConstraint::Specific(url) if *url != key.ns)
                && local_name.0 == key.local
                && operation.eval_str(value)
        })
    }

    fn match_non_ts_pseudo_class(
        &self,
        _pc: &NonTSPseudoClass,
        _context: &mut matching::MatchingContext<'_, Self::Impl>,
    ) -> bool {
        false
    }

    fn match_pseudo_element(
        &self,
        _pe: &PseudoElement,
        _context: &mut matching::MatchingContext<Self::Impl>,
    ) -> bool {
        false
    }

    fn is_link(&self) -> bool {
        self.value().name.ns == ns!(html)
            && matches!(self.value().name(), "a" | "area" | "link")
            && self.value().attr("href").is_some()
    }

    fn is_html_slot_element(&self) -> bool {
        self.value().name.ns == ns!(html) && self.value().name() == "slot"
    }

    fn has_id(&self, id: &CssLocalName, case_sensitivity: CaseSensitivity) -> bool {
        match self.value().id() {
            Some(val) => case_sensitivity.eq(id.0.as_bytes(), val.as_bytes()),
            None => false,
        }
    }

    fn has_class(&self, name: &CssLocalName, case_sensitivity: CaseSensitivity) -> bool {
        self.value().has_class(&name.0, case_sensitivity)
    }

    fn has_custom_state(&self, _name: &CssLocalName) -> bool {
        false
    }

    fn is_empty(&self) -> bool {
        !self
            .children()
            .any(|child| child.value().is_element() || child.value().is_text())
    }

    fn is_root(&self) -> bool {
        self.parent()
            .is_some_and(|parent| parent.value().is_document())
    }

    fn apply_selector_flags(&self, _flags: matching::ElementSelectorFlags) {}

    fn add_element_unique_hashes(&self, _filter: &mut BloomFilter) -> bool {
        // FIXME: Do we want to add `self.node.id()` here?
        false
    }
}
