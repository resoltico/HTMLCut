//! Compact navigation and utility-widget classification.

use scraper::{ElementRef, Node};

pub(super) fn element_looks_like_compact_utility_widget(
    element: &ElementRef<'_>,
    content_count: usize,
) -> bool {
    if content_count > 0
        || !matches!(element.value().name(), "aside" | "div" | "section")
        || has_heading_ancestor(element)
    {
        return false;
    }

    let mut child_element_count = 0usize;
    let mut link_count = 0usize;
    let mut image_count = 0usize;
    let mut text_chars = 0usize;

    for descendant in element.descendants() {
        match descendant.value() {
            Node::Element(data) => {
                if descendant.id() != element.id() {
                    child_element_count += 1;
                    if matches!(
                        data.name(),
                        "blockquote"
                            | "dd"
                            | "dl"
                            | "dt"
                            | "figcaption"
                            | "figure"
                            | "h1"
                            | "h2"
                            | "h3"
                            | "h4"
                            | "h5"
                            | "h6"
                            | "li"
                            | "ol"
                            | "p"
                            | "picture"
                            | "pre"
                            | "table"
                            | "ul"
                            | "video"
                    ) {
                        return false;
                    }

                    if data.name() == "img" {
                        image_count += 1;
                        if image_count > 1 {
                            return false;
                        }
                    }
                }

                if data.name() == "a" {
                    link_count += 1;
                    if link_count > if image_count > 0 { 2 } else { 1 } {
                        return false;
                    }
                }
            }
            Node::Text(contents) => {
                text_chars += contents
                    .chars()
                    .filter(|character| !character.is_whitespace())
                    .count();
                if text_chars > 64 {
                    return false;
                }
            }
            _ => {}
        }
    }

    child_element_count > 0 && text_chars > 0
}

pub(super) fn has_heading_ancestor(element: &ElementRef<'_>) -> bool {
    let mut parent = element.parent();
    while let Some(current) = parent {
        if let Some(parent_element) = ElementRef::wrap(current)
            && matches!(
                parent_element.value().name(),
                "h1" | "h2" | "h3" | "h4" | "h5" | "h6"
            )
        {
            return true;
        }
        parent = current.parent();
    }

    false
}
