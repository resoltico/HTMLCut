use ego_tree::NodeRef as DomNodeRef;
use scraper::{ElementRef, Node};

use super::super::super::summary::heading_level;
use super::super::super::urls::href_is_meaningful_destination;
use super::super::policy::{
    element_has_hidden_style, element_should_skip_in_reader_text,
    node_starts_terminal_non_narrative_section, should_skip_rendered_element,
};
use super::super::signals::element_looks_like_utility_chrome;
use super::format::{
    collapse_blank_lines, collapse_inline_whitespace, needs_space, normalize_rendered_output,
    normalize_structured_line, push_newline, push_prefixed_block,
};
use super::list::render_list_item_with_intent;
use super::math::render_math_element;
use super::media::image_has_caption_context;
use super::table::render_table;
use super::{BLOCK_TAGS, SKIP_TAGS, TextRenderIntent, extract_heading_text};
use crate::contracts::WhitespaceMode;

pub(super) fn render_children_as_text<'a>(
    children: impl Iterator<Item = DomNodeRef<'a, Node>>,
    whitespace: WhitespaceMode,
    intent: TextRenderIntent,
) -> String {
    let mut output = String::new();
    render_child_nodes(
        children,
        &mut output,
        false,
        false,
        true,
        intent,
        matches!(intent, TextRenderIntent::SelectedFragment),
    );

    normalize_rendered_output(output, whitespace)
}

#[cfg(test)]
pub(crate) fn render_node(
    node: DomNodeRef<'_, Node>,
    output: &mut String,
    in_pre: bool,
    list_item: bool,
) {
    render_node_with_intent(
        node,
        output,
        in_pre,
        list_item,
        TextRenderIntent::ReaderDocument,
        false,
    );
}

pub(super) fn render_node_with_intent(
    node: DomNodeRef<'_, Node>,
    output: &mut String,
    in_pre: bool,
    list_item: bool,
    intent: TextRenderIntent,
    selected_root: bool,
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

            let element = ElementRef::wrap(node).expect("element nodes must wrap as ElementRef");
            if tag_name == "math" {
                if let Some(rendered) = render_math_element(&element) {
                    push_inline_text(output, &rendered);
                }
                return;
            }

            if let Some(rendered) = hidden_math_replacement(&element) {
                push_inline_text(output, &rendered);
                return;
            }

            if should_skip_rendered_element(&element, intent, selected_root) {
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

            if tag_name == "img" {
                if ElementRef::wrap(node).is_some_and(|element| image_has_caption_context(&element))
                {
                    return;
                }
                let alt_text = data.attr("alt").map(collapse_inline_whitespace);
                let Some(alt_text) = alt_text.filter(|alt| !alt.is_empty()) else {
                    return;
                };

                if needs_space(output, &alt_text) {
                    output.push(' ');
                }
                output.push_str(&alt_text);
                return;
            }

            if tag_name == "a" {
                let rendered = render_anchor(node, in_pre, intent);
                if rendered.is_empty() {
                    return;
                }
                if needs_space(output, &rendered) {
                    output.push(' ');
                }
                output.push_str(&rendered);
                return;
            }

            if let Some(level) = heading_level(tag_name) {
                let Some(heading_text) =
                    ElementRef::wrap(node).and_then(|element| extract_heading_text(&element))
                else {
                    return;
                };
                push_newline(output, 2);
                output.push_str(&"#".repeat(level as usize));
                output.push(' ');
                output.push_str(&heading_text);
                push_newline(output, 2);
                return;
            }

            if tag_name == "p"
                && paragraph_looks_like_shouty_link_banner_with_intent(node, in_pre, intent)
            {
                return;
            }

            if tag_name == "ul" || tag_name == "ol" {
                push_newline(output, 2);
                render_child_nodes(node.children(), output, false, false, false, intent, false);
                push_newline(output, 2);
                return;
            }

            if tag_name == "li" {
                render_list_item_with_intent(node, output, in_pre, intent);
                return;
            }

            if tag_name == "code" && !in_pre {
                let rendered = render_children_to_string(node, true, false, intent);
                if rendered.trim().is_empty() {
                    return;
                }
                if needs_space(output, "`") {
                    output.push(' ');
                }
                output.push('`');
                output.push_str(rendered.trim());
                output.push('`');
                return;
            }

            if tag_name == "blockquote" {
                push_newline(output, 2);
                let rendered = render_children_to_string(node, false, false, intent);
                push_prefixed_block(output, rendered.trim(), "> ");
                push_newline(output, 2);
                return;
            }

            if tag_name == "dl" {
                push_newline(output, 2);
                render_child_nodes(node.children(), output, false, false, false, intent, false);
                push_newline(output, 2);
                return;
            }

            if tag_name == "table" {
                let rendered = render_table(node, in_pre, intent);
                if rendered.is_empty() {
                    return;
                }
                push_newline(output, 2);
                output.push_str(&rendered);
                push_newline(output, 2);
                return;
            }

            if tag_name == "dt" {
                push_newline(output, 2);
                render_child_nodes(node.children(), output, false, false, false, intent, false);
                push_newline(output, 1);
                return;
            }

            if tag_name == "dd" {
                push_newline(output, 1);
                output.push_str(": ");
                render_child_nodes(node.children(), output, false, true, false, intent, false);
                push_newline(output, 2);
                return;
            }

            if let Some(label_value_row) = render_label_value_row_with_intent(node, in_pre, intent)
            {
                push_newline(output, 2);
                output.push_str(&label_value_row);
                push_newline(output, 2);
                return;
            }

            let is_block = BLOCK_TAGS.contains(&tag_name);
            if is_block && !list_item {
                push_newline(output, 2);
            }

            let child_in_pre = in_pre || tag_name == "pre";
            render_child_nodes(
                node.children(),
                output,
                child_in_pre,
                false,
                true,
                intent,
                false,
            );

            if is_block {
                push_newline(output, 2);
            }
        }
        _ => {
            for child in node.children() {
                render_node_with_intent(child, output, in_pre, list_item, intent, false);
            }
        }
    }
}

pub(super) fn render_heading_text_node(
    node: DomNodeRef<'_, Node>,
    output: &mut String,
    in_pre: bool,
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

            let element = ElementRef::wrap(node).expect("element nodes must wrap as ElementRef");
            if tag_name == "math" {
                if let Some(rendered) = render_math_element(&element) {
                    push_inline_text(output, &rendered);
                }
                return;
            }

            if let Some(rendered) = hidden_math_replacement(&element) {
                push_inline_text(output, &rendered);
                return;
            }

            if element_should_skip_in_reader_text(&element) {
                return;
            }

            if tag_name != "button" && element_looks_like_utility_chrome(&element) {
                return;
            }

            if tag_name == "br" {
                push_newline(output, 1);
                return;
            }

            if tag_name == "img" {
                let alt_text = data.attr("alt").map(collapse_inline_whitespace);
                let Some(alt_text) = alt_text.filter(|alt| !alt.is_empty()) else {
                    return;
                };
                if needs_space(output, &alt_text) {
                    output.push(' ');
                }
                output.push_str(&alt_text);
                return;
            }

            let child_in_pre = in_pre || tag_name == "pre";
            for child in node.children() {
                render_heading_text_node(child, output, child_in_pre);
            }
        }
        _ => {
            for child in node.children() {
                render_heading_text_node(child, output, in_pre);
            }
        }
    }
}

fn render_anchor(node: DomNodeRef<'_, Node>, in_pre: bool, intent: TextRenderIntent) -> String {
    let label = render_children_to_string(node, in_pre, false, intent);
    let label = if in_pre {
        label.trim_matches('\n').to_owned()
    } else {
        collapse_inline_whitespace(label.trim())
    };
    let href = ElementRef::wrap(node)
        .and_then(|element| element.value().attr("href"))
        .map(str::trim)
        .filter(|href| href_is_meaningful_destination(href));

    match (label.is_empty(), href) {
        (true, _) => String::new(),
        (false, Some(href)) if label == href => label,
        (false, Some(href)) => format!("{label} [{href}]"),
        (false, None) => label,
    }
}

pub(super) fn push_inline_text(output: &mut String, text: &str) {
    if text.is_empty() {
        return;
    }
    if needs_space(output, text) {
        output.push(' ');
    }
    output.push_str(text);
}

pub(super) fn hidden_math_replacement(element: &ElementRef<'_>) -> Option<String> {
    if !element_has_hidden_style(element) {
        return None;
    }

    element
        .descendants()
        .filter_map(ElementRef::wrap)
        .find(|descendant| descendant.value().name() == "math")
        .and_then(|math| render_math_element(&math))
}

fn render_child_nodes<'a>(
    children: impl Iterator<Item = DomNodeRef<'a, Node>>,
    output: &mut String,
    in_pre: bool,
    list_item: bool,
    stop_at_terminal_auxiliary: bool,
    intent: TextRenderIntent,
    selected_root_children: bool,
) {
    let mut rendered_substantive = output.chars().any(|character| !character.is_whitespace());
    for child in children {
        if stop_at_terminal_auxiliary
            && rendered_substantive
            && node_starts_terminal_non_narrative_section(child)
        {
            break;
        }
        let before_len = output.len();
        render_node_with_intent(
            child,
            output,
            in_pre,
            list_item,
            intent,
            selected_root_children,
        );
        if output.len() > before_len {
            rendered_substantive = true;
        }
    }
}

#[cfg(test)]
pub(super) fn paragraph_looks_like_shouty_link_banner(
    node: DomNodeRef<'_, Node>,
    in_pre: bool,
) -> bool {
    paragraph_looks_like_shouty_link_banner_with_intent(
        node,
        in_pre,
        TextRenderIntent::ReaderDocument,
    )
}

fn paragraph_looks_like_shouty_link_banner_with_intent(
    node: DomNodeRef<'_, Node>,
    in_pre: bool,
    intent: TextRenderIntent,
) -> bool {
    let Some(anchor) = direct_anchor_child(node) else {
        return false;
    };

    let anchor_text = collapse_inline_whitespace(
        render_children_to_string(*anchor, in_pre, false, intent).trim(),
    );
    !anchor_text.is_empty() && looks_like_shouty_banner(&anchor_text)
}

pub(super) fn direct_anchor_child(node: DomNodeRef<'_, Node>) -> Option<ElementRef<'_>> {
    let mut anchor = None;

    for child in node.children() {
        match child.value() {
            Node::Text(contents) if contents.trim().is_empty() => {}
            Node::Element(_) => {
                let element = ElementRef::wrap(child)?;
                if element.value().name() != "a" || anchor.is_some() {
                    return None;
                }
                anchor = Some(element);
            }
            _ => {
                return None;
            }
        }
    }

    anchor
}

pub(super) fn direct_child_elements(node: DomNodeRef<'_, Node>) -> Vec<ElementRef<'_>> {
    node.children().filter_map(ElementRef::wrap).collect()
}

#[cfg(test)]
pub(super) fn render_label_value_row(node: DomNodeRef<'_, Node>, in_pre: bool) -> Option<String> {
    render_label_value_row_with_intent(node, in_pre, TextRenderIntent::ReaderDocument)
}

fn render_label_value_row_with_intent(
    node: DomNodeRef<'_, Node>,
    in_pre: bool,
    intent: TextRenderIntent,
) -> Option<String> {
    let element = ElementRef::wrap(node)?;
    if !matches!(element.value().name(), "div" | "section") || !direct_text_is_whitespace_only(node)
    {
        return None;
    }

    let children = direct_child_elements(node);
    if children.len() != 2 {
        return None;
    }

    let left = render_compact_block_text(children[0], in_pre, intent)?;
    let right = render_compact_block_text(children[1], in_pre, intent)?;
    if !left.ends_with(':')
        || left.chars().count() > 60
        || right.chars().count() > 160
        || left.contains('|')
        || right.contains('|')
    {
        return None;
    }

    Some(format!("{left} {right}"))
}

pub(super) fn direct_text_is_whitespace_only(node: DomNodeRef<'_, Node>) -> bool {
    node.children().all(|child| {
        !matches!(
            child.value(),
            Node::Text(contents) if !contents.trim().is_empty()
        )
    })
}

fn render_compact_block_text(
    element: ElementRef<'_>,
    in_pre: bool,
    intent: TextRenderIntent,
) -> Option<String> {
    let rendered = render_children_to_string(*element, in_pre, false, intent);
    let compact = collapse_blank_lines(&rendered)
        .lines()
        .map(normalize_structured_line)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    (!compact.is_empty()).then_some(compact)
}

pub(super) fn looks_like_shouty_banner(text: &str) -> bool {
    let mut uppercase_letters = 0usize;
    let mut lowercase_letters = 0usize;

    for character in text.chars() {
        match character {
            character if !character.is_alphabetic() => {}
            character if character.is_uppercase() => uppercase_letters += 1,
            character if character.is_lowercase() => lowercase_letters += 1,
            _ => {}
        }
    }

    uppercase_letters >= 8 && lowercase_letters == 0
}

pub(super) fn render_children_to_string(
    node: DomNodeRef<'_, Node>,
    in_pre: bool,
    list_item: bool,
    intent: TextRenderIntent,
) -> String {
    let mut rendered = String::new();
    for child in node.children() {
        render_node_with_intent(child, &mut rendered, in_pre, list_item, intent, false);
    }
    rendered
}

pub(super) fn render_node_to_string(
    node: DomNodeRef<'_, Node>,
    in_pre: bool,
    list_item: bool,
    intent: TextRenderIntent,
) -> String {
    let mut rendered = String::new();
    render_node_with_intent(node, &mut rendered, in_pre, list_item, intent, false);
    rendered
}
