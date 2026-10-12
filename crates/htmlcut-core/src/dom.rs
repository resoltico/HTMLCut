// SPDX-License-Identifier: MPL-2.0
//! Maintained upstream DOM with bounded product serialization.
use ego_tree::NodeRef;
pub(crate) use scraper::Node;
use std::ops::Deref;
#[derive(Debug)]
pub(crate) struct Html(scraper::Html);
impl Deref for Html {
    type Target = scraper::Html;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl Html {
    pub fn parse_document(source: &str) -> Self {
        Self(scraper::Html::parse_document(source))
    }
    #[cfg(test)]
    pub fn select(&self, selector: &Selector) -> std::vec::IntoIter<ElementRef<'_>> {
        self.tree
            .root()
            .descendants()
            .filter_map(ElementRef::wrap)
            .filter(|e| selector.test_matches(e))
            .collect::<Vec<_>>()
            .into_iter()
    }
}

pub(crate) use scraper::node::Element;
pub(crate) mod parser;
pub(crate) mod selector;
mod serialization;
pub(crate) use selector::Selector;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ElementRef<'a> {
    pub node: NodeRef<'a, Node>,
}
impl<'a> ElementRef<'a> {
    pub fn wrap(node: NodeRef<'a, Node>) -> Option<Self> {
        node.value().is_element().then_some(Self { node })
    }
    pub fn value(&self) -> &'a Element {
        self.node.value().as_element().unwrap()
    }
    #[cfg(test)]
    pub fn text(&self) -> impl Iterator<Item = &'a str> {
        self.descendants()
            .filter_map(|n| n.value().as_text().map(|t| t.text.as_ref()))
    }
    pub fn attr(&self, name: &str) -> Option<&'a str> {
        self.value().attr(name)
    }
}
impl<'a> Deref for ElementRef<'a> {
    type Target = NodeRef<'a, Node>;
    fn deref(&self) -> &Self::Target {
        &self.node
    }
}
