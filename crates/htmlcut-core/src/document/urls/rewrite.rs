//! HTML-attribute and document-tree URL rewriting.

use ego_tree::NodeId;
use scraper::{Html, Node, StrTendril};

use super::super::parse::{
    first_body, parse_document_node, parse_wrapped_fragment, serialize_children, serialize_document,
};
use super::base::{resolve_url, starts_with_ignore_ascii_case};
use super::css::rewrite_css_urls;

const DIRECT_URL_ATTRIBUTE_NAMES: [&str; 7] = [
    "action",
    "cite",
    "data",
    "formaction",
    "href",
    "poster",
    "src",
];
const SRCSET_ATTRIBUTE_NAMES: [&str; 2] = ["imagesrcset", "srcset"];
const SPACE_SEPARATED_URL_ATTRIBUTE_NAMES: [&str; 1] = ["ping"];
const CSS_URL_ATTRIBUTE_NAMES: [&str; 1] = ["style"];

pub(crate) fn rewrite_html_urls(
    fragment: &str,
    base_url: Option<&str>,
    force_document: bool,
) -> String {
    let Some(base) = base_url else {
        return fragment.to_owned();
    };

    let is_document = force_document || looks_like_full_document(fragment);
    let mut document = if is_document {
        parse_document_node(fragment)
    } else {
        parse_wrapped_fragment(fragment)
    };

    rewrite_urls_in_document(&mut document, base);

    if is_document {
        serialize_document(&document)
    } else {
        let body = first_body(&document).expect("wrapped fragments always include a body element");
        serialize_children(&body)
    }
}

pub(crate) fn looks_like_full_document(fragment: &str) -> bool {
    let trimmed = fragment.trim_start();
    starts_with_ignore_ascii_case(trimmed, "<!doctype")
        || starts_with_ignore_ascii_case(trimmed, "<html")
}

pub(crate) fn rewrite_urls_in_document(document: &mut Html, base_url: &str) {
    let node_ids: Vec<NodeId> = document.tree.nodes().map(|node| node.id()).collect();
    rewrite_urls_in_document_with_node_ids(document, base_url, node_ids);
}

fn rewrite_urls_in_document_with_node_ids(
    document: &mut Html,
    base_url: &str,
    node_ids: impl IntoIterator<Item = NodeId>,
) {
    for node_id in node_ids {
        let Some(mut node) = document.tree.get_mut(node_id) else {
            continue;
        };
        let mut rewrite_style_children = false;
        {
            if let Node::Element(element) = node.value() {
                let tag_name = element.name().to_owned();
                let is_meta_refresh = raw_element_is_meta_refresh(element);
                for (name, value) in &mut element.attrs {
                    let rewritten = rewrite_attribute_value(
                        &tag_name,
                        name.local.as_ref(),
                        value,
                        Some(base_url),
                        is_meta_refresh,
                    );
                    if rewritten != value.as_ref() {
                        *value = StrTendril::from(rewritten);
                    }
                }
                rewrite_style_children = tag_name == "style";
            }
        }

        if rewrite_style_children {
            node.for_each_child(|child| {
                if let Node::Text(text) = child.value() {
                    let rewritten = rewrite_css_urls(text, Some(base_url));
                    if rewritten != text.as_ref() {
                        text.text = StrTendril::from(rewritten);
                    }
                }
            });
        }
    }
}

#[cfg(test)]
pub(crate) fn rewrite_urls_in_document_with_node_ids_for_tests(
    document: &mut Html,
    base_url: &str,
    node_ids: Vec<NodeId>,
) {
    rewrite_urls_in_document_with_node_ids(document, base_url, node_ids);
}

#[cfg(test)]
/// Returns whether an attribute participates in HTMLCut's URL-rewrite policy.
pub(crate) fn attribute_supports_url_rewrite(name: &str) -> bool {
    DIRECT_URL_ATTRIBUTE_NAMES.contains(&name)
        || SRCSET_ATTRIBUTE_NAMES.contains(&name)
        || SPACE_SEPARATED_URL_ATTRIBUTE_NAMES.contains(&name)
        || CSS_URL_ATTRIBUTE_NAMES.contains(&name)
}

pub(crate) fn rewrite_attribute_value(
    tag_name: &str,
    name: &str,
    value: &str,
    base_url: Option<&str>,
    is_meta_refresh: bool,
) -> String {
    if DIRECT_URL_ATTRIBUTE_NAMES.contains(&name) {
        return resolve_url(value, base_url);
    }

    if SRCSET_ATTRIBUTE_NAMES.contains(&name) {
        return rewrite_srcset(value, base_url);
    }

    if SPACE_SEPARATED_URL_ATTRIBUTE_NAMES.contains(&name) {
        return rewrite_space_separated_urls(value, base_url);
    }

    if CSS_URL_ATTRIBUTE_NAMES.contains(&name) {
        return rewrite_css_urls(value, base_url);
    }

    if name == "content" && tag_name == "meta" && is_meta_refresh {
        return rewrite_meta_refresh_content(value, base_url);
    }

    value.to_owned()
}

fn rewrite_srcset(value: &str, base_url: Option<&str>) -> String {
    rewrite_srcset_with_step(value, base_url, advance_srcset_cursor)
}

fn rewrite_srcset_with_step(
    value: &str,
    base_url: Option<&str>,
    advance: fn(&mut usize, usize) -> bool,
) -> String {
    rewrite_srcset_with_step_and_budget(value, base_url, advance, value.len().saturating_add(1))
}

fn rewrite_srcset_with_step_and_budget(
    value: &str,
    base_url: Option<&str>,
    advance: fn(&mut usize, usize) -> bool,
    mut remaining_steps: usize,
) -> String {
    let mut candidates = Vec::new();
    let mut cursor = 0usize;
    let bytes = value.as_bytes();

    while cursor < bytes.len() {
        while cursor < bytes.len() && (bytes[cursor].is_ascii_whitespace() || bytes[cursor] == b',')
        {
            if !consume_srcset_step_budget(&mut remaining_steps)
                || !advance_srcset_with_progress(&mut cursor, bytes.len(), advance)
            {
                return value.to_owned();
            }
        }
        if cursor >= bytes.len() {
            break;
        }

        let url_start = cursor;
        let data_url = value[url_start..].starts_with("data:");
        while cursor < bytes.len() {
            let byte = bytes[cursor];
            if byte.is_ascii_whitespace() {
                break;
            }
            if !data_url && byte == b',' {
                break;
            }
            if !consume_srcset_step_budget(&mut remaining_steps)
                || !advance_srcset_with_progress(&mut cursor, bytes.len(), advance)
            {
                return value.to_owned();
            }
        }
        let url = &value[url_start..cursor];

        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            if !consume_srcset_step_budget(&mut remaining_steps)
                || !advance_srcset_with_progress(&mut cursor, bytes.len(), advance)
            {
                return value.to_owned();
            }
        }

        let descriptor_start = cursor;
        while cursor < bytes.len() && bytes[cursor] != b',' {
            if !consume_srcset_step_budget(&mut remaining_steps)
                || !advance_srcset_with_progress(&mut cursor, bytes.len(), advance)
            {
                return value.to_owned();
            }
        }
        let descriptor = value[descriptor_start..cursor].trim();
        let rewritten_url = resolve_url(url, base_url);
        if descriptor.is_empty() {
            candidates.push(rewritten_url);
        } else {
            candidates.push(format!("{rewritten_url} {descriptor}"));
        }
    }

    if candidates.is_empty() {
        value.to_owned()
    } else {
        candidates.join(", ")
    }
}

fn advance_srcset_cursor(cursor: &mut usize, length: usize) -> bool {
    let Some(next) = cursor.checked_add(1) else {
        return false;
    };
    if next > length {
        return false;
    }
    *cursor = next;
    true
}

fn advance_srcset_with_progress(
    cursor: &mut usize,
    length: usize,
    advance: fn(&mut usize, usize) -> bool,
) -> bool {
    let before = *cursor;
    let advanced = advance(cursor, length);
    srcset_cursor_progress_is_valid(advanced, before, *cursor, length)
}

fn srcset_cursor_progress_is_valid(
    callback_succeeded: bool,
    before: usize,
    after: usize,
    length: usize,
) -> bool {
    callback_succeeded && after > before && after <= length
}

fn consume_srcset_step_budget(remaining_steps: &mut usize) -> bool {
    if *remaining_steps == 0 {
        return false;
    }
    *remaining_steps -= 1;
    true
}

#[cfg(test)]
pub(crate) fn rewrite_srcset_for_tests(value: &str, base_url: Option<&str>) -> String {
    rewrite_srcset(value, base_url)
}

#[cfg(test)]
pub(crate) fn srcset_rejects_non_advancing_progress_for_tests(value: &str) -> bool {
    rewrite_srcset_with_step(value, Some("https://example.test/"), |_, _| false) == value
}

#[cfg(test)]
pub(crate) fn srcset_rejects_staged_non_advancing_progress_for_tests(descriptor: bool) -> bool {
    fn advance_until_whitespace(cursor: &mut usize, length: usize) -> bool {
        if *cursor == 9 {
            return false;
        }
        advance_srcset_cursor(cursor, length)
    }
    fn advance_until_descriptor(cursor: &mut usize, length: usize) -> bool {
        if *cursor == 10 {
            return false;
        }
        advance_srcset_cursor(cursor, length)
    }

    let advance = if descriptor {
        advance_until_descriptor
    } else {
        advance_until_whitespace
    };
    rewrite_srcset_with_step("asset.png 2x", Some("https://example.test/"), advance)
        == "asset.png 2x"
}

#[cfg(test)]
pub(crate) fn srcset_progress_is_valid_for_tests(cursor: usize, length: usize) -> bool {
    let mut cursor = cursor;
    advance_srcset_cursor(&mut cursor, length)
}

#[cfg(test)]
pub(crate) fn srcset_rejects_success_without_progress_for_tests() -> bool {
    rewrite_srcset_with_step("asset.png 2x", Some("https://example.test/"), |_, _| true)
        == "asset.png 2x"
}

#[cfg(test)]
pub(crate) fn srcset_callback_progress_is_valid_for_tests(
    callback_succeeded: bool,
    before: usize,
    after: usize,
    length: usize,
) -> bool {
    srcset_cursor_progress_is_valid(callback_succeeded, before, after, length)
}

#[cfg(test)]
pub(crate) fn srcset_step_budget_exhausts_for_tests() -> bool {
    let mut remaining_steps = 3;
    let outcomes = [
        consume_srcset_step_budget(&mut remaining_steps),
        consume_srcset_step_budget(&mut remaining_steps),
        consume_srcset_step_budget(&mut remaining_steps),
        consume_srcset_step_budget(&mut remaining_steps),
    ];
    outcomes == [true, true, true, false]
}

#[cfg(test)]
pub(crate) fn srcset_budget_rejection_for_tests(stage: SrcsetBudgetStage) -> bool {
    let (value, remaining_steps) = match stage {
        SrcsetBudgetStage::LeadingSeparator => (" asset.png", 0),
        SrcsetBudgetStage::Url => ("asset.png", 0),
        SrcsetBudgetStage::Whitespace => ("asset.png 2x", 9),
        SrcsetBudgetStage::Descriptor => ("asset.png 2x", 10),
    };
    rewrite_srcset_with_step_and_budget(
        value,
        Some("https://example.test/"),
        advance_srcset_cursor,
        remaining_steps,
    ) == value
}

#[cfg(test)]
#[derive(Clone, Copy)]
pub(crate) enum SrcsetBudgetStage {
    LeadingSeparator,
    Url,
    Whitespace,
    Descriptor,
}

fn rewrite_space_separated_urls(value: &str, base_url: Option<&str>) -> String {
    value
        .split_whitespace()
        .map(|token| resolve_url(token, base_url))
        .collect::<Vec<_>>()
        .join(" ")
}

fn rewrite_meta_refresh_content(value: &str, base_url: Option<&str>) -> String {
    value
        .split(';')
        .map(rewrite_meta_refresh_segment(base_url))
        .collect::<Vec<_>>()
        .join(";")
}

fn rewrite_meta_refresh_segment<'a>(base_url: Option<&'a str>) -> impl Fn(&str) -> String + 'a {
    move |segment| {
        let trimmed_start = segment.trim_start();
        let leading_whitespace_len = segment.len() - trimmed_start.len();
        let trimmed = trimmed_start.trim_end();
        let trailing_whitespace = &trimmed_start[trimmed.len()..];
        if !trimmed
            .get(..4)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("url="))
        {
            return segment.to_owned();
        }

        let prefix = &trimmed[..4];
        let raw_value = &trimmed[4..];
        let raw_value_trimmed = raw_value.trim_start();
        let value_leading_whitespace = &raw_value[..raw_value.len() - raw_value_trimmed.len()];
        let resolved_value = if let Some(stripped) = raw_value_trimmed
            .strip_prefix('"')
            .and_then(|quoted| quoted.strip_suffix('"'))
        {
            format!("\"{}\"", resolve_url(stripped, base_url))
        } else if let Some(stripped) = raw_value_trimmed
            .strip_prefix('\'')
            .and_then(|quoted| quoted.strip_suffix('\''))
        {
            format!("'{}'", resolve_url(stripped, base_url))
        } else {
            resolve_url(raw_value_trimmed, base_url)
        };

        format!(
            "{}{}{}{}{}",
            &segment[..leading_whitespace_len],
            prefix,
            value_leading_whitespace,
            resolved_value,
            trailing_whitespace
        )
    }
}
pub(super) fn raw_element_is_meta_refresh(element: &scraper::node::Element) -> bool {
    if element.name() != "meta" {
        return false;
    }

    element.attrs.iter().any(|(name, value)| {
        name.local.as_ref() == "http-equiv" && value.eq_ignore_ascii_case("refresh")
    })
}
