use ego_tree::NodeRef as DomNodeRef;
use scraper::{ElementRef, Html, Node};

use crate::contracts::WhitespaceMode;

use super::parse::{first_body, parse_wrapped_fragment};

pub(crate) const ELLIPSIS: &str = "...";
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

pub(crate) fn render_html_as_text(fragment: &str, whitespace: WhitespaceMode) -> String {
    let document = parse_wrapped_fragment(fragment);
    render_document_body_as_text(&document, whitespace)
}

pub(crate) fn render_document_body_as_text(document: &Html, whitespace: WhitespaceMode) -> String {
    if let Some(body) = first_body(document) {
        render_children_as_text(body.children(), whitespace)
    } else {
        render_children_as_text(document.root_element().children(), whitespace)
    }
}

pub(crate) fn render_element_children_as_text(
    node: &ElementRef<'_>,
    whitespace: WhitespaceMode,
) -> String {
    render_children_as_text(node.children(), whitespace)
}

pub(crate) fn render_element_as_text(node: &ElementRef<'_>, whitespace: WhitespaceMode) -> String {
    let mut output = String::new();
    render_node(**node, &mut output, false, false);
    normalize_rendered_output(output, whitespace)
}

fn render_children_as_text<'a>(
    children: impl Iterator<Item = DomNodeRef<'a, Node>>,
    whitespace: WhitespaceMode,
) -> String {
    let mut output = String::new();
    for child in children {
        render_node(child, &mut output, false, false);
    }

    normalize_rendered_output(output, whitespace)
}

fn normalize_rendered_output(output: String, whitespace: WhitespaceMode) -> String {
    let normalized = collapse_blank_lines(
        &output
            .lines()
            .map(str::trim_end)
            .collect::<Vec<_>>()
            .join("\n"),
    );

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

            if tag_name == "img" {
                let alt_text = data.attr("alt").map(|alt| {
                    if in_pre {
                        alt.to_owned()
                    } else {
                        collapse_inline_whitespace(alt)
                    }
                });
                let Some(alt_text) = alt_text.filter(|alt| !alt.is_empty()) else {
                    return;
                };

                if needs_space(output, &alt_text) {
                    output.push(' ');
                }
                output.push_str(&alt_text);
                return;
            }

            if tag_name == "li" {
                push_newline(output, 1);
                output.push_str(&list_item_marker(node));
                for child in node.children() {
                    render_node(child, output, false, true);
                }
                push_newline(output, 1);
                return;
            }

            if tag_name == "code" && !in_pre {
                let rendered = render_children_to_string(node, true, false);
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
                let rendered = render_children_to_string(node, false, false);
                push_prefixed_block(output, rendered.trim(), "> ");
                push_newline(output, 2);
                return;
            }

            if tag_name == "dl" {
                push_newline(output, 2);
                for child in node.children() {
                    render_node(child, output, false, false);
                }
                push_newline(output, 2);
                return;
            }

            if tag_name == "dt" {
                push_newline(output, 2);
                for child in node.children() {
                    render_node(child, output, false, false);
                }
                push_newline(output, 1);
                return;
            }

            if tag_name == "dd" {
                push_newline(output, 1);
                output.push_str(": ");
                for child in node.children() {
                    render_node(child, output, false, true);
                }
                push_newline(output, 2);
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

fn list_item_marker(node: DomNodeRef<'_, Node>) -> String {
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

fn parse_list_ordinal(value: &str) -> Option<i64> {
    value.parse().ok()
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
    let Some(last_character) = output.chars().next_back() else {
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
    let trimmed_len = output.trim_end_matches('\n').len();
    output.truncate(trimmed_len);
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

fn render_children_to_string(node: DomNodeRef<'_, Node>, in_pre: bool, list_item: bool) -> String {
    let mut rendered = String::new();
    for child in node.children() {
        render_node(child, &mut rendered, in_pre, list_item);
    }
    rendered
}

fn push_prefixed_block(output: &mut String, block: &str, prefix: &str) {
    if block.is_empty() {
        return;
    }

    let normalized = collapse_blank_lines(block);
    for (index, line) in normalized.lines().enumerate() {
        if index > 0 {
            output.push('\n');
        }
        output.push_str(prefix);
        if !line.is_empty() {
            output.push_str(line);
        }
    }
}

fn collapse_blank_lines(input: &str) -> String {
    let mut collapsed = String::with_capacity(input.len());
    let mut consecutive_newlines = 0usize;

    for ch in input.chars() {
        if ch == '\n' {
            if consecutive_newlines < 2 {
                collapsed.push(ch);
            }
            consecutive_newlines += 1;
        } else {
            consecutive_newlines = 0;
            collapsed.push(ch);
        }
    }

    collapsed
}

#[cfg(test)]
pub(crate) fn collapse_blank_lines_for_tests(input: &str) -> String {
    collapse_blank_lines(input)
}
