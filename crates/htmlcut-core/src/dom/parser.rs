// SPDX-License-Identifier: MPL-2.0 AND ISC
//! Construction-time resource boundary for the maintained parser.

use std::borrow::Cow;
use std::cell::{Cell, Ref};

use ego_tree::NodeId;
use html5ever::tree_builder::{ElementFlags, NodeOrText, QuirksMode, TreeSink};
use html5ever::{Attribute, QualName, driver};
use tendril::{StrTendril, TendrilSink};

use crate::dom::Html;
#[path = "tree_sink.rs"]
mod tree_sink;
use crate::dom::Node;
use tree_sink::HtmlTreeSink;

#[cfg(test)]
#[path = "bounded_tests.rs"]
mod tests;
use crate::dom::Element;

/// Finite construction limits, including parser attachment/reparenting work.
#[derive(Clone, Copy, Debug)]
pub struct ParseLimits {
    /// Maximum constructed elements, excluding one fixed internal stop sentinel.
    pub elements: u32,
    /// Maximum allocated nodes, including the document and internal stop sentinel.
    pub nodes: u32,
    /// Maximum depth from the document root at zero.
    pub depth: u32,
    /// Maximum construction/attachment accounting work.
    pub work: u32,
}

/// Sticky failure; a bounded parser never returns its partial document.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParseLimitExceeded {
    /// Element allocation would exceed its limit.
    Elements,
    /// Node allocation would exceed its limit.
    Nodes,
    /// Attachment/reparenting would exceed the permitted depth.
    Depth,
    /// Construction accounting exhausted its work budget.
    Work,
}

impl Html {
    /// Parses without allocating governed nodes or attaching deeper trees past their limits.
    /// The parser stops at the first failing token and discards the partial DOM.
    pub fn parse_document_bounded(
        source: &str,
        limits: ParseLimits,
    ) -> Result<Self, ParseLimitExceeded> {
        if limits.nodes < 2 {
            return Err(ParseLimitExceeded::Nodes);
        }
        let sink = BoundedSink::new(limits);
        let mut parser = driver::parse_document(sink, Default::default());
        let mut offset = 0;
        while offset < source.len() {
            let mut end = (offset + 4096).min(source.len());
            while !source.is_char_boundary(end) {
                end -= 1;
            }
            parser.process(source[offset..end].into());
            if let Some(error) = parser.tokenizer.sink.sink.failure.get() {
                return Err(error);
            }
            offset = end;
        }
        parser.finish()
    }
}

struct BoundedSink {
    inner: HtmlTreeSink,
    limits: ParseLimits,
    failure: Cell<Option<ParseLimitExceeded>>,
    elements: Cell<u32>,
    nodes: Cell<u32>,
    remaining: Cell<u32>,
    sentinel: NodeId,
}

impl BoundedSink {
    fn new(limits: ParseLimits) -> Self {
        let mut html = Html::new_document();
        // After a failure, trait methods may still be called within the current tree-builder
        // token. A fixed valid element handle lets these calls finish without allocating or
        // mutating anything; the tokenizer then stops before processing another token.
        let sentinel = html
            .tree
            .orphan(Node::Element(Element::new(
                QualName::new(None, html5ever::ns!(html), html5ever::local_name!("html")),
                Vec::new(),
            )))
            .id();
        Self {
            inner: HtmlTreeSink::new(html),
            limits,
            failure: Cell::new(None),
            elements: Cell::new(0),
            nodes: Cell::new(2),
            remaining: Cell::new(limits.work),
            sentinel,
        }
    }
    fn fail(&self, failure: ParseLimitExceeded) -> bool {
        if self.failure.get().is_none() {
            self.failure.set(Some(failure));
        }
        false
    }
    fn work(&self) -> bool {
        if self.stop_requested() {
            return false;
        }
        match self.remaining.get().checked_sub(1) {
            Some(remaining) => {
                self.remaining.set(remaining);
                true
            }
            None => self.fail(ParseLimitExceeded::Work),
        }
    }
    fn allocate(&self, nodes: u32, elements: u32) -> bool {
        if !self.work() {
            return false;
        }
        let Some(node_count) = self.nodes.get().checked_add(nodes) else {
            return self.fail(ParseLimitExceeded::Nodes);
        };
        let Some(element_count) = self.elements.get().checked_add(elements) else {
            return self.fail(ParseLimitExceeded::Elements);
        };
        if node_count > self.limits.nodes {
            return self.fail(ParseLimitExceeded::Nodes);
        }
        if element_count > self.limits.elements {
            return self.fail(ParseLimitExceeded::Elements);
        }
        self.nodes.set(node_count);
        self.elements.set(element_count);
        true
    }
    fn parent_depth(&self, parent: NodeId) -> Option<u32> {
        let html = self.inner.0.borrow();
        let mut depth = 0_u32;
        for _ in html.tree.get(parent).unwrap().ancestors() {
            if !self.work() {
                return None;
            }
            depth += 1;
            if depth > self.limits.depth {
                self.fail(ParseLimitExceeded::Depth);
                return None;
            }
        }
        Some(depth)
    }
    fn attach(&self, parent: NodeId, child: &NodeOrText<NodeId>) -> bool {
        if !self.work() {
            return false;
        }
        let Some(parent_depth) = self.parent_depth(parent) else {
            return false;
        };
        let html = self.inner.0.borrow();
        let height = match child {
            NodeOrText::AppendNode(id) => {
                let root = html.tree.get(*id).unwrap();
                let mut height = 0_u32;
                let mut depth = 0_u32;
                for edge in root.traverse() {
                    if !self.work() {
                        return false;
                    }
                    match edge {
                        ego_tree::iter::Edge::Open(_) => {
                            height = height.max(depth);
                            depth += 1;
                        }
                        ego_tree::iter::Edge::Close(_) => {
                            depth -= 1;
                        }
                    }
                }
                height
            }
            NodeOrText::AppendText(_) => 0,
        };
        if parent_depth.saturating_add(1).saturating_add(height) > self.limits.depth {
            return self.fail(ParseLimitExceeded::Depth);
        }
        true
    }
}

impl TreeSink for BoundedSink {
    type Output = Result<Html, ParseLimitExceeded>;
    type Handle = NodeId;
    type ElemName<'a> = Ref<'a, QualName>;
    fn stop_requested(&self) -> bool {
        self.failure.get().is_some()
    }
    fn finish(self) -> Self::Output {
        if let Some(error) = self.failure.get() {
            return Err(error);
        }
        Ok(self.inner.finish())
    }

    fn parse_error(&self, _message: Cow<'static, str>) {
        // Parser warnings are not requested output. Charge their work without retaining an
        // attacker-controlled diagnostics array; resource errors remain the typed failure.
        self.work();
    }
    fn set_quirks_mode(&self, mode: QuirksMode) {
        if self.work() {
            self.inner.set_quirks_mode(mode);
        }
    }
    fn get_document(&self) -> NodeId {
        self.inner.get_document()
    }
    fn same_node(&self, x: &NodeId, y: &NodeId) -> bool {
        self.inner.same_node(x, y)
    }
    fn elem_name<'a>(&'a self, target: &'a NodeId) -> Ref<'a, QualName> {
        self.inner.elem_name(target)
    }
    fn create_element(&self, name: QualName, attrs: Vec<Attribute>, flags: ElementFlags) -> NodeId {
        // Delegate accounting follows the sink's actual allocation rule, even if a caller
        // supplies incomplete ElementFlags. Template hosts allocate a fragment as well.
        let nodes = if name.expanded() == html5ever::expanded_name!(html "template") {
            2
        } else {
            1
        };
        if self.allocate(nodes, 1) {
            self.inner.create_element(name, attrs, flags)
        } else {
            self.sentinel
        }
    }
    fn create_comment(&self, text: StrTendril) -> NodeId {
        if self.allocate(1, 0) {
            self.inner.create_comment(text)
        } else {
            self.sentinel
        }
    }
    fn create_pi(&self, target: StrTendril, data: StrTendril) -> NodeId {
        if self.allocate(1, 0) {
            self.inner.create_pi(target, data)
        } else {
            self.sentinel
        }
    }
    fn append_doctype_to_document(
        &self,
        name: StrTendril,
        public_id: StrTendril,
        system_id: StrTendril,
    ) {
        if self.stop_requested() {
            return;
        }
        if self.limits.depth == 0 {
            self.fail(ParseLimitExceeded::Depth);
            return;
        }
        if self.allocate(1, 0) {
            self.inner
                .append_doctype_to_document(name, public_id, system_id);
        }
    }
    fn append(&self, parent: &NodeId, child: NodeOrText<NodeId>) {
        if !self.attach(*parent, &child) {
            return;
        }
        if matches!(child, NodeOrText::AppendText(_)) {
            let concat = self
                .inner
                .0
                .borrow()
                .tree
                .get(*parent)
                .unwrap()
                .last_child()
                .is_some_and(|node| node.value().is_text());
            if !concat && !self.allocate(1, 0) {
                return;
            }
        }
        self.inner.append(parent, child);
    }
    fn append_before_sibling(&self, sibling: &NodeId, child: NodeOrText<NodeId>) {
        if self.stop_requested() {
            return;
        }
        let parent = self
            .inner
            .0
            .borrow()
            .tree
            .get(*sibling)
            .unwrap()
            .parent()
            .map(|p| p.id());
        if let Some(parent) = parent {
            if !self.attach(parent, &child) {
                return;
            }
            if matches!(child, NodeOrText::AppendText(_)) {
                let concat = self
                    .inner
                    .0
                    .borrow()
                    .tree
                    .get(*sibling)
                    .unwrap()
                    .prev_sibling()
                    .is_some_and(|node| node.value().is_text());
                if !concat && !self.allocate(1, 0) {
                    return;
                }
            }
            self.inner.append_before_sibling(sibling, child);
        }
    }
    fn remove_from_parent(&self, target: &NodeId) {
        if self.work() {
            self.inner.remove_from_parent(target);
        }
    }
    fn reparent_children(&self, node: &NodeId, new_parent: &NodeId) {
        if self.stop_requested() {
            return;
        }
        let children = self
            .inner
            .0
            .borrow()
            .tree
            .get(*node)
            .unwrap()
            .children()
            .map(|n| n.id())
            .collect::<Vec<_>>();
        for child in children {
            if !self.attach(*new_parent, &NodeOrText::AppendNode(child)) {
                return;
            }
        }
        if self.work() {
            self.inner.reparent_children(node, new_parent);
        }
    }
    fn add_attrs_if_missing(&self, target: &NodeId, attrs: Vec<Attribute>) {
        if self.work() {
            self.inner.add_attrs_if_missing(target, attrs);
        }
    }
    fn get_template_contents(&self, target: &NodeId) -> NodeId {
        if self.stop_requested() {
            self.sentinel
        } else {
            self.inner.get_template_contents(target)
        }
    }
    fn mark_script_already_started(&self, _node: &NodeId) {
        self.work();
    }
    fn append_based_on_parent_node(
        &self,
        element: &NodeId,
        prev_element: &NodeId,
        child: NodeOrText<NodeId>,
    ) {
        if self.stop_requested() {
            return;
        }
        let has_parent = self
            .inner
            .0
            .borrow()
            .tree
            .get(*element)
            .unwrap()
            .parent()
            .is_some();
        if has_parent {
            self.append_before_sibling(element, child);
        } else {
            self.append(prev_element, child);
        }
    }
}

#[cfg(test)]
pub(super) fn parse_unbounded(source: &str) -> Html {
    driver::parse_document(HtmlTreeSink::new(Html::new_document()), Default::default()).one(source)
}
