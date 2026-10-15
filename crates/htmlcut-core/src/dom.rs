// SPDX-License-Identifier: MPL-2.0
//! Private admitted tree and immutable selector view.
use crate::{Error, Limits, limits::Clock};
use ego_tree::{NodeId, NodeRef, Tree};
use html5ever::{
    Attribute, QualName,
    tendril::StrTendril,
    tree_builder::{ElementFlags, NodeOrText, QuirksMode, TreeSink},
};
use std::{cell::RefCell, collections::HashMap, ops::Deref, rc::Rc};
mod matching;

#[derive(Debug)]
pub(crate) struct Element {
    pub(crate) name: QualName,
    pub(crate) attrs: Vec<(QualName, StrTendril)>,
}
impl Element {
    pub(crate) fn name(&self) -> &str {
        &self.name.local
    }
    pub(crate) fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(key, _)| key.ns == html5ever::ns!() && &*key.local == name)
            .map(|(_, v)| v.as_ref())
    }
    pub(crate) fn id(&self) -> Option<&str> {
        self.attr("id")
    }
    pub(crate) fn classes(&self) -> impl Iterator<Item = &str> {
        self.attr("class")
            .unwrap_or("")
            .split([' ', '\t', '\r', '\n', '\x0c'])
            .filter(|s| !s.is_empty())
    }
    pub(crate) fn has_class(&self, name: &str, case: selectors::attr::CaseSensitivity) -> bool {
        self.classes()
            .any(|c| case.eq(c.as_bytes(), name.as_bytes()))
    }
}
#[derive(Debug)]
pub(crate) enum Node {
    Document,
    Fragment,
    Element(Element),
    Text(StrTendril),
    Comment(StrTendril),
    Doctype,
    ProcessingInstruction,
}
impl Node {
    pub(crate) fn as_element(&self) -> Option<&Element> {
        if let Self::Element(e) = self {
            Some(e)
        } else {
            None
        }
    }
    pub(crate) fn is_element(&self) -> bool {
        matches!(self, Self::Element(_))
    }
    pub(crate) fn is_text(&self) -> bool {
        matches!(self, Self::Text(_))
    }
    pub(crate) fn is_document(&self) -> bool {
        matches!(self, Self::Document)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ElementRef<'a>(NodeRef<'a, Node>);
impl<'a> ElementRef<'a> {
    pub(crate) fn wrap(node: NodeRef<'a, Node>) -> Option<Self> {
        node.value().is_element().then_some(Self(node))
    }
    #[cfg(test)]
    pub(crate) fn text(&self) -> impl Iterator<Item = &'a str> {
        self.0.descendants().filter_map(|n| {
            if let Node::Text(t) = n.value() {
                Some(&**t)
            } else {
                None
            }
        })
    }
    pub(crate) fn value(&self) -> &'a Element {
        self.0.value().as_element().unwrap()
    }
}
impl<'a> Deref for ElementRef<'a> {
    type Target = NodeRef<'a, Node>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
#[derive(Debug)]
pub(crate) struct Html {
    pub(crate) tree: Tree<Node>,
}
impl Html {
    pub(crate) fn root_element(&self) -> ElementRef<'_> {
        self.tree
            .root()
            .children()
            .find_map(ElementRef::wrap)
            .unwrap()
    }
    #[cfg(test)]
    pub(crate) fn select<'a>(
        &'a self,
        selector: &crate::select::Selector,
    ) -> std::vec::IntoIter<ElementRef<'a>> {
        let mut caches = Default::default();
        let root = self.root_element();
        self.tree
            .root()
            .descendants()
            .filter_map(ElementRef::wrap)
            .filter(move |&e| selector.matches(e, root, &mut caches))
            .collect::<Vec<_>>()
            .into_iter()
    }
    #[cfg(test)]
    pub(crate) fn parse_document(text: &str) -> Self {
        crate::Document::parse(
            crate::Source::new(text.as_bytes()),
            &Limits {
                time_limit: None,
                ..Limits::default()
            },
        )
        .unwrap()
        .html
    }
    #[cfg(test)]
    pub(crate) fn html(&self) -> String {
        let limits = Limits::default();
        let clock = Clock::new(&limits);
        crate::read::read_html(self.root_element(), &clock).unwrap()
    }
}
// Handles retain parser-observable identity/name even after the allocation latch.
#[derive(Clone, Debug)]
pub(crate) struct Handle(Rc<HandleFacts>);
#[derive(Debug)]
struct HandleFacts {
    id: Option<NodeId>,
    name: QualName,
    mathml: bool,
    template: Option<Handle>,
}
struct Builder {
    html: Html,
    depths: HashMap<NodeId, usize>,
    maximum: usize,
    error: Option<Error>,
    clock: Clock,
}
impl Builder {
    fn check(&mut self) -> bool {
        if self.error.is_none() {
            self.error = self.clock.tick().err();
        }
        self.error.is_none()
    }
    fn allocate(&mut self, node: impl FnOnce() -> Node) -> Option<NodeId> {
        if !self.check() {
            return None;
        }
        if self.html.tree.nodes().len() >= self.maximum {
            self.error = Some(Error::limit("nodes", self.maximum));
            return None;
        }
        let id = self.html.tree.orphan(node()).id();
        self.depths.insert(id, 0);
        Some(id)
    }
    // Leaves attach in O(1); existing subtrees update only when their depth changes.
    fn depth(&mut self, id: NodeId, depth: usize) -> bool {
        if !self.check() {
            return false;
        }
        if self.depths[&id] == depth {
            return true;
        }
        let mut pending = vec![(id, depth)];
        let mut updates = Vec::new();
        while let Some((id, d)) = pending.pop() {
            if d > crate::limits::MAX_DOM_DEPTH {
                self.error = Some(Error::limit("depth", crate::limits::MAX_DOM_DEPTH));
                return false;
            }
            if !self.check() {
                return false;
            }
            updates.push((id, d));
            pending.extend(
                self.html
                    .tree
                    .get(id)
                    .unwrap()
                    .children()
                    .map(|n| (n.id(), d + 1)),
            );
        }
        self.depths.extend(updates);
        true
    }
    fn insert(&mut self, parent: NodeId, before: Option<NodeId>, child: NodeOrText<Handle>) {
        if !self.check() {
            return;
        }
        match child {
            NodeOrText::AppendNode(handle) => {
                if let Some(id) = handle.0.id
                    && self.depth(id, self.depths[&parent] + 1)
                {
                    if let Some(before) = before {
                        self.html.tree.get_mut(before).unwrap().insert_id_before(id);
                    } else {
                        self.html.tree.get_mut(parent).unwrap().append_id(id);
                    }
                }
            }
            NodeOrText::AppendText(text) => {
                let previous = if let Some(before) = before {
                    self.html.tree.get(before).unwrap().prev_sibling()
                } else {
                    self.html.tree.get(parent).unwrap().last_child()
                }
                .map(|n| n.id());
                if let Some(id) = previous
                    && let Node::Text(value) = self.html.tree.get_mut(id).unwrap().value()
                {
                    value.push_tendril(&text);
                    return;
                }
                if let Some(id) = self.allocate(|| Node::Text(text))
                    && self.depth(id, self.depths[&parent] + 1)
                {
                    if let Some(before) = before {
                        self.html.tree.get_mut(before).unwrap().insert_id_before(id);
                    } else {
                        self.html.tree.get_mut(parent).unwrap().append_id(id);
                    }
                }
            }
        }
    }
}
pub(crate) struct TreeBuilder {
    inner: RefCell<Builder>,
    document: Handle,
}
impl TreeBuilder {
    pub(crate) fn new(limits: &Limits, clock: Clock) -> Self {
        let tree = Tree::new(Node::Document);
        let id = tree.root().id();
        let document = Self::handle(
            Some(id),
            QualName::new(None, html5ever::ns!(), html5ever::local_name!("html")),
            false,
            None,
        );
        Self {
            inner: RefCell::new(Builder {
                html: Html { tree },
                depths: HashMap::from([(id, 0)]),
                maximum: limits.max_nodes,
                error: None,
                clock,
            }),
            document,
        }
    }
    fn handle(
        id: Option<NodeId>,
        name: QualName,
        mathml: bool,
        template: Option<Handle>,
    ) -> Handle {
        Handle(Rc::new(HandleFacts {
            id,
            name,
            mathml,
            template,
        }))
    }
    fn unnamed(&self, node: Node) -> Handle {
        let id = self.inner.borrow_mut().allocate(|| node);
        Self::handle(id, self.document.0.name.clone(), false, None)
    }
    #[cfg(test)]
    pub(crate) fn node_count(&self) -> usize {
        self.inner.borrow().html.tree.nodes().len()
    }
    pub(crate) fn refusal(&self) -> Option<Error> {
        self.inner.borrow().error.clone()
    }
}
impl TreeSink for TreeBuilder {
    type Handle = Handle;
    type Output = Result<Html, Error>;
    type ElemName<'a> = &'a QualName;
    fn finish(self) -> Result<Html, Error> {
        let b = self.inner.into_inner();
        match b.error {
            Some(e) => Err(e),
            None => Ok(b.html),
        }
    }
    fn parse_error(&self, _: std::borrow::Cow<'static, str>) {}
    fn get_document(&self) -> Handle {
        self.document.clone()
    }
    fn elem_name<'a>(&'a self, target: &'a Handle) -> &'a QualName {
        &target.0.name
    }
    fn same_node(&self, a: &Handle, b: &Handle) -> bool {
        Rc::ptr_eq(&a.0, &b.0)
    }
    fn set_quirks_mode(&self, _: QuirksMode) {}
    fn create_element(&self, name: QualName, attrs: Vec<Attribute>, flags: ElementFlags) -> Handle {
        let template =
            name.ns == html5ever::ns!(html) && name.local == html5ever::local_name!("template");
        let mut b = self.inner.borrow_mut();
        let id = if b.check() {
            b.allocate(|| {
                Node::Element(Element {
                    name: name.clone(),
                    attrs: attrs.into_iter().map(|a| (a.name, a.value)).collect(),
                })
            })
        } else {
            None
        };
        let contents = template.then(|| {
            let fragment = b.allocate(|| Node::Fragment);
            if let (Some(parent), Some(child)) = (id, fragment)
                && b.depth(child, 1)
            {
                b.html.tree.get_mut(parent).unwrap().append_id(child);
            }
            Self::handle(fragment, self.document.0.name.clone(), false, None)
        });
        Self::handle(
            id,
            name,
            flags.mathml_annotation_xml_integration_point,
            contents,
        )
    }
    fn create_comment(&self, text: StrTendril) -> Handle {
        self.unnamed(Node::Comment(text))
    }
    fn create_pi(&self, _: StrTendril, _: StrTendril) -> Handle {
        self.unnamed(Node::ProcessingInstruction)
    }
    fn append(&self, parent: &Handle, child: NodeOrText<Handle>) {
        if let Some(id) = parent.0.id {
            self.inner.borrow_mut().insert(id, None, child);
        }
    }
    fn append_before_sibling(&self, sibling: &Handle, child: NodeOrText<Handle>) {
        let mut b = self.inner.borrow_mut();
        if !b.check() {
            return;
        }
        if let Some(id) = sibling.0.id
            && let Some(parent) = b.html.tree.get(id).unwrap().parent().map(|n| n.id())
        {
            b.insert(parent, Some(id), child);
        }
    }
    fn append_based_on_parent_node(
        &self,
        element: &Handle,
        previous: &Handle,
        child: NodeOrText<Handle>,
    ) {
        let attached = element.0.id.is_some_and(|id| {
            self.inner
                .borrow()
                .html
                .tree
                .get(id)
                .unwrap()
                .parent()
                .is_some()
        });
        if attached {
            self.append_before_sibling(element, child);
        } else {
            self.append(previous, child);
        }
    }
    fn append_doctype_to_document(&self, _name: StrTendril, _: StrTendril, _: StrTendril) {
        let h = self.unnamed(Node::Doctype);
        self.append(&self.document, NodeOrText::AppendNode(h));
    }
    fn get_template_contents(&self, target: &Handle) -> Handle {
        target.0.template.clone().unwrap()
    }
    fn is_mathml_annotation_xml_integration_point(&self, target: &Handle) -> bool {
        target.0.mathml
    }
    fn remove_from_parent(&self, target: &Handle) {
        let mut b = self.inner.borrow_mut();
        if b.check()
            && let Some(id) = target.0.id
        {
            b.html.tree.get_mut(id).unwrap().detach();
            b.depth(id, 0);
        }
    }
    fn reparent_children(&self, node: &Handle, parent: &Handle) {
        let mut b = self.inner.borrow_mut();
        if !b.check() {
            return;
        }
        if let (Some(node), Some(parent)) = (node.0.id, parent.0.id) {
            let depth = b.depths[&parent] + 1;
            let mut child = b.html.tree.get(node).unwrap().first_child().map(|n| n.id());
            while let Some(id) = child {
                child = b.html.tree.get(id).unwrap().next_sibling().map(|n| n.id());
                if !b.depth(id, depth) {
                    return;
                }
            }
            b.html
                .tree
                .get_mut(parent)
                .unwrap()
                .reparent_from_id_append(node);
        }
    }
    fn add_attrs_if_missing(&self, target: &Handle, attrs: Vec<Attribute>) {
        let mut b = self.inner.borrow_mut();
        if b.check()
            && let Some(id) = target.0.id
            && let Node::Element(e) = b.html.tree.get_mut(id).unwrap().value()
        {
            for a in attrs {
                if !e.attrs.iter().any(|(name, _)| *name == a.name) {
                    e.attrs.push((a.name, a.value));
                }
            }
        }
    }
}
