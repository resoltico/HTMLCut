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
const CSS_URL_ATTRIBUTE_NAMES: [&str; 1] = ["style"];

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

fn rewrite_css_urls(value: &str, base_url: Option<&str>) -> String {
    let Some(base_url) = base_url else {
        return value.to_owned();
    };

    let mut rewritten = String::with_capacity(value.len());
    let mut cursor = 0usize;
    while cursor < value.len() {
        if let Some(end) = css_comment_end(value, cursor) {
            rewritten.push_str(&value[cursor..end]);
            cursor = end;
            continue;
        }

        if let Some((replacement, next)) = rewrite_css_url_function_at(value, cursor, base_url) {
            rewritten.push_str(&replacement);
            cursor = next;
            continue;
        }

        if let Some((replacement, next)) = rewrite_css_import_string_at(value, cursor, base_url) {
            rewritten.push_str(&replacement);
            cursor = next;
            continue;
        }

        let next = next_char_boundary(value, cursor);
        rewritten.push_str(&value[cursor..next]);
        cursor = next;
    }

    rewritten
}

fn css_comment_end(value: &str, cursor: usize) -> Option<usize> {
    if !value[cursor..].starts_with("/*") {
        return None;
    }

    Some(
        value[cursor + 2..]
            .find("*/")
            .map(|offset| cursor + 2 + offset + 2)
            .unwrap_or(value.len()),
    )
}

fn rewrite_css_import_string_at(
    value: &str,
    cursor: usize,
    base_url: &str,
) -> Option<(String, usize)> {
    if !value[cursor..].starts_with('@') || !starts_with_css_keyword(value, cursor + 1, "import") {
        return None;
    }

    let mut index = cursor + 1 + "import".len();
    index = skip_css_ignorable(value, index);
    let quote = value[index..].chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }

    let content_start = index + quote.len_utf8();
    let content_end = find_css_string_end(value, index)?;
    let resolved = resolve_url(&value[content_start..content_end], Some(base_url));
    let next = content_end + quote.len_utf8();

    Some((
        format!(
            "{}{}{}",
            &value[cursor..content_start],
            resolved,
            &value[content_end..next]
        ),
        next,
    ))
}

fn rewrite_css_url_function_at(
    value: &str,
    cursor: usize,
    base_url: &str,
) -> Option<(String, usize)> {
    if !starts_with_css_keyword(value, cursor, "url") {
        return None;
    }
    if cursor > 0
        && value[..cursor]
            .chars()
            .next_back()
            .is_some_and(is_css_identifier_char)
    {
        return None;
    }

    let mut index = cursor + "url".len();
    index = skip_ascii_whitespace(value, index);
    if !value[index..].starts_with('(') {
        return None;
    }

    let mut content_start = skip_ascii_whitespace(value, index + 1);
    let quote = value[content_start..].chars().next()?;
    if quote == '"' || quote == '\'' {
        let raw_start = content_start + quote.len_utf8();
        let raw_end = find_css_string_end(value, content_start)?;
        let after_quote = skip_ascii_whitespace(value, raw_end + quote.len_utf8());
        if !value[after_quote..].starts_with(')') {
            return None;
        }
        let resolved = resolve_url(&value[raw_start..raw_end], Some(base_url));
        let next = after_quote + 1;
        return Some((
            format!(
                "{}{}{}",
                &value[cursor..raw_start],
                resolved,
                &value[raw_end..next]
            ),
            next,
        ));
    }

    let raw_start = content_start;
    while content_start < value.len() {
        let ch = value[content_start..].chars().next()?;
        if ch == ')' {
            break;
        }
        content_start = next_char_boundary(value, content_start);
    }
    if content_start >= value.len() {
        return None;
    }
    debug_assert!(value[content_start..].starts_with(')'));

    let mut raw_end = content_start;
    while raw_end > raw_start
        && value[..raw_end]
            .chars()
            .next_back()
            .is_some_and(char::is_whitespace)
    {
        raw_end = previous_char_boundary(value, raw_end);
    }
    if raw_end == raw_start {
        return None;
    }

    let resolved = resolve_url(&value[raw_start..raw_end], Some(base_url));
    let next = content_start + 1;
    Some((
        format!(
            "{}{}{}",
            &value[cursor..raw_start],
            resolved,
            &value[raw_end..next]
        ),
        next,
    ))
}

fn skip_css_ignorable(value: &str, mut cursor: usize) -> usize {
    loop {
        let next = skip_ascii_whitespace(value, cursor);
        if let Some(end) = css_comment_end(value, next) {
            cursor = end;
            continue;
        }
        return next;
    }
}

fn skip_ascii_whitespace(value: &str, mut cursor: usize) -> usize {
    while cursor < value.len() {
        let ch = value[cursor..].chars().next().expect("char boundary");
        if !ch.is_ascii_whitespace() {
            break;
        }
        cursor += ch.len_utf8();
    }
    cursor
}

fn starts_with_css_keyword(value: &str, cursor: usize, keyword: &str) -> bool {
    let end = cursor + keyword.len();
    value
        .get(cursor..end)
        .is_some_and(|candidate| candidate.eq_ignore_ascii_case(keyword))
}

fn find_css_string_end(value: &str, quote_index: usize) -> Option<usize> {
    let quote = value[quote_index..].chars().next()?;
    let mut cursor = quote_index + quote.len_utf8();
    while cursor < value.len() {
        let ch = value[cursor..].chars().next()?;
        if ch == '\\' {
            cursor = next_char_boundary(value, cursor);
            if cursor < value.len() {
                cursor = next_char_boundary(value, cursor);
            }
            continue;
        }
        if ch == quote {
            return Some(cursor);
        }
        cursor = next_char_boundary(value, cursor);
    }
    None
}

fn next_char_boundary(value: &str, cursor: usize) -> usize {
    cursor
        + value[cursor..]
            .chars()
            .next()
            .expect("char boundary")
            .len_utf8()
}

fn previous_char_boundary(value: &str, cursor: usize) -> usize {
    value[..cursor]
        .char_indices()
        .last()
        .map(|(index, _)| index)
        .unwrap_or(0)
}

fn is_css_identifier_char(ch: char) -> bool {
    ch == '-' || ch == '_' || ch.is_alphanumeric()
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

#[cfg(test)]
pub(crate) fn rewrite_css_urls_for_tests(value: &str, base_url: Option<&str>) -> String {
    rewrite_css_urls(value, base_url)
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
    use scraper::node::Comment;

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

    #[test]
    fn css_rewrite_helpers_cover_comments_escapes_and_invalid_forms() {
        assert_eq!(rewrite_css_urls("url(hero.png)", None), "url(hero.png)");
        assert_eq!(
            rewrite_css_urls(
                "/* keep url(old.png) */ url(\"hero.png\")",
                Some("https://example.test/assets/")
            ),
            "/* keep url(old.png) */ url(\"https://example.test/assets/hero.png\")"
        );
        assert_eq!(
            rewrite_css_urls(
                "@import /* note */ 'theme.css' screen; body { background: url( hero.png  ) }",
                Some("https://example.test/assets/")
            ),
            "@import /* note */ 'https://example.test/assets/theme.css' screen; body { background: url( https://example.test/assets/hero.png  ) }"
        );
        assert_eq!(
            rewrite_css_urls(
                "background: myurl(icon.png); list-style: url( \"icon\\\"2.png\" )",
                Some("https://example.test/assets/")
            ),
            "background: myurl(icon.png); list-style: url( \"https://example.test/assets/icon/%222.png\" )"
        );
        assert_eq!(
            rewrite_css_urls(
                "@import url(theme.css); background: url(\"unterminated.png\";",
                Some("https://example.test/assets/")
            ),
            "@import url(https://example.test/assets/theme.css); background: url(\"unterminated.png\";"
        );
        assert_eq!(
            rewrite_css_urls("background: url(   )", Some("https://example.test/assets/")),
            "background: url(   )"
        );
        assert_eq!(
            rewrite_css_urls(
                "background: url hero.png); color: red;",
                Some("https://example.test/assets/")
            ),
            "background: url hero.png); color: red;"
        );
        assert_eq!(
            rewrite_css_urls(
                "background: url(hero.png",
                Some("https://example.test/assets/")
            ),
            "background: url(hero.png"
        );
        assert_eq!(find_css_string_end("\"a\\\"b\"", 0), Some("\"a\\\"b".len()));
        assert_eq!(find_css_string_end("\"escape-at-end\\", 0), None);
        assert_eq!(find_css_string_end("\"unterminated", 0), None);
        assert_eq!(
            css_comment_end("/* unterminated", 0),
            Some("/* unterminated".len())
        );
        assert_eq!(
            rewrite_css_url_function_at("url hero.png)", 0, "https://example.test/assets/"),
            None
        );
        assert_eq!(
            rewrite_css_url_function_at("url(hero.png", 0, "https://example.test/assets/"),
            None
        );
        assert_eq!(
            rewrite_css_import_string_at("@media screen", 0, "https://example.test/assets/"),
            None
        );
        assert_eq!(skip_ascii_whitespace("x", 1), 1);
        assert!(is_css_identifier_char('-'));
        assert!(is_css_identifier_char('_'));

        let mut document = parse_document_node(
            "<style>/* keep */ @import \"theme.css\"; .hero { background: url('../img/card.png') }</style>",
        );
        let style_id = select_first(&document, "style").expect("style").id();
        rewrite_urls_in_document_with_node_ids_for_tests(
            &mut document,
            "https://example.test/docs/articles/",
            vec![style_id],
        );
        let serialized = crate::document::serialize_document(&document);
        assert!(serialized.contains("@import \"https://example.test/docs/articles/theme.css\""));
        assert!(serialized.contains("url('https://example.test/docs/img/card.png')"));

        let mut style_document =
            parse_document_node("<style>.hero { background: url(hero.png) }</style>");
        let style_id = select_first(&style_document, "style").expect("style").id();
        rewrite_urls_in_document_with_node_ids_for_tests(
            &mut style_document,
            "https://example.test/assets/",
            vec![style_id],
        );
        assert!(
            crate::document::serialize_document(&style_document)
                .contains("url(https://example.test/assets/hero.png)")
        );

        let mut unchanged_style_document =
            parse_document_node("<style>.hero { color: red }</style>");
        let style_id = select_first(&unchanged_style_document, "style")
            .expect("style")
            .id();
        rewrite_urls_in_document_with_node_ids_for_tests(
            &mut unchanged_style_document,
            "https://example.test/assets/",
            vec![style_id],
        );
        assert!(
            crate::document::serialize_document(&unchanged_style_document)
                .contains(".hero { color: red }")
        );

        let mut style_with_comment =
            parse_document_node("<style>.hero { background: url(hero.png) }</style>");
        let style_id = select_first(&style_with_comment, "style")
            .expect("style")
            .id();
        style_with_comment
            .tree
            .get_mut(style_id)
            .expect("style node")
            .append(Node::Comment(Comment {
                comment: StrTendril::from("kept"),
            }));
        rewrite_urls_in_document_with_node_ids_for_tests(
            &mut style_with_comment,
            "https://example.test/assets/",
            vec![style_id],
        );
        assert!(
            crate::document::serialize_document(&style_with_comment)
                .contains("https://example.test/assets/hero.png")
        );
    }
}
