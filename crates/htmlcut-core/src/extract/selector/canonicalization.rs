//! Detached selected-subtree canonicalization for selector comparison evidence.

use ego_tree::NodeId;
use scraper::{ElementRef, Html, Node};

use crate::document::{extract_element_plain_text, render_element_as_text};

use super::SelectorDomCanonicalization;

pub(super) fn project_canonicalized_selected_clone(
    document: &mut Html,
    selected_node_id: NodeId,
    canonicalization: &SelectorDomCanonicalization,
    whitespace: crate::WhitespaceMode,
) -> (String, String) {
    let detached_clone_id = document
        .tree
        .get_mut(selected_node_id)
        .expect("selected node IDs must survive an HTML clone")
        .clone_subtree()
        .id();
    canonicalize_detached_subtree(document, detached_clone_id, canonicalization);
    let detached_clone = document
        .tree
        .get(detached_clone_id)
        .and_then(ElementRef::wrap)
        .expect("cloned selected element must remain an element");
    (
        render_element_as_text(&detached_clone, whitespace),
        extract_element_plain_text(&detached_clone, whitespace),
    )
}

pub(super) fn canonicalize_detached_subtree(
    document: &mut Html,
    detached_root_id: NodeId,
    canonicalization: &SelectorDomCanonicalization,
) {
    let mut node_ids = vec![detached_root_id];
    node_ids.extend(
        document
            .tree
            .get(detached_root_id)
            .expect("detached clone root must survive canonicalization")
            .descendants()
            .map(|node| node.id()),
    );
    for node_id in node_ids {
        let remove_node = document
            .tree
            .get(node_id)
            .is_some_and(|node| match node.value() {
                Node::Text(text) => {
                    canonicalization.strip_whitespace_nodes && text.trim().is_empty()
                }
                _ => false,
            });
        if remove_node {
            document
                .tree
                .get_mut(node_id)
                .expect("cloned node must survive canonicalization")
                .detach();
            continue;
        }
        let mut node = document
            .tree
            .get_mut(node_id)
            .expect("cloned node must survive canonicalization");
        let Node::Element(element) = node.value() else {
            continue;
        };
        if !canonicalization.ignored_attributes.is_empty() {
            element.retain_attributes(|name| !canonicalization.ignores_attribute(name));
        }
    }
}
