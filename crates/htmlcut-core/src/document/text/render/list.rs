use ego_tree::NodeRef as DomNodeRef;
use scraper::{ElementRef, Node};

use super::format::push_newline;
use super::tree::{render_node_to_string, render_node_with_intent};
use super::{BLOCK_TAGS, TextRenderIntent};

pub(super) fn list_item_marker(node: DomNodeRef<'_, Node>) -> String {
    let Some(parent) = node.parent().and_then(ElementRef::wrap) else {
        return "- ".to_owned();
    };
    if parent.value().name() != "ol" {
        return "- ".to_owned();
    }

    let reversed = parent.value().attr("reversed").is_some();
    let list_items = parent
        .children()
        .filter_map(ElementRef::wrap)
        .filter(|element| element.value().name() == "li")
        .collect::<Vec<_>>();

    let mut ordinal = parent
        .value()
        .attr("start")
        .and_then(parse_list_ordinal)
        .unwrap_or(if reversed { list_items.len() as i64 } else { 1 });

    for list_item in list_items
        .iter()
        .copied()
        .take_while(|list_item| list_item.id() != node.id())
    {
        if let Some(explicit_value) = list_item.value().attr("value").and_then(parse_list_ordinal) {
            ordinal = explicit_value;
        }
        ordinal += if reversed { -1 } else { 1 };
    }

    if let Some(explicit_value) = ElementRef::wrap(node)
        .and_then(|element| element.value().attr("value"))
        .and_then(parse_list_ordinal)
    {
        ordinal = explicit_value;
    }

    format!("{ordinal}. ")
}

#[cfg(test)]
pub(super) fn render_list_item(node: DomNodeRef<'_, Node>, output: &mut String, in_pre: bool) {
    render_list_item_with_intent(node, output, in_pre, TextRenderIntent::ReaderDocument)
}

pub(super) fn render_list_item_with_intent(
    node: DomNodeRef<'_, Node>,
    output: &mut String,
    in_pre: bool,
    intent: TextRenderIntent,
) {
    let indent = "    ".repeat(list_depth(node).saturating_sub(1));
    let marker = list_item_marker(node);
    let continuation = format!("{indent}{}", " ".repeat(marker.chars().count()));

    let mut body_segments = Vec::new();
    let mut nested_lists = Vec::new();
    let mut inline_segment = String::new();

    for child in node.children() {
        if is_list_container(child) {
            flush_list_item_inline_segment(&mut inline_segment, &mut body_segments);
            let nested = render_node_to_string(child, false, false, intent);
            let nested = nested.trim_matches('\n').to_owned();
            nested_lists.push(nested);
            continue;
        }

        if is_list_item_block_segment(child) {
            flush_list_item_inline_segment(&mut inline_segment, &mut body_segments);
            let rendered = render_node_to_string(child, in_pre, false, intent);
            let rendered = rendered.trim_matches('\n').trim().to_owned();
            body_segments.push(rendered);
            continue;
        }

        render_node_with_intent(child, &mut inline_segment, in_pre, true, intent, false);
    }

    flush_list_item_inline_segment(&mut inline_segment, &mut body_segments);
    nested_lists.retain(|nested| !nested.is_empty());
    body_segments.retain(|rendered| !rendered.is_empty());

    if body_segments.is_empty() && nested_lists.is_empty() {
        return;
    }

    push_newline(output, 1);

    let body = body_segments.join("\n\n");
    if body.is_empty() {
        for (index, nested) in nested_lists.iter().enumerate() {
            if index > 0 {
                output.push('\n');
            }
            output.push_str(nested);
        }
        push_newline(output, 1);
        return;
    } else {
        for (index, line) in body.lines().enumerate() {
            if index > 0 {
                output.push('\n');
                output.push_str(&continuation);
            } else {
                output.push_str(&indent);
                output.push_str(&marker);
            }
            output.push_str(line);
        }
    }

    for nested in nested_lists {
        output.push('\n');
        output.push_str(&nested);
    }

    push_newline(output, 1);
}

fn list_depth(node: DomNodeRef<'_, Node>) -> usize {
    let mut depth = 0usize;
    let mut parent = node.parent();
    while let Some(current) = parent {
        if let Some(element) = ElementRef::wrap(current)
            && matches!(element.value().name(), "ul" | "ol")
        {
            depth += 1;
        }
        parent = current.parent();
    }
    depth
}

fn is_list_container(node: DomNodeRef<'_, Node>) -> bool {
    ElementRef::wrap(node)
        .map(|element| matches!(element.value().name(), "ul" | "ol"))
        .unwrap_or(false)
}

pub(super) fn is_list_item_block_segment(node: DomNodeRef<'_, Node>) -> bool {
    ElementRef::wrap(node)
        .map(|element| {
            BLOCK_TAGS.contains(&element.value().name())
                || matches!(element.value().name(), "blockquote" | "dl" | "hr" | "pre")
        })
        .unwrap_or(false)
}

pub(super) fn flush_list_item_inline_segment(
    inline_segment: &mut String,
    body_segments: &mut Vec<String>,
) {
    let rendered = inline_segment.trim_matches('\n').trim().to_owned();
    if !rendered.is_empty() {
        body_segments.push(rendered);
    }
    inline_segment.clear();
}

fn parse_list_ordinal(value: &str) -> Option<i64> {
    value.parse().ok()
}
