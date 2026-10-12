// SPDX-License-Identifier: MPL-2.0
//! Postparse DOM acceptance checks; physical containment belongs to the CLI worker.
use crate::dom::Html;
use crate::{ExtractionError, PreparationLimits};
use ego_tree::iter::Edge;
pub(crate) fn prepare(source: &str, limits: &PreparationLimits) -> Result<Html, ExtractionError> {
    let document = Html::parse_document(source);
    let mut nodes = 0u32;
    let mut elements = 0u32;
    for node in document.tree.nodes() {
        nodes += 1;
        if nodes > limits.max_nodes {
            return Err(ExtractionError::resource(
                "preparation",
                "max_nodes",
                limits.max_nodes.into(),
            ));
        }
        if node.value().is_element() {
            elements += 1;
            if elements > limits.max_elements {
                return Err(ExtractionError::resource(
                    "preparation",
                    "max_elements",
                    limits.max_elements.into(),
                ));
            }
        }
    }
    let mut depth = 0u32;
    for edge in document.tree.root().traverse() {
        match edge {
            Edge::Open(_) => {
                if depth > limits.max_depth {
                    return Err(ExtractionError::resource(
                        "preparation",
                        "max_depth",
                        limits.max_depth.into(),
                    ));
                }
                depth += 1;
            }
            Edge::Close(_) => depth -= 1,
        }
    }
    Ok(document)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retained_detached_nodes_count_toward_postparse_acceptance() {
        // Frameset repair removes an already constructed body from the document.
        let source = "<p></p><frameset>";
        let document = Html::parse_document(source);
        let all = document.tree.nodes().count();
        let attached = document.tree.root().descendants().count();
        assert!(
            all > attached,
            "control must retain detached nodes: {all}/{attached}"
        );
        let limits = PreparationLimits {
            max_nodes: attached as u32,
            ..Default::default()
        };
        assert_eq!(
            prepare(source, &limits)
                .unwrap_err()
                .resource_counter
                .as_deref(),
            Some("max_nodes")
        );
    }
}
