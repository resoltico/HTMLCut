use std::collections::BTreeMap;

#[cfg(test)]
use ego_tree::NodeRef as DomNodeRef;
#[cfg(test)]
use scraper::Node;
use scraper::{ElementRef, Html, Selector};

use super::render::render_html_as_text;
use super::urls::rewrite_attribute_value;
use crate::contracts::WhitespaceMode;

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
    let tag_name = node.value().name();
    let is_meta_refresh = node
        .value()
        .attrs()
        .any(|(name, value)| name == "http-equiv" && value.eq_ignore_ascii_case("refresh"));

    for (name, value) in node.value().attrs() {
        let attribute_value = if rewrite_urls {
            rewrite_attribute_value(tag_name, name, value, base_url, is_meta_refresh)
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

pub(crate) fn first_body(document: &Html) -> Option<ElementRef<'_>> {
    select_first(document, "body")
}

pub(crate) fn first_body_child_element(document: &Html) -> Option<ElementRef<'_>> {
    let body = first_body(document)?;
    body.child_elements().next()
}

pub(crate) fn text_from_title(document: &Html) -> Option<String> {
    select_first(document, "title").and_then(|title| {
        let text = render_html_as_text(&serialize_children(&title), WhitespaceMode::Normalize);
        (!text.is_empty()).then_some(text)
    })
}
