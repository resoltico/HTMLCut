// SPDX-License-Identifier: MPL-2.0
//! Bounded structural facts from the immutable original DOM, never outside payload.

use super::*;

pub(super) fn html(element: &crate::dom::Element) -> bool {
    let namespace: &str = element.name.ns.as_ref();
    namespace == "http://www.w3.org/1999/xhtml"
}

pub(super) fn inherited_pre(
    root: ElementRef<'_>,
    budget: &WorkBudget,
) -> Result<bool, ExtractionError> {
    for parent in root.ancestors() {
        crate::execution::charge(budget, 1)?;
        if parent
            .value()
            .as_element()
            .is_some_and(|e| e.name() == "pre" && html(e))
        {
            return Ok(true);
        }
    }
    Ok(false)
}
