//! Safe, bounded public attribute evidence for exploration descriptors.

use scraper::ElementRef;

use super::{
    ExplorationAttribute, MAX_ATTRIBUTE_NAME_BYTES, MAX_ATTRIBUTE_VALUE_BYTES, MAX_ATTRIBUTES,
};

pub(super) fn safe_attributes(element: &ElementRef<'_>) -> (Vec<ExplorationAttribute>, bool) {
    let mut attributes = element
        .value()
        .attrs()
        .filter_map(|(name, value)| {
            let name = name.to_owned();
            let allowed = matches!(name.as_str(), "id" | "class" | "role" | "itemprop")
                || name == "aria-label"
                || name.starts_with("data-");
            (allowed && name.len() <= MAX_ATTRIBUTE_NAME_BYTES).then(|| {
                let value = value.to_string();
                let value_truncated = value.len() > MAX_ATTRIBUTE_VALUE_BYTES;
                ExplorationAttribute {
                    name,
                    value: bounded_prefix(&value, MAX_ATTRIBUTE_VALUE_BYTES),
                    value_truncated,
                }
            })
        })
        .collect::<Vec<_>>();
    attributes.sort_by(|left, right| left.name.cmp(&right.name));
    let attributes_truncated = attributes.len() > MAX_ATTRIBUTES;
    attributes.truncate(MAX_ATTRIBUTES);
    (attributes, attributes_truncated)
}

pub(super) fn bounded_prefix(value: &str, maximum: usize) -> String {
    if value.len() <= maximum {
        return value.to_owned();
    }
    value[..value.floor_char_boundary(maximum)].to_owned()
}

pub(super) fn exceeds_byte_limit(length: usize, maximum: usize) -> bool {
    length > maximum
}
