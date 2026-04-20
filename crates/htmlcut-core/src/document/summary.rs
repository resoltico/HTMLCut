use std::collections::BTreeMap;

use scraper::Html;
use serde_json::Value;

use crate::contracts::InspectionCount;

use super::parse::text_from_title;
use super::render::ELLIPSIS;

pub(crate) fn build_preview(value: &Value, preview_chars: usize) -> String {
    let rendered = match value {
        Value::String(text) => text.clone(),
        other => serde_json::to_string(other).unwrap_or_else(|_| String::new()),
    };

    if rendered.len() <= preview_chars {
        return rendered;
    }

    let keep = preview_chars.saturating_sub(ELLIPSIS.len());
    format!("{}{}", rendered[..keep].trim_end(), ELLIPSIS)
}

pub(crate) fn summarize_counts(
    counts: BTreeMap<String, usize>,
    sample_limit: usize,
) -> Vec<InspectionCount> {
    let mut entries: Vec<InspectionCount> = counts
        .into_iter()
        .map(|(name, count)| InspectionCount { name, count })
        .collect();
    entries.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.name.cmp(&right.name))
    });
    entries.truncate(sample_limit);
    entries
}

pub(crate) fn extract_document_title(document: &Html) -> Option<String> {
    text_from_title(document)
}

pub(crate) fn heading_level(tag_name: &str) -> Option<u8> {
    tag_name
        .strip_prefix('h')
        .and_then(|level| level.parse::<u8>().ok())
        .filter(|level| (1..=6).contains(level))
}
