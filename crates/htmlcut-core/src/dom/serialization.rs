// SPDX-License-Identifier: MPL-2.0 AND ISC
//! Streaming original-DOM serialization with immutable subtree exclusions.

use std::collections::HashSet;
use std::io::{self, Write};

use crate::budget::WorkBudget;
use ego_tree::{NodeId, iter::Edge};
use html5ever::serialize::{Serialize, SerializeOpts, Serializer, TraversalScope, serialize};

use crate::dom::{ElementRef, Node};

impl ElementRef<'_> {
    /// Streams parsed HTML into a caller-bounded writer without cloning or mutating the DOM.
    pub fn write_filtered_html<W: Write>(
        &self,
        writer: W,
        include_root: bool,
        excluded: &HashSet<NodeId>,
        budget: &WorkBudget,
    ) -> io::Result<()> {
        serialize(
            writer,
            &Filtered {
                root: *self,
                excluded,
                budget,
            },
            SerializeOpts {
                scripting_enabled: false,
                traversal_scope: if include_root {
                    TraversalScope::IncludeNode
                } else {
                    TraversalScope::ChildrenOnly(None)
                },
                create_missing_parent: false,
            },
        )
    }
}

struct Filtered<'a, 'b> {
    root: ElementRef<'a>,
    excluded: &'b HashSet<NodeId>,
    budget: &'b WorkBudget,
}

impl Serialize for Filtered<'_, '_> {
    fn serialize<S: Serializer>(
        &self,
        serializer: &mut S,
        scope: TraversalScope,
    ) -> io::Result<()> {
        let mut skipped = 0_u32;
        for edge in self.root.traverse() {
            if !self.budget.consume() {
                return Err(io::Error::other("Selector work limit exceeded."));
            }
            match edge {
                Edge::Open(node) => {
                    if skipped > 0 || self.excluded.contains(&node.id()) {
                        skipped += 1;
                        continue;
                    }
                    if node.id() == self.root.id() && scope == TraversalScope::ChildrenOnly(None) {
                        continue;
                    }
                    match node.value() {
                        Node::Doctype(value) => serializer.write_doctype(value.name())?,
                        Node::Comment(value) => serializer.write_comment(value)?,
                        Node::Text(value) => serializer.write_text(value)?,
                        Node::Element(value) => serializer.start_elem(
                            value.name.clone(),
                            value.attrs.iter().map(|(key, value)| (key, &value[..])),
                        )?,
                        _ => (),
                    }
                }
                Edge::Close(node) => {
                    if skipped > 0 {
                        skipped -= 1;
                        continue;
                    }
                    if node.id() == self.root.id() && scope == TraversalScope::ChildrenOnly(None) {
                        continue;
                    }
                    if let Some(value) = node.value().as_element() {
                        serializer.end_elem(value.name.clone())?;
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
pub(super) fn document_html(document: &crate::dom::Html) -> String {
    struct Document<'a>(&'a crate::dom::Html);
    impl Serialize for Document<'_> {
        fn serialize<S: Serializer>(
            &self,
            serializer: &mut S,
            _scope: TraversalScope,
        ) -> io::Result<()> {
            for edge in self.0.tree.root().traverse() {
                match edge {
                    Edge::Open(n) => match n.value() {
                        Node::Doctype(v) => serializer.write_doctype(v.name())?,
                        Node::Comment(v) => serializer.write_comment(v)?,
                        Node::Text(v) => serializer.write_text(v)?,
                        Node::Element(v) => serializer
                            .start_elem(v.name.clone(), v.attrs.iter().map(|(n, v)| (n, &v[..])))?,
                        _ => (),
                    },
                    Edge::Close(n) => {
                        if let Some(e) = n.value().as_element() {
                            serializer.end_elem(e.name.clone())?;
                        }
                    }
                }
            }
            Ok(())
        }
    }
    let mut bytes = Vec::new();
    serialize(
        &mut bytes,
        &Document(document),
        SerializeOpts {
            scripting_enabled: false,
            traversal_scope: TraversalScope::IncludeNode,
            create_missing_parent: false,
        },
    )
    .unwrap();
    String::from_utf8(bytes).unwrap()
}
