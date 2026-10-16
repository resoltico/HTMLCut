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

#[cfg(test)]
mod tests {
    use crate::{Contract, Document, Limits, Source, Value};

    #[test]
    fn public_structural_matching_uses_element_siblings_and_real_text_emptiness() {
        let html = b"<html id=h><main><p id=a></p>text<!--c--><span id=s>x</span><p id=b><i></i></p><p id=c> </p><p id=d><!--only--></p></main>";
        let document = Document::parse(
            Source::new(html),
            &Limits::default().preparation(),
            &crate::ExecutionContext::new(Limits::default().time_limit).unwrap(),
        )
        .unwrap();
        for (selector, expected) in [
            ("main > p:nth-of-type(2)", vec!["b"]),
            ("main > p:nth-last-of-type(2)", vec!["c"]),
            ("main > p + span", vec!["s"]),
            ("main > span ~ p", vec!["b", "c", "d"]),
            ("main > :first-child", vec!["a"]),
            ("main > :last-child", vec!["d"]),
            ("main > p:empty", vec!["a", "d"]),
            (":root", vec!["h"]),
            ("|p", vec![]),
        ] {
            let contract = Contract::from_json(
                &serde_json::json!({"select":selector, "match":"all", "min":0, "read":"attr:id"})
                    .to_string(),
                &crate::ExecutionContext::new(None).unwrap(),
            )
            .unwrap();
            assert_eq!(
                document
                    .extract(
                        &contract,
                        &crate::Limits::default().execution(),
                        &crate::ExecutionContext::new((crate::Limits::default()).time_limit)
                            .unwrap()
                    )
                    .unwrap(),
                Value::List(
                    expected
                        .into_iter()
                        .map(|id| Value::String(id.into()))
                        .collect()
                ),
                "{selector}"
            );
        }
    }
    #[test]
    fn repeated_relative_selector_lookups_do_not_publish_an_empty_bloom_filter() {
        let document = Document::parse(
            Source::new(b"<main id=matched><span class=first></span><b class=second></b></main>"),
            &Limits::default().preparation(),
            &crate::ExecutionContext::new(Limits::default().time_limit).unwrap(),
        )
        .unwrap();
        let contract = Contract::from_json(
            r#"{"select":"main:has(.first):has(.second)","read":"attr:id"}"#,
            &crate::ExecutionContext::new(None).unwrap(),
        )
        .unwrap();
        assert_eq!(
            document
                .extract(
                    &contract,
                    &crate::Limits::default().execution(),
                    &crate::ExecutionContext::new((crate::Limits::default()).time_limit).unwrap()
                )
                .unwrap(),
            Value::String("matched".into())
        );
    }

    #[test]
    fn html_and_foreign_element_case_rules_stay_distinct() {
        let html = b"<svg><linearGradient id=g></linearGradient></svg><input id=h type=TEXT><svg><a id=s type=TEXT></a></svg>";
        let document = Document::parse(
            Source::new(html),
            &Limits::default().preparation(),
            &crate::ExecutionContext::new(Limits::default().time_limit).unwrap(),
        )
        .unwrap();
        for (selector, expected) in [
            ("INPUT", vec!["h"]),
            ("linearGradient", vec!["g"]),
            ("lineargradient", vec![]),
            ("[type=text]", vec!["h"]),
            ("[type=text i]", vec!["h", "s"]),
            ("[type=text s]", vec![]),
        ] {
            let contract = Contract::from_json(
                &serde_json::json!({"select":selector,"match":"all","min":0,"read":"attr:id"})
                    .to_string(),
                &crate::ExecutionContext::new(None).unwrap(),
            )
            .unwrap();
            assert_eq!(
                document
                    .extract(
                        &contract,
                        &crate::Limits::default().execution(),
                        &crate::ExecutionContext::new((crate::Limits::default()).time_limit)
                            .unwrap()
                    )
                    .unwrap(),
                Value::List(
                    expected
                        .into_iter()
                        .map(|id| Value::String(id.into()))
                        .collect()
                ),
                "{selector}"
            );
        }
    }
}
