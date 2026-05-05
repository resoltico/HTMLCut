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
}

pub(crate) fn resolve_url(value: &str, base_url: Option<&str>) -> String {
    let Some(base) = base_url else {
        return value.to_owned();
    };

    if value.trim().is_empty() {
        return value.to_owned();
    }

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

pub(crate) fn href_is_meaningful_destination(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed == "#" {
        return false;
    }

    !starts_with_ignore_ascii_case(trimmed, "javascript:")
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

fn raw_element_is_meta_refresh(element: &scraper::node::Element) -> bool {
    if element.name() != "meta" {
        return false;
    }

    element.attrs.iter().any(|(name, value)| {
        name.local.as_ref() == "http-equiv" && value.eq_ignore_ascii_case("refresh")
    })
}

fn starts_with_ignore_ascii_case(value: &str, prefix: &str) -> bool {
    value
        .get(..prefix.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(prefix))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{parse_document_node, select_first};

    #[test]
    fn meta_refresh_helper_detects_exact_attribute_names_and_values() {
        let base_document =
            parse_document_node("<base href=\" ../help/ \"><a href=\"guide.html\">Guide</a>");
        assert_eq!(
            document_base_href(&base_document).as_deref(),
            Some("../help/")
        );
        let blank_base_document = parse_document_node("<base href=\"   \">");
        assert_eq!(document_base_href(&blank_base_document), None);
        let fragment_base_document = parse_document_node("<base href=\"#content\">");
        assert_eq!(
            resolve_document_base_url(
                &fragment_base_document,
                Some("https://example.test/docs/start.html")
            )
            .as_deref(),
            Some("https://example.test/docs/start.html")
        );
        let absolute_base_document =
            parse_document_node("<base href=\"https://docs.example.test/help/\">");
        assert_eq!(
            resolve_document_base_url(
                &absolute_base_document,
                Some("https://example.test/docs/start.html")
            )
            .as_deref(),
            Some("https://docs.example.test/help/")
        );
        let relative_base_document = parse_document_node("<base href=\"../help/\">");
        assert_eq!(
            resolve_document_base_url(
                &relative_base_document,
                Some("https://example.test/docs/start.html")
            )
            .as_deref(),
            Some("https://example.test/help/")
        );
        let unsupported_base_document =
            parse_document_node("<base href=\"ftp://example.test/help/\">");
        assert_eq!(
            resolve_document_base_url(
                &unsupported_base_document,
                Some("https://example.test/docs/start.html")
            )
            .as_deref(),
            Some("https://example.test/docs/start.html")
        );
        assert!(!href_is_meaningful_destination("   "));
        assert!(!href_is_meaningful_destination("#"));
        assert!(!href_is_meaningful_destination("javascript:void(0)"));
        assert!(href_is_meaningful_destination("/guide"));
        assert_eq!(
            rewrite_html_urls("<a href=\"/guide\">Guide</a>", None, false),
            "<a href=\"/guide\">Guide</a>"
        );
        assert_eq!(resolve_url("", Some("https://example.test/base/")), "");
        assert_eq!(
            resolve_url("#fragment", Some("https://example.test/base/")),
            "#fragment"
        );
        assert_eq!(
            resolve_url(
                "mailto:help@example.test",
                Some("https://example.test/base/")
            ),
            "mailto:help@example.test"
        );
        assert_eq!(
            rewrite_attribute_value(
                "img",
                "srcset",
                "hero.jpg 1x, hero@2x.jpg 2x",
                Some("https://example.test/assets/"),
                false,
            ),
            "https://example.test/assets/hero.jpg 1x, https://example.test/assets/hero@2x.jpg 2x"
        );
        assert_eq!(
            rewrite_attribute_value(
                "a",
                "ping",
                "/a /b",
                Some("https://example.test/base/"),
                false,
            ),
            "https://example.test/a https://example.test/b"
        );
        assert_eq!(
            rewrite_attribute_value(
                "meta",
                "content",
                "0; url= /next",
                Some("https://example.test/base/"),
                true,
            ),
            "0; url= https://example.test/next"
        );
        assert_eq!(
            rewrite_attribute_value(
                "meta",
                "content",
                "0; URL=\"/quoted\"",
                Some("https://example.test/base/"),
                true,
            ),
            "0; URL=\"https://example.test/quoted\""
        );
        assert_eq!(
            rewrite_attribute_value(
                "div",
                "data-href",
                "/keep",
                Some("https://example.test/"),
                false
            ),
            "/keep"
        );
        assert_eq!(
            rewrite_srcset_for_tests(
                "data:image/gif;base64,AAAA 1x, /hero@2x.jpg 2x",
                Some("https://example.test/assets/")
            ),
            "data:image/gif;base64,AAAA 1x, https://example.test/hero@2x.jpg 2x"
        );

        let refresh =
            parse_document_node("<meta http-equiv=\"refresh\" content=\"0; url=next.html\">");
        let refresh_meta = select_first(&refresh, "meta").expect("refresh meta");
        assert!(raw_element_is_meta_refresh(refresh_meta.value()));
        let refresh_upper =
            parse_document_node("<meta http-equiv=\"REFRESH\" content=\"0; url=next.html\">");
        let refresh_upper_meta = select_first(&refresh_upper, "meta").expect("refresh meta");
        assert!(raw_element_is_meta_refresh(refresh_upper_meta.value()));

        let disguised =
            parse_document_node("<meta data-http-equiv=\"refresh\" content=\"0; url=next.html\">");
        let disguised_meta = select_first(&disguised, "meta").expect("disguised meta");
        assert!(!raw_element_is_meta_refresh(disguised_meta.value()));

        assert!(starts_with_ignore_ascii_case("<HTML", "<html"));
        assert!(!starts_with_ignore_ascii_case("ht", "<html"));
    }
}
