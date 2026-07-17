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
    let mut candidates = Vec::new();
    let mut cursor = 0usize;
    let bytes = value.as_bytes();

    while cursor < bytes.len() {
        while cursor < bytes.len() && (bytes[cursor].is_ascii_whitespace() || bytes[cursor] == b',')
        {
            cursor += 1;
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
            cursor += 1;
        }
        let url = &value[url_start..cursor];

        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }

        let descriptor_start = cursor;
        while cursor < bytes.len() && bytes[cursor] != b',' {
            cursor += 1;
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

#[cfg(test)]
pub(crate) fn rewrite_srcset_for_tests(value: &str, base_url: Option<&str>) -> String {
    rewrite_srcset(value, base_url)
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
