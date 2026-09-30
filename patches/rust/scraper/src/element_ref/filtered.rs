//! Streaming original-DOM serialization with immutable subtree exclusions.

use std::collections::HashSet;
use std::io::{self, Write};

use ego_tree::{NodeId, iter::Edge};
use html5ever::serialize::{Serialize, SerializeOpts, Serializer, TraversalScope, serialize};
use selectors::work_budget::SelectorWorkBudget;

use crate::{ElementRef, Node};

impl ElementRef<'_> {
    /// Streams parsed HTML into a caller-bounded writer without cloning or mutating the DOM.
    pub fn write_filtered_html<W: Write>(
        &self,
        writer: W,
        include_root: bool,
        excluded: &HashSet<NodeId>,
        budget: &SelectorWorkBudget,
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
    budget: &'b SelectorWorkBudget,
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
