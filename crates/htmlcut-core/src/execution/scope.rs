// SPDX-License-Identifier: MPL-2.0
//! A selected original-DOM row subtree.
use crate::dom::ElementRef;
pub(crate) struct SelectionScope<'a> {
    pub(super) anchor: ElementRef<'a>,
}
pub(super) fn selected_scopes<'a>(selected: &[ElementRef<'a>]) -> Vec<SelectionScope<'a>> {
    selected
        .iter()
        .map(|anchor| SelectionScope { anchor: *anchor })
        .collect()
}
