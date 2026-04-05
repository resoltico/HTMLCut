use std::collections::BTreeMap;

use ego_tree::{NodeId, NodeRef as DomNodeRef};
use scraper::{ElementRef, Html, Node, Selector, StrTendril};
use serde_json::Value;
use url::Url;

use crate::contracts::{InspectionCount, WhitespaceMode};

pub(crate) const ELLIPSIS: &str = "...";
const URL_ATTRIBUTE_NAMES: [&str; 7] = [
    "action",
    "cite",
    "data",
    "formaction",
    "href",
    "poster",
    "src",
];
const BLOCK_TAGS: [&str; 21] = [
    "article",
    "aside",
    "blockquote",
    "dd",
    "div",
    "dl",
    "dt",
    "figcaption",
    "figure",
    "footer",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "header",
    "li",
    "main",
    "p",
    "section",
];
const SKIP_TAGS: [&str; 5] = ["head", "noscript", "script", "style", "template"];

pub(crate) fn parse_document_node(input: &str) -> Html {
    Html::parse_document(input)
}

pub(crate) fn parse_wrapped_fragment(fragment: &str) -> Html {
    Html::parse_document(&format!(
        "<!DOCTYPE html><html><body>{fragment}</body></html>"
    ))
}

pub(crate) fn serialize_document(document: &Html) -> String {
    document.html()
}

pub(crate) fn serialize_element(node: &ElementRef<'_>) -> String {
    node.html()
}

pub(crate) fn serialize_children(node: &ElementRef<'_>) -> String {
    node.inner_html()
}

pub(crate) fn select_first<'a>(document: &'a Html, selector: &str) -> Option<ElementRef<'a>> {
    let selector = Selector::parse(selector).expect("static selectors should parse");
    document.select(&selector).next()
}

pub(crate) fn build_node_path(node: &ElementRef<'_>) -> String {
    let mut segments = Vec::new();
    let mut current = Some(*node);

    while let Some(node_ref) = current {
        let name = node_ref.value().name().to_owned();
        let position = if let Some(parent) = node_ref.parent().and_then(ElementRef::wrap) {
            let mut index = 0usize;
            for sibling in parent.child_elements() {
                if sibling.value().name() == name {
                    index += 1;
                }
                if sibling.id() == node_ref.id() {
                    break;
                }
            }
            current = Some(parent);
            index
        } else {
            current = None;
            1
        };

        segments.push(format!("{name}:nth-of-type({position})"));
    }

    segments.reverse();
    segments.join(" > ")
}

#[cfg(test)]
pub(crate) fn element_name(node: DomNodeRef<'_, Node>) -> Option<String> {
    match node.value() {
        Node::Element(element) => Some(element.name().to_owned()),
        _ => None,
    }
}

pub(crate) fn element_attributes(
    node: &ElementRef<'_>,
    base_url: Option<&str>,
    rewrite_urls: bool,
) -> BTreeMap<String, String> {
    let mut attributes = BTreeMap::new();

    for (name, value) in node.value().attrs() {
        let attribute_value = if rewrite_urls && attribute_supports_url_rewrite(name) {
            resolve_url(value, base_url)
        } else {
            value.to_owned()
        };
        attributes.insert(name.to_owned(), attribute_value);
    }

    attributes
}

pub(crate) fn first_fragment_attributes(
    fragment: &str,
    base_url: Option<&str>,
    rewrite_urls: bool,
) -> BTreeMap<String, String> {
    let document = parse_wrapped_fragment(fragment);
    let Some(first_element) = first_body_child_element(&document) else {
        return BTreeMap::new();
    };

    element_attributes(&first_element, base_url, rewrite_urls)
}

pub(crate) fn render_html_as_text(fragment: &str, whitespace: WhitespaceMode) -> String {
    let document = parse_wrapped_fragment(fragment);
    let body = first_body(&document).expect("wrapped fragments always contain a body");

    let mut output = String::new();
    for child in body.children() {
        render_node(child, &mut output, false, false);
    }

    let normalized = output
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        .replace("\n\n\n", "\n\n");

    apply_whitespace_mode(normalized.trim(), whitespace)
}

pub(crate) fn render_node(
    node: DomNodeRef<'_, Node>,
    output: &mut String,
    in_pre: bool,
    list_item: bool,
) {
    match node.value() {
        Node::Text(contents) => {
            let text = if in_pre {
                contents.to_string()
            } else {
                collapse_inline_whitespace(contents)
            };
            if text.is_empty() {
                return;
            }
            if needs_space(output, &text) {
                output.push(' ');
            }
            output.push_str(&text);
        }
        Node::Element(data) => {
            let tag_name = data.name();
            if SKIP_TAGS.contains(&tag_name) {
                return;
            }

            if tag_name == "br" {
                push_newline(output, 1);
                return;
            }

            if tag_name == "hr" {
                push_newline(output, 2);
                output.push_str("---");
                push_newline(output, 2);
                return;
            }

            if tag_name == "li" {
                push_newline(output, 1);
                output.push_str("- ");
                for child in node.children() {
                    render_node(child, output, false, true);
                }
                push_newline(output, 1);
                return;
            }

            let is_block = BLOCK_TAGS.contains(&tag_name);
            if is_block && !list_item {
                push_newline(output, 2);
            }

            let child_in_pre = in_pre || tag_name == "pre";
            for child in node.children() {
                render_node(child, output, child_in_pre, false);
            }

            if is_block {
                push_newline(output, 2);
            }
        }
        _ => {
            for child in node.children() {
                render_node(child, output, in_pre, list_item);
            }
        }
    }
}

pub(crate) fn collapse_inline_whitespace(input: &str) -> String {
    let mut output = String::new();
    let mut previous_was_whitespace = false;

    for character in input.chars() {
        if character.is_whitespace() {
            previous_was_whitespace = true;
            continue;
        }

        if previous_was_whitespace && !output.is_empty() {
            output.push(' ');
        }

        output.push(character);
        previous_was_whitespace = false;
    }

    output
}

pub(crate) fn needs_space(output: &str, next_text: &str) -> bool {
    let Some(last_character) = output.chars().last() else {
        return false;
    };
    let Some(first_character) = next_text.chars().next() else {
        return false;
    };

    !last_character.is_whitespace()
        && !matches!(last_character, '(' | '[' | '{' | '/' | '-')
        && !matches!(
            first_character,
            ')' | ']' | '}' | ',' | '.' | ';' | ':' | '!' | '?'
        )
}

pub(crate) fn push_newline(output: &mut String, count: usize) {
    let trimmed = output.trim_end_matches('\n').to_owned();
    *output = trimmed;
    if !output.is_empty() {
        output.push_str(&"\n".repeat(count));
    }
}

pub(crate) fn apply_whitespace_mode(input: &str, whitespace: WhitespaceMode) -> String {
    match whitespace {
        WhitespaceMode::Preserve => input.trim().to_owned(),
        WhitespaceMode::Normalize => {
            let mut lines = Vec::new();
            let mut blank_streak = 0usize;

            for line in input.lines() {
                let trimmed = collapse_inline_whitespace(line);
                if trimmed.is_empty() {
                    blank_streak += 1;
                    lines.extend((blank_streak == 1).then_some(String::new()));
                } else {
                    blank_streak = 0;
                    lines.push(trimmed);
                }
            }

            lines.join("\n").trim().to_owned()
        }
    }
}

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
            for (_, value) in element
                .attrs
                .iter_mut()
                .filter(|(name, _)| URL_ATTRIBUTE_NAMES.contains(&name.local.as_ref()))
            {
                *value = StrTendril::from(resolve_url(value, Some(base_url)));
            }
        }
    }
}

/// Returns whether an attribute participates in HTMLCut's URL-rewrite normalization.
pub(crate) fn attribute_supports_url_rewrite(name: &str) -> bool {
    URL_ATTRIBUTE_NAMES.contains(&name)
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

pub(crate) fn first_body(document: &Html) -> Option<ElementRef<'_>> {
    select_first(document, "body")
}

pub(crate) fn first_body_child_element(document: &Html) -> Option<ElementRef<'_>> {
    let body = first_body(document)?;
    body.child_elements().next()
}

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
    select_first(document, "title").and_then(|title| {
        let text = render_html_as_text(&serialize_children(&title), WhitespaceMode::Normalize);
        (!text.is_empty()).then_some(text)
    })
}

pub(crate) fn heading_level(tag_name: &str) -> Option<u8> {
    tag_name
        .strip_prefix('h')
        .and_then(|level| level.parse::<u8>().ok())
        .filter(|level| (1..=6).contains(level))
}
