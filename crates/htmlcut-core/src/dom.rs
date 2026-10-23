// SPDX-License-Identifier: MPL-2.0
//! Private admitted tree and immutable selector view.
use crate::{Error, ParseOptions, stop::Clock};
use ego_tree::{NodeId, NodeRef, Tree};
use html5ever::{
    Attribute, QualName,
    tendril::StrTendril,
    tree_builder::{ElementFlags, NodeOrText, QuirksMode, TreeSink},
};
use std::{cell::RefCell, collections::HashMap, ops::Deref, rc::Rc};
mod matching;
#[cfg(test)]
use crate::TestOptions;

#[derive(Debug)]
pub(crate) struct Element {
    pub(crate) name: QualName,
    pub(crate) attrs: Vec<(QualName, String)>,
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
    Text(String),
    Comment(String),
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
            text.as_bytes(),
            &crate::ParseOptions::default(),
            &crate::Stop::none(),
        )
        .unwrap()
        .html
    }
    #[cfg(test)]
    pub(crate) fn html(&self) -> String {
        let limits = TestOptions::default();
        let clock = Clock::new(limits.time_limit).unwrap();
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
            self.error = Some(Error::LimitExceeded {
                limit: crate::Limit::Nodes,
                bound: (self.maximum).into(),
            });
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
            if d > crate::bounds::MAX_DOM_DEPTH {
                self.error = Some(Error::LimitExceeded {
                    limit: crate::Limit::Depth,
                    bound: (crate::bounds::MAX_DOM_DEPTH).into(),
                });
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
                    value.push_str(&text);
                    return;
                }
                if let Some(id) = self.allocate(|| Node::Text(text.to_string()))
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
    pub(crate) fn new(limits: &ParseOptions, clock: Clock) -> Self {
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
    fn unnamed(&self, node: impl FnOnce() -> Node) -> Handle {
        let id = self.inner.borrow_mut().allocate(node);
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
        let id = b.allocate(|| {
            Node::Element(Element {
                name: name.clone(),
                attrs: Vec::new(),
            })
        });
        if let Some(id) = id {
            for attribute in attrs {
                if !b.check() {
                    break;
                }
                if let Node::Element(element) = b.html.tree.get_mut(id).unwrap().value() {
                    element
                        .attrs
                        .push((attribute.name, attribute.value.to_string()));
                }
            }
        }
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
        self.unnamed(|| Node::Comment(text.to_string()))
    }
    fn create_pi(&self, _: StrTendril, _: StrTendril) -> Handle {
        self.unnamed(|| Node::ProcessingInstruction)
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
        let h = self.unnamed(|| Node::Doctype);
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
        if let Some(id) = target.0.id {
            for attribute in attrs {
                if !b.check() {
                    break;
                }
                if let Node::Element(element) = b.html.tree.get_mut(id).unwrap().value()
                    && !element
                        .attrs
                        .iter()
                        .any(|(name, _)| *name == attribute.name)
                {
                    element
                        .attrs
                        .push((attribute.name, attribute.value.to_string()));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::test_support::{DocumentFixture, TestOptions, parse_fixture};
    use crate::{Contract, Value};

    fn reading(html: &str, select: &str, read: &str) -> Value {
        let document = parse_fixture(
            DocumentFixture::new(html.as_bytes()),
            &TestOptions::default().preparation(),
            &crate::stop::Clock::new(TestOptions::default().time_limit).unwrap(),
        )
        .unwrap();
        let contract = Contract::from_json(
            &serde_json::json!({"select": select, "read": read}).to_string(),
            &crate::stop::Clock::new(None).unwrap(),
        )
        .unwrap();
        document
            .extract(
                &contract,
                crate::ExportFormat::Json,
                &crate::test_support::TestOptions::default().execution(),
                &crate::stop::Clock::new((crate::test_support::TestOptions::default()).time_limit)
                    .unwrap(),
            )
            .unwrap()
    }

    #[test]
    fn callback_refusal_stops_allocation_but_retains_parser_handle_facts() {
        use super::{Clock, TreeBuilder};
        use html5ever::{
            QualName, local_name, ns,
            tree_builder::{ElementFlags, TreeSink},
        };
        let limits = TestOptions {
            max_nodes: 4096,
            time_limit: Some(std::time::Duration::ZERO),
            ..TestOptions::default()
        };
        let sink = TreeBuilder::new(
            &limits.preparation(),
            Clock::new(limits.time_limit).unwrap(),
        );
        for _ in 0..1024 {
            sink.create_comment("before-refusal".into());
        }
        assert_eq!(
            serde_json::to_value(sink.refusal().unwrap()).unwrap()["limit"].as_str(),
            Some("time")
        );
        let admitted = sink.node_count();
        assert!(admitted <= 1024);
        let first = sink.create_element(
            QualName::new(None, ns!(html), local_name!("div")),
            vec![],
            ElementFlags::default(),
        );
        let second = sink.create_element(
            QualName::new(None, ns!(html), local_name!("span")),
            vec![],
            ElementFlags::default(),
        );
        assert!(sink.same_node(&first, &first.clone()));
        assert!(!sink.same_node(&first, &second));
        assert_eq!(sink.elem_name(&first).local, local_name!("div"));
        assert_eq!(sink.elem_name(&second).local, local_name!("span"));
        let template = sink.create_element(
            QualName::new(None, ns!(html), local_name!("template")),
            vec![],
            ElementFlags::default(),
        );
        let contents = sink.get_template_contents(&template);
        assert!(!sink.same_node(&template, &contents));
        let mut flags = ElementFlags::default();
        flags.mathml_annotation_xml_integration_point = true;
        let mathml = sink.create_element(
            QualName::new(None, ns!(mathml), local_name!("annotation-xml")),
            vec![],
            flags,
        );
        assert!(sink.is_mathml_annotation_xml_integration_point(&mathml));
        assert!(!sink.is_mathml_annotation_xml_integration_point(&first));
        for _ in 0..128 {
            sink.create_comment("after-refusal".into());
        }
        assert_eq!(sink.node_count(), admitted);
        assert_eq!(
            serde_json::to_value(sink.finish().unwrap_err()).unwrap()["limit"].as_str(),
            Some("time")
        );
    }

    #[test]
    fn text_leaf_depth_is_admitted_at_the_boundary_and_refused_beyond_it() {
        for (nested_divs, expected) in [(1021, None), (1022, Some("depth"))] {
            let html = format!(
                "<html><head></head><body>{}leaf{}",
                "<div>".repeat(nested_divs),
                "</div>".repeat(nested_divs)
            );
            let result = parse_fixture(
                DocumentFixture::new(html.as_bytes()),
                &TestOptions::default().preparation(),
                &crate::stop::Clock::new(TestOptions::default().time_limit).unwrap(),
            );
            match expected {
                None => assert!(result.is_ok()),
                Some(limit) => assert_eq!(
                    serde_json::to_value(result.err().unwrap()).unwrap()["limit"].as_str(),
                    Some(limit)
                ),
            }
        }
    }

    #[test]
    fn attaching_or_reparenting_existing_subtrees_checks_every_descendant_depth() {
        use super::{Clock, TreeBuilder};
        use html5ever::{
            QualName, local_name, ns,
            tree_builder::{ElementFlags, NodeOrText, TreeSink},
        };
        for (parent_depth, reparent) in [(1023, false), (1024, true)] {
            let limits = TestOptions {
                max_nodes: 4096,
                time_limit: None,
                ..TestOptions::default()
            };
            let sink = TreeBuilder::new(
                &limits.preparation(),
                Clock::new(limits.time_limit).unwrap(),
            );
            let element = || {
                sink.create_element(
                    QualName::new(None, ns!(html), local_name!("div")),
                    vec![],
                    ElementFlags::default(),
                )
            };
            let mut parent = sink.get_document();
            for _ in 0..parent_depth {
                let child = element();
                sink.append(&parent, NodeOrText::AppendNode(child.clone()));
                parent = child;
            }
            let branch = element();
            let leaf = element();
            sink.append(&branch, NodeOrText::AppendNode(leaf));
            if reparent {
                sink.reparent_children(&branch, &parent);
            } else {
                sink.append(&parent, NodeOrText::AppendNode(branch));
            }
            let Err(error) = sink.finish() else {
                panic!("an existing descendant beyond depth1024 must refuse");
            };
            assert_eq!(
                serde_json::to_value(&error).unwrap()["limit"].as_str(),
                Some("depth")
            );
            assert_eq!(
                serde_json::to_value(&error).unwrap().get("bound").cloned(),
                Some(1024.into())
            );
        }
    }

    #[test]
    fn malformed_formatting_reconstructs_literal_tree_instead_of_reusing_live_nodes() {
        assert_eq!(
            reading("<p><b>bold<i>both</b>italic</i>end", "p", "outer-html"),
            Value::String("<p><b>bold<i>both</i></b><i>italic</i>end</p>".into())
        );
    }

    #[test]
    fn adoption_agency_reparents_existing_block_children_into_reconstructed_formatting() {
        assert_eq!(
            reading("<b>one<p>two</b>three", "body", "outer-html"),
            Value::String("<body><b>one</b><p><b>two</b>three</p></body>".into())
        );
    }

    #[test]
    fn table_foster_text_precedes_table_and_keeps_authored_order() {
        assert_eq!(
            reading("<table>a<tr><td>b</td></tr>c</table>", "body", "outer-html"),
            Value::String(
                "<body>ac<table><tbody><tr><td>b</td></tr></tbody></table></body>".into()
            )
        );
    }

    #[test]
    fn repeated_root_tags_add_only_missing_attributes() {
        let html = "<html id=first><body><html id=second class=added><body data-body=yes>";
        assert_eq!(
            reading(html, "html", "attr:id"),
            Value::String("first".into())
        );
        assert_eq!(
            reading(html, "html", "attr:class"),
            Value::String("added".into())
        );
        assert_eq!(
            reading(html, "body", "attr:data-body"),
            Value::String("yes".into())
        );
    }

    #[test]
    fn ordinary_attributes_do_not_alias_foreign_namespace_attributes() {
        let html = "<svg><a id=foreign xlink:href=foreign></a><a id=plain href=plain></a></svg>";
        assert_eq!(
            reading(html, "a[href]", "attr:id"),
            Value::String("plain".into())
        );
        assert_eq!(
            reading(html, "a[*|href=foreign]", "attr:id"),
            Value::String("foreign".into())
        );
    }
}
