use ego_tree::NodeRef as DomNodeRef;
use scraper::Node;

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
    let body = first_body(&document).expect("wrapped fragments always contain a body");

    let mut output = String::new();
    for child in body.children() {
        render_node(child, &mut output, false, false);
    }

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

            if tag_name == "li" {
                push_newline(output, 1);
                output.push_str("- ");
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
    let mut collapsed = input.to_owned();
    while collapsed.contains("\n\n\n") {
        collapsed = collapsed.replace("\n\n\n", "\n\n");
    }
    collapsed
}

#[cfg(test)]
pub(crate) fn collapse_blank_lines_for_tests(input: &str) -> String {
    collapse_blank_lines(input)
}
