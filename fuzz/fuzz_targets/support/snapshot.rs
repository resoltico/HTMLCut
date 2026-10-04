// SPDX-License-Identifier: MPL-2.0
use htmlcut_core::{PreparationLimits, PreparedDocument, SnapshotMetadata, SourceSnapshot};

pub fn text(value: &str, maximum: usize) -> &str {
    let mut end = value.len().min(maximum);
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    &value[..end]
}

pub fn document(html: &str) -> Option<PreparedDocument> {
    let limits = PreparationLimits {
        max_source_bytes: 32768,
        max_elements: 512,
        max_nodes: 2048,
        max_depth: 64,
        max_parse_work: 100_000,
    };
    PreparedDocument::new(
        SourceSnapshot::new(text(html, 32768), SnapshotMetadata::default()).ok()?,
        limits,
    )
    .ok()
}
