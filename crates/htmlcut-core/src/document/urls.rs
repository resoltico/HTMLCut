use ego_tree::NodeId;
use scraper::{Html, Node, StrTendril};
use url::Url;

use super::parse::{
    first_body, parse_document_node, parse_wrapped_fragment, select_first, serialize_children,
    serialize_document,
};

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

pub(crate) fn document_base_href(document: &Html) -> Option<String> {
    select_first(document, "base[href]")
        .and_then(|node| node.value().attr("href"))
        .map(str::trim)
        .filter(|href| !href.is_empty())
        .map(str::to_owned)
}

pub(crate) fn resolve_document_base_url(
    document: &Html,
    input_base_url: Option<&str>,
) -> Option<String> {
    let Some(document_base_href) = document_base_href(document) else {
        return input_base_url.map(ToOwned::to_owned);
    };

    if document_base_href.starts_with('#') {
        return input_base_url.map(ToOwned::to_owned);
    }

    if let Ok(parsed) = Url::parse(&document_base_href)
        && matches!(parsed.scheme(), "http" | "https")
    {
        return Some(parsed.to_string());
    }

    input_base_url
        .and_then(|base| Url::parse(base).ok())
        .and_then(|base| base.join(&document_base_href).ok())
        .filter(|resolved| matches!(resolved.scheme(), "http" | "https"))
        .map(|resolved| resolved.to_string())
        .or_else(|| input_base_url.map(ToOwned::to_owned))
}

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
        let body = first_body(&document).expect("wrapped fragments always contain a body");
        serialize_children(&body)
    }
}

pub(crate) fn looks_like_full_document(fragment: &str) -> bool {
    let trimmed = fragment.trim_start().to_lowercase();
    trimmed.starts_with("<!doctype") || trimmed.starts_with("<html")
}

pub(crate) fn rewrite_urls_in_document(document: &mut Html, base_url: &str) {
    let node_ids: Vec<NodeId> = document.tree.nodes().map(|node| node.id()).collect();

    for node_id in node_ids {
        let mut node = document
            .tree
            .get_mut(node_id)
            .expect("collected node ids must remain valid");
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
        }
    }
}

#[cfg(test)]
/// Returns whether an attribute participates in HTMLCut's URL-rewrite normalization.
pub(crate) fn attribute_supports_url_rewrite(name: &str) -> bool {
    DIRECT_URL_ATTRIBUTE_NAMES.contains(&name)
        || SRCSET_ATTRIBUTE_NAMES.contains(&name)
        || SPACE_SEPARATED_URL_ATTRIBUTE_NAMES.contains(&name)
}

pub(crate) fn resolve_url(value: &str, base_url: Option<&str>) -> String {
    let Some(base) = base_url else {
        return value.to_owned();
    };

    if value.starts_with('#') {
        return value.to_owned();
    }

    if Url::parse(value).is_ok() {
        return value.to_owned();
    }

    match Url::parse(base).and_then(|base_url| base_url.join(value)) {
        Ok(url) => url.to_string(),
        Err(_) => value.to_owned(),
    }
}

pub(super) fn rewrite_attribute_value(
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
        .join("; ")
}

fn rewrite_meta_refresh_segment<'a>(base_url: Option<&'a str>) -> impl Fn(&str) -> String + 'a {
    move |segment| {
        let trimmed = segment.trim();
        if !trimmed
            .get(..4)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("url="))
        {
            return trimmed.to_owned();
        }

        let raw_value = trimmed[4..].trim();
        if let Some(stripped) = raw_value
            .strip_prefix('"')
            .and_then(|quoted| quoted.strip_suffix('"'))
        {
            return format!("url=\"{}\"", resolve_url(stripped, base_url));
        }

        if let Some(stripped) = raw_value
            .strip_prefix('\'')
            .and_then(|quoted| quoted.strip_suffix('\''))
        {
            return format!("url='{}'", resolve_url(stripped, base_url));
        }

        format!("url={}", resolve_url(raw_value, base_url))
    }
}

fn raw_element_is_meta_refresh(element: &scraper::node::Element) -> bool {
    if element.name() != "meta" {
        return false;
    }

    element.attrs.iter().any(|(name, value)| {
        name.local.as_ref() == "http-equiv" && value.eq_ignore_ascii_case("refresh")
    })
}
