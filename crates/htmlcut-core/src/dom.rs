// SPDX-License-Identifier: MPL-2.0 AND ISC
//! Immutable product DOM and the public ego-tree boundary.
use ego_tree::{NodeRef, Tree};
use html5ever::{Attribute, QualName};
use std::ops::Deref;

pub(crate) mod parser;
pub(crate) mod selector;
mod serialization;
pub(crate) use selector::Selector;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Node {
    Document,
    Fragment,
    Doctype(Doctype),
    Comment(String),
    Text(Text),
    Element(Element),
    ProcessingInstruction(ProcessingInstruction),
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Doctype {
    pub name: String,
    pub public_id: String,
    pub system_id: String,
}
impl Doctype {
    pub fn name(&self) -> &str {
        &self.name
    }
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ProcessingInstruction {
    pub target: String,
    pub data: String,
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Text {
    pub text: String,
}
impl Deref for Text {
    type Target = str;
    fn deref(&self) -> &str {
        &self.text
    }
}
impl Node {
    #[cfg(test)]
    pub fn is_processing_instruction(&self) -> bool {
        matches!(self, Self::ProcessingInstruction(_))
    }
    pub fn is_document(&self) -> bool {
        matches!(self, Self::Document)
    }
    pub fn is_text(&self) -> bool {
        matches!(self, Self::Text(_))
    }
    pub fn is_element(&self) -> bool {
        matches!(self, Self::Element(_))
    }
    pub fn as_element(&self) -> Option<&Element> {
        if let Self::Element(e) = self {
            Some(e)
        } else {
            None
        }
    }
    #[cfg(test)]
    pub fn as_text(&self) -> Option<&Text> {
        if let Self::Text(e) = self {
            Some(e)
        } else {
            None
        }
    }
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Element {
    pub name: QualName,
    pub attrs: Vec<(QualName, String)>,
}
impl Element {
    pub fn new(name: QualName, attrs: Vec<Attribute>) -> Self {
        let mut attrs = attrs
            .into_iter()
            .map(|a| (a.name, a.value.to_string()))
            .collect::<Vec<_>>();
        attrs.sort_unstable_by(|a, b| a.0.cmp(&b.0));
        Self { name, attrs }
    }
    pub fn name(&self) -> &str {
        &self.name.local
    }
    pub fn attr(&self, name: &str) -> Option<&str> {
        let name = QualName::new(None, html5ever::ns!(), name.into());
        self.attrs
            .binary_search_by(|a| a.0.cmp(&name))
            .ok()
            .map(|i| self.attrs[i].1.as_str())
    }
    pub fn attrs(&self) -> impl Iterator<Item = (&str, &str)> {
        self.attrs
            .iter()
            .map(|(n, v)| (n.local.as_ref(), v.as_str()))
    }
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Html {
    pub tree: Tree<Node>,
}
impl Html {
    pub fn new_document() -> Self {
        Self {
            tree: Tree::new(Node::Document),
        }
    }
    #[cfg(test)]
    pub fn parse_document(source: &str) -> Self {
        parser::parse_unbounded(source)
    }
    #[cfg(test)]
    pub fn select<'a>(&'a self, selector: &Selector) -> std::vec::IntoIter<ElementRef<'a>> {
        let budget = crate::budget::WorkBudget::new(u32::MAX);
        let mut matcher = selector.budgeted(self, None, &budget).unwrap();
        self.tree
            .root()
            .descendants()
            .filter_map(ElementRef::wrap)
            .filter(|e| matcher.matches(e).unwrap())
            .collect::<Vec<_>>()
            .into_iter()
    }
    #[cfg(test)]
    pub fn html(&self) -> String {
        serialization::document_html(self)
    }
}
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
    pub fn attr(&self, name: &str) -> Option<&'a str> {
        self.value().attr(name)
    }
    #[cfg(test)]
    pub fn text(&self) -> impl Iterator<Item = &'a str> {
        self.descendants()
            .filter_map(|n| n.value().as_text().map(|t| t.text.as_str()))
    }
    #[cfg(test)]
    pub fn inner_html(&self) -> String {
        let mut bytes = Vec::new();
        self.write_filtered_html(
            &mut bytes,
            false,
            &Default::default(),
            &crate::budget::WorkBudget::new(u32::MAX),
        )
        .unwrap();
        String::from_utf8(bytes).unwrap()
    }
    #[cfg(test)]
    pub fn html(&self) -> String {
        let mut bytes = Vec::new();
        self.write_filtered_html(
            &mut bytes,
            true,
            &Default::default(),
            &crate::budget::WorkBudget::new(u32::MAX),
        )
        .unwrap();
        String::from_utf8(bytes).unwrap()
    }
}
impl<'a> Deref for ElementRef<'a> {
    type Target = NodeRef<'a, Node>;
    fn deref(&self) -> &Self::Target {
        &self.node
    }
}
