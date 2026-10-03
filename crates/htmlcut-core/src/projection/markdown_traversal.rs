//! Original Markdown payload edges, with one charged subtree-exclusion state machine.

use super::*;

pub(super) fn omitted_payload(element: &scraper::node::Element) -> bool {
    let namespace: &str = element.name.ns.as_ref();
    (context::html(element) && matches!(element.name(), "script" | "style" | "template"))
        || (namespace == "http://www.w3.org/2000/svg"
            && matches!(element.name(), "script" | "style"))
}

pub(super) fn payload_edges<'a>(
    root: ElementRef<'a>,
    excluded: &'a HashSet<NodeId>,
    budget: &'a SelectorWorkBudget,
) -> impl Iterator<Item = Result<Edge<'a, Node>, ExtractionError>> + 'a {
    let mut skipped = 0_usize;
    root.traverse().filter_map(move |edge| {
        if let Err(error) = crate::execution::charge(budget, 1) {
            return Some(Err(error));
        }
        if skipped > 0 {
            match edge {
                Edge::Open(_) => skipped += 1,
                Edge::Close(_) => skipped -= 1,
            }
            return None;
        }
        if let Edge::Open(node) = edge
            && (excluded.contains(&node.id())
                || node.value().as_element().is_some_and(omitted_payload))
        {
            skipped = 1;
            return None;
        }
        Some(Ok(edge))
    })
}
