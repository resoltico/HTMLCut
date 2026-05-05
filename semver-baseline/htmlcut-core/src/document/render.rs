use ego_tree::NodeRef as DomNodeRef;
use scraper::{ElementRef, Html, Node};

use crate::contracts::WhitespaceMode;

use super::parse::{first_body, parse_wrapped_fragment};
use super::signals::{
    element_looks_like_utility_chrome, structural_signal_tokens, token_match_count,
};
use super::summary::heading_level;
use super::urls::href_is_meaningful_destination;

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

pub(crate) fn extract_heading_text(node: &ElementRef<'_>) -> Option<String> {
    let mut rendered = String::new();
    for child in node.children() {
        render_heading_text_node(child, &mut rendered, false);
    }

    let heading_text = normalize_heading_text(&rendered);
    (!heading_text.is_empty()).then_some(heading_text)
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
    let normalized = remove_immediate_heading_echoes(&collapse_blank_lines(
        &output
            .lines()
            .map(str::trim_end)
            .collect::<Vec<_>>()
            .join("\n"),
    ));

    apply_whitespace_mode(normalized.trim_matches('\n'), whitespace)
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

            let element = ElementRef::wrap(node).expect("element nodes must wrap as ElementRef");
            if element_looks_like_utility_chrome(&element) {
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
                let rendered = render_anchor(node, in_pre);
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

            if tag_name == "p" && paragraph_looks_like_shouty_link_banner(node, in_pre) {
                return;
            }

            if tag_name == "ul" || tag_name == "ol" {
                push_newline(output, 2);
                for child in node.children() {
                    render_node(child, output, false, false);
                }
                push_newline(output, 2);
                return;
            }

            if tag_name == "li" {
                render_list_item(node, output, in_pre);
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

            if tag_name == "table" {
                let rendered = render_table(node, in_pre);
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

            if let Some(label_value_row) = render_label_value_row(node, in_pre) {
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

fn render_heading_text_node(node: DomNodeRef<'_, Node>, output: &mut String, in_pre: bool) {
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

fn render_anchor(node: DomNodeRef<'_, Node>, in_pre: bool) -> String {
    let label = render_children_to_string(node, in_pre, false);
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

fn normalize_heading_text(rendered: &str) -> String {
    collapse_inline_whitespace(rendered.trim())
}

fn render_table(node: DomNodeRef<'_, Node>, in_pre: bool) -> String {
    let caption = table_caption(node, in_pre);
    let mut rows = Vec::<Vec<String>>::new();
    collect_table_rows(node, in_pre, &mut rows);
    rows.retain(|row| row.iter().any(|cell| !cell.is_empty()));
    if rows.is_empty() {
        return caption.unwrap_or_default();
    }

    let column_count = rows
        .iter()
        .map(Vec::len)
        .max()
        .expect("non-empty rendered tables must have at least one column");

    for row in &mut rows {
        row.resize(column_count, String::new());
    }

    let widths = (0..column_count)
        .map(|column_index| {
            rows.iter()
                .map(|row| row[column_index].chars().count())
                .max()
                .unwrap_or(0)
        })
        .collect::<Vec<_>>();

    let rendered_rows = rows
        .into_iter()
        .map(|row| format_table_row(&row, &widths))
        .collect::<Vec<_>>()
        .join("\n")
        .trim_matches('\n')
        .to_owned();

    match caption {
        Some(caption) => format!("{caption}\n{rendered_rows}"),
        None => rendered_rows,
    }
}

fn image_has_caption_context(element: &ElementRef<'_>) -> bool {
    let mut ancestor = element.parent();
    let mut depth = 0usize;

    while let Some(current) = ancestor {
        let Some(ancestor_element) = ElementRef::wrap(current) else {
            ancestor = current.parent();
            depth += 1;
            continue;
        };

        if matches!(ancestor_element.value().name(), "figure" | "figcaption") {
            return true;
        }

        for descendant in ancestor_element.descendants().filter_map(ElementRef::wrap) {
            if descendant.id() == element.id() {
                continue;
            }
            if descendant.value().name() == "figcaption" {
                return true;
            }
            let tokens = structural_signal_tokens(&descendant);
            if token_match_count(&tokens, &["caption"]) > 0 {
                return true;
            }
        }

        depth += 1;
        if depth >= 3 {
            break;
        }
        ancestor = current.parent();
    }

    false
}

fn collect_table_rows(node: DomNodeRef<'_, Node>, in_pre: bool, rows: &mut Vec<Vec<String>>) {
    let Some(element) = ElementRef::wrap(node) else {
        return;
    };

    match element.value().name() {
        "tr" => {
            let row = direct_child_elements(node)
                .into_iter()
                .filter(|cell| matches!(cell.value().name(), "td" | "th"))
                .map(|cell| render_table_cell(cell, in_pre))
                .collect::<Vec<_>>();
            if !row.is_empty() {
                rows.push(row);
            }
        }
        "table" | "thead" | "tbody" | "tfoot" => {
            for child in node.children() {
                collect_table_rows(child, in_pre, rows);
            }
        }
        _ => {}
    }
}

fn render_table_cell(cell: ElementRef<'_>, in_pre: bool) -> String {
    let rendered = render_children_to_string(*cell, in_pre, false);
    normalize_table_cell_text(&rendered)
}

fn table_caption(node: DomNodeRef<'_, Node>, in_pre: bool) -> Option<String> {
    direct_child_elements(node)
        .into_iter()
        .find(|child| child.value().name() == "caption")
        .and_then(|caption| {
            let rendered = render_children_to_string(*caption, in_pre, false);
            let normalized = collapse_blank_lines(&rendered)
                .lines()
                .map(normalize_structured_line)
                .filter(|line| !line.is_empty())
                .collect::<Vec<_>>()
                .join(" ");
            (!normalized.is_empty()).then_some(normalized)
        })
}

fn normalize_table_cell_text(rendered: &str) -> String {
    collapse_blank_lines(rendered)
        .lines()
        .map(normalize_structured_line)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" / ")
}

fn format_table_row(row: &[String], widths: &[usize]) -> String {
    let mut line = String::new();

    for (index, cell) in row.iter().enumerate() {
        if index > 0 {
            line.push_str(" | ");
        }

        line.push_str(cell);
        if index + 1 != row.len() {
            line.push_str(&" ".repeat(widths[index].saturating_sub(cell.chars().count())));
        }
    }

    line.trim_end().to_owned()
}

fn paragraph_looks_like_shouty_link_banner(node: DomNodeRef<'_, Node>, in_pre: bool) -> bool {
    let Some(anchor) = direct_anchor_child(node) else {
        return false;
    };

    let anchor_text =
        collapse_inline_whitespace(render_children_to_string(*anchor, in_pre, false).trim());
    !anchor_text.is_empty() && looks_like_shouty_banner(&anchor_text)
}

fn direct_anchor_child(node: DomNodeRef<'_, Node>) -> Option<ElementRef<'_>> {
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

fn direct_child_elements(node: DomNodeRef<'_, Node>) -> Vec<ElementRef<'_>> {
    node.children().filter_map(ElementRef::wrap).collect()
}

fn render_label_value_row(node: DomNodeRef<'_, Node>, in_pre: bool) -> Option<String> {
    let element = ElementRef::wrap(node)?;
    if !matches!(element.value().name(), "div" | "section") || !direct_text_is_whitespace_only(node)
    {
        return None;
    }

    let children = direct_child_elements(node);
    if children.len() != 2 {
        return None;
    }

    let left = render_compact_block_text(children[0], in_pre)?;
    let right = render_compact_block_text(children[1], in_pre)?;
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

fn direct_text_is_whitespace_only(node: DomNodeRef<'_, Node>) -> bool {
    node.children().all(|child| {
        !matches!(
            child.value(),
            Node::Text(contents) if !contents.trim().is_empty()
        )
    })
}

fn render_compact_block_text(element: ElementRef<'_>, in_pre: bool) -> Option<String> {
    let rendered = render_children_to_string(*element, in_pre, false);
    let compact = collapse_blank_lines(&rendered)
        .lines()
        .map(normalize_structured_line)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    (!compact.is_empty()).then_some(compact)
}

fn looks_like_shouty_banner(text: &str) -> bool {
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

fn render_list_item(node: DomNodeRef<'_, Node>, output: &mut String, in_pre: bool) {
    let indent = "    ".repeat(list_depth(node).saturating_sub(1));
    let marker = list_item_marker(node);
    let continuation = format!("{indent}{}", " ".repeat(marker.chars().count()));

    let mut body_segments = Vec::new();
    let mut nested_lists = Vec::new();
    let mut inline_segment = String::new();

    for child in node.children() {
        if is_list_container(child) {
            flush_list_item_inline_segment(&mut inline_segment, &mut body_segments);
            let nested = render_node_to_string(child, false, false);
            let nested = nested.trim_matches('\n').to_owned();
            nested_lists.push(nested);
            continue;
        }

        if is_list_item_block_segment(child) {
            flush_list_item_inline_segment(&mut inline_segment, &mut body_segments);
            let rendered = render_node_to_string(child, in_pre, false);
            let rendered = rendered.trim_matches('\n').trim().to_owned();
            body_segments.push(rendered);
            continue;
        }

        render_node(child, &mut inline_segment, in_pre, true);
    }

    flush_list_item_inline_segment(&mut inline_segment, &mut body_segments);
    nested_lists.retain(|nested| !nested.is_empty());
    body_segments.retain(|rendered| !rendered.is_empty());

    push_newline(output, 1);

    let body = body_segments.join("\n\n");
    if body.is_empty() {
        output.push_str(&indent);
        output.push_str(&marker);
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

fn is_list_item_block_segment(node: DomNodeRef<'_, Node>) -> bool {
    ElementRef::wrap(node)
        .map(|element| {
            BLOCK_TAGS.contains(&element.value().name())
                || matches!(element.value().name(), "blockquote" | "dl" | "hr" | "pre")
        })
        .unwrap_or(false)
}

fn flush_list_item_inline_segment(inline_segment: &mut String, body_segments: &mut Vec<String>) {
    let rendered = inline_segment.trim_matches('\n').trim().to_owned();
    if !rendered.is_empty() {
        body_segments.push(rendered);
    }
    inline_segment.clear();
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
        WhitespaceMode::Preserve => input.trim_matches('\n').to_owned(),
        WhitespaceMode::Normalize => {
            let mut lines = Vec::new();
            let mut blank_streak = 0usize;

            for line in input.lines() {
                let trimmed = normalize_structured_line(line);
                if trimmed.is_empty() {
                    blank_streak += 1;
                    lines.extend((blank_streak == 1).then_some(String::new()));
                } else {
                    blank_streak = 0;
                    lines.push(trimmed);
                }
            }

            lines.join("\n").trim_matches('\n').to_owned()
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

fn render_node_to_string(node: DomNodeRef<'_, Node>, in_pre: bool, list_item: bool) -> String {
    let mut rendered = String::new();
    render_node(node, &mut rendered, in_pre, list_item);
    rendered
}

fn normalize_structured_line(line: &str) -> String {
    let trimmed_start = line.trim_start();
    let indent = &line[..line.len() - trimmed_start.len()];
    let collapsed = collapse_inline_whitespace(trimmed_start);
    if collapsed.is_empty() {
        String::new()
    } else {
        format!("{indent}{collapsed}")
    }
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

fn remove_immediate_heading_echoes(input: &str) -> String {
    let lines = input.lines().collect::<Vec<_>>();
    let mut output = Vec::<String>::new();
    let mut index = 0usize;

    while index < lines.len() {
        let current = lines[index];
        output.push(current.to_owned());

        if let Some(heading_text) = current
            .strip_prefix('#')
            .map(|_| current.trim_start_matches('#').trim())
            .filter(|heading_text| !heading_text.is_empty())
            && lines.get(index + 1) == Some(&"")
            && lines
                .get(index + 2)
                .is_some_and(|line| line.trim() == heading_text)
        {
            index += 3;
            index += usize::from(lines.get(index) == Some(&""));
            output.push(String::new());
            continue;
        }

        index += 1;
    }

    output.join("\n")
}

#[cfg(test)]
pub(crate) fn collapse_blank_lines_for_tests(input: &str) -> String {
    collapse_blank_lines(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::WhitespaceMode;
    use crate::document::{parse_document_node, select_first};

    #[test]
    fn helper_branches_cover_heading_table_banner_and_spacing_edges() {
        assert_eq!(
            render_html_as_text(
                "<script>ignore</script><style>.x{}</style><p>Body</p>",
                WhitespaceMode::Preserve,
            ),
            "Body"
        );
        assert_eq!(
            render_html_as_text("<p>Alpha<br>Beta</p>", WhitespaceMode::Preserve),
            "Alpha\nBeta"
        );
        assert_eq!(
            render_html_as_text("<p>Alpha</p><hr><p>Beta</p>", WhitespaceMode::Preserve),
            "Alpha\n\n---\n\nBeta"
        );
        assert_eq!(
            render_html_as_text(
                "<p><a href=\"/empty\"><img alt=\"\" src=\"hero.png\"></a>After</p>",
                WhitespaceMode::Preserve,
            ),
            "After"
        );
        assert_eq!(
            render_html_as_text("<h2>   </h2><p>Body</p>", WhitespaceMode::Preserve),
            "Body"
        );
        assert_eq!(
            render_html_as_text(
                "<dl><dt>Term</dt><dd>Definition</dd></dl>",
                WhitespaceMode::Preserve,
            ),
            "Term\n: Definition"
        );

        let heading_document = parse_document_node(
            "<h2><script>ignored</script><pre>  Keep\n  spacing</pre><br><img alt=\"Hero\">Trail<!--note--></h2>",
        );
        let heading = select_first(&heading_document, "h2").expect("heading");
        assert_eq!(
            extract_heading_text(&heading).as_deref(),
            Some("Keep spacing Hero Trail")
        );
        let empty_heading_image =
            parse_document_node("<h2><img alt=\"   \"><span>Trail</span></h2>");
        let empty_heading_image_heading =
            select_first(&empty_heading_image, "h2").expect("heading");
        assert_eq!(
            extract_heading_text(&empty_heading_image_heading).as_deref(),
            Some("Trail")
        );
        let heading_image_spacing = parse_document_node("<h2>Title<img alt=\"Hero\"></h2>");
        let heading_image_spacing_heading =
            select_first(&heading_image_spacing, "h2").expect("heading");
        assert_eq!(
            extract_heading_text(&heading_image_spacing_heading).as_deref(),
            Some("Title Hero")
        );
        let pre_only_heading =
            parse_document_node("<h2><pre><span>  Keep\n  spacing</span></pre></h2>");
        let mut pre_only_rendered = String::new();
        render_heading_text_node(
            *select_first(&pre_only_heading, "pre").expect("pre"),
            &mut pre_only_rendered,
            true,
        );
        assert!(pre_only_rendered.contains("Keep"));
        let mut root_heading_rendered = String::new();
        render_heading_text_node(
            heading_document.tree.root(),
            &mut root_heading_rendered,
            false,
        );
        assert!(root_heading_rendered.contains("Keep"));

        assert_eq!(
            render_html_as_text(
                "<article><table><tr></tr></table><p>Body.</p></article>",
                WhitespaceMode::Preserve,
            ),
            "Body."
        );
        assert_eq!(
            render_html_as_text(
                "<article><table>\n<tr><td>Alpha</td></tr>\n</table></article>",
                WhitespaceMode::Preserve,
            ),
            "Alpha"
        );
        assert_eq!(
            render_html_as_text(
                "<article><table><caption>Windows builds</caption><tr><td>Alpha</td></tr></table></article>",
                WhitespaceMode::Preserve,
            ),
            "Windows builds\nAlpha"
        );
        assert_eq!(
            render_html_as_text(
                "<article><table><caption>Caption only</caption></table></article>",
                WhitespaceMode::Preserve,
            ),
            "Caption only"
        );
        assert_eq!(
            render_html_as_text(
                "<article><figure><img alt=\"Hero\" src=\"hero.jpg\"><figcaption>Caption</figcaption></figure><div class=\"caption-box\"><img alt=\"Hero Two\" src=\"hero2.jpg\"><div class=\"caption\">Caption</div></div></article>",
                WhitespaceMode::Preserve,
            ),
            ""
        );
        assert_eq!(
            render_html_as_text(
                "<article><div><img alt=\"Hero\" src=\"hero.jpg\"><figcaption>Caption</figcaption></div></article>",
                WhitespaceMode::Preserve,
            ),
            ""
        );
        let single_anchor_parent =
            parse_document_node("<p><a href=\"https://example.com\">Link</a></p>");
        assert!(
            direct_anchor_child(*select_first(&single_anchor_parent, "p").expect("paragraph"))
                .is_some()
        );
        let multiple_anchor_parent = parse_document_node(
            "<p><a href=\"https://example.com/one\">One</a><a href=\"https://example.com/two\">Two</a></p>",
        );
        assert!(
            direct_anchor_child(*select_first(&multiple_anchor_parent, "p").expect("paragraph"))
                .is_none()
        );
        let text_before_anchor =
            parse_document_node("<p>Intro <a href=\"https://example.com\">Link</a></p>");
        assert!(
            direct_anchor_child(*select_first(&text_before_anchor, "p").expect("paragraph"))
                .is_none()
        );
        let non_anchor_parent = parse_document_node("<p><span>Not a link</span></p>");
        assert!(
            direct_anchor_child(*select_first(&non_anchor_parent, "p").expect("paragraph"))
                .is_none()
        );
        let label_value_section = parse_document_node(
            "<section><div><p>Release Date:</p></div><div><p>4/14/2026</p></div></section>",
        );
        assert_eq!(
            render_label_value_row(
                *select_first(&label_value_section, "section").expect("section"),
                false,
            )
            .as_deref(),
            Some("Release Date: 4/14/2026")
        );
        let label_value_missing_right =
            parse_document_node("<section><div><p>Release Date:</p></div><div></div></section>");
        assert_eq!(
            render_label_value_row(
                *select_first(&label_value_missing_right, "section").expect("section"),
                false,
            ),
            None
        );
        let label_value_with_direct_text = parse_document_node(
            "<section>Lead<div><p>Release Date:</p></div><div><p>4/14/2026</p></div></section>",
        );
        assert_eq!(
            render_label_value_row(
                *select_first(&label_value_with_direct_text, "section").expect("section"),
                false,
            ),
            None
        );
        let label_value_missing_colon = parse_document_node(
            "<section><div><p>Release Date</p></div><div><p>4/14/2026</p></div></section>",
        );
        assert_eq!(
            render_label_value_row(
                *select_first(&label_value_missing_colon, "section").expect("section"),
                false,
            ),
            None
        );
        let label_value_three_children = parse_document_node(
            "<section><div><p>Left:</p></div><div><p>Middle</p></div><div><p>Right</p></div></section>",
        );
        assert_eq!(
            render_label_value_row(
                *select_first(&label_value_three_children, "section").expect("section"),
                false,
            ),
            None
        );
        let label_value_piped = parse_document_node(
            "<section><div><p>Left:</p></div><div><p>A | B</p></div></section>",
        );
        assert_eq!(
            render_label_value_row(
                *select_first(&label_value_piped, "section").expect("section"),
                false,
            ),
            None
        );
        let long_label_row = parse_document_node(&format!(
            "<section><div><p>{}:</p></div><div><p>Value</p></div></section>",
            "L".repeat(61)
        ));
        assert_eq!(
            render_label_value_row(
                *select_first(&long_label_row, "section").expect("section"),
                false,
            ),
            None
        );
        let long_value_row = parse_document_node(&format!(
            "<section><div><p>Label:</p></div><div><p>{}</p></div></section>",
            "V".repeat(161)
        ));
        assert_eq!(
            render_label_value_row(
                *select_first(&long_value_row, "section").expect("section"),
                false,
            ),
            None
        );
        let banner_paragraph =
            parse_document_node("<p><a href=\"https://example.com\">BREAKING NEWS</a></p>");
        assert!(paragraph_looks_like_shouty_link_banner(
            *select_first(&banner_paragraph, "p").expect("paragraph"),
            false,
        ));
        let normal_paragraph =
            parse_document_node("<p><a href=\"https://example.com\">Normal headline</a></p>");
        assert!(!paragraph_looks_like_shouty_link_banner(
            *select_first(&normal_paragraph, "p").expect("paragraph"),
            false,
        ));
        let mut prefixed_block = String::new();
        push_prefixed_block(&mut prefixed_block, "", "> ");
        assert!(prefixed_block.is_empty());
        push_prefixed_block(&mut prefixed_block, "Alpha\n\nBeta", "> ");
        assert_eq!(prefixed_block, "> Alpha\n> \n> Beta");
        let mut newline_output = String::new();
        push_newline(&mut newline_output, 2);
        assert!(newline_output.is_empty());
        newline_output.push_str("Alpha\n\n");
        push_newline(&mut newline_output, 1);
        assert_eq!(newline_output, "Alpha\n");
        assert!(!needs_space("(", "word"));
        assert!(!needs_space("word", "."));
        let list_item_block_document =
            parse_document_node("<li><hr></li><li><span>Body</span></li>");
        assert!(is_list_item_block_segment(
            *select_first(&list_item_block_document, "hr").expect("hr")
        ));
        assert!(!is_list_item_block_segment(
            *select_first(&list_item_block_document, "span").expect("span")
        ));
        let non_table_document = parse_document_node("<div><span>Alpha</span></div>");
        let mut rows = Vec::new();
        collect_table_rows(non_table_document.tree.root(), false, &mut rows);
        assert!(rows.is_empty());
        collect_table_rows(
            *select_first(&non_table_document, "div").expect("div"),
            false,
            &mut rows,
        );
        assert!(rows.is_empty());
        let stray_cells =
            parse_document_node("<table><tr><td>Alpha</td><div>Ignored</div></tr></table>");
        rows.clear();
        collect_table_rows(
            *select_first(&stray_cells, "table").expect("table"),
            false,
            &mut rows,
        );
        assert_eq!(rows, vec![vec!["Alpha".to_owned()]]);

        let banner_document =
            parse_document_node("<p><a href=\"/promo\">READ THE FULL TRANSCRIPT HERE</a></p>");
        let banner = select_first(&banner_document, "p").expect("banner");
        assert!(paragraph_looks_like_shouty_link_banner(*banner, false));
        assert!(looks_like_shouty_banner("READ THE FULL TRANSCRIPT HERE"));
        assert!(!looks_like_shouty_banner("Read THE FULL TRANSCRIPT HERE"));
        assert!(!looks_like_shouty_banner("汉字汉字汉字汉字"));
        let non_banner_document =
            parse_document_node("<p><a href=\"/promo\">Read more</a><span>now</span></p>");
        let non_banner = select_first(&non_banner_document, "p").expect("paragraph");
        assert!(!paragraph_looks_like_shouty_link_banner(*non_banner, false));
        let spaced_anchor_document =
            parse_document_node("<p>  <a href=\"/promo\">LOUD BANNER COPY</a>  </p>");
        let spaced_anchor = select_first(&spaced_anchor_document, "p").expect("paragraph");
        assert!(paragraph_looks_like_shouty_link_banner(
            *spaced_anchor,
            false
        ));

        let right_pipe_row =
            parse_document_node("<div><div>Label:</div><div>Value | more</div></div>");
        let right_pipe = select_first(&right_pipe_row, "div").expect("row");
        assert_eq!(render_label_value_row(*right_pipe, false), None);

        let left_pipe_row = parse_document_node("<div><div>La|bel:</div><div>Value</div></div>");
        let left_pipe = select_first(&left_pipe_row, "div").expect("row");
        assert_eq!(render_label_value_row(*left_pipe, false), None);

        let list_document = parse_document_node("<ul><li></li><li>Text</li></ul>");
        let list_items = list_document
            .select(&scraper::Selector::parse("li").expect("li selector"))
            .collect::<Vec<_>>();
        let mut empty_item = String::new();
        render_list_item(*list_items[0], &mut empty_item, false);
        assert_eq!(empty_item.trim(), "-");

        let mut inline_segment = "  Text  ".to_owned();
        let mut body_segments = Vec::new();
        flush_list_item_inline_segment(&mut inline_segment, &mut body_segments);
        assert_eq!(body_segments, vec!["Text"]);
        flush_list_item_inline_segment(&mut inline_segment, &mut body_segments);
        assert_eq!(body_segments, vec!["Text"]);
        let direct_text_document = parse_document_node("<section>Lead<div>Value</div></section>");
        let direct_text_section = select_first(&direct_text_document, "section").expect("section");
        assert!(!direct_text_is_whitespace_only(*direct_text_section));
        let whitespace_only_document =
            parse_document_node("<section>\n  <div>Value</div>\n</section>");
        let whitespace_only_section =
            select_first(&whitespace_only_document, "section").expect("section");
        assert!(direct_text_is_whitespace_only(*whitespace_only_section));
        let ordered_document = parse_document_node(
            "<ol reversed start=\"5\"><li>One</li><li value=\"10\">Two</li><li>Three</li></ol>",
        );
        let ordered_items = ordered_document
            .select(&scraper::Selector::parse("li").expect("li selector"))
            .collect::<Vec<_>>();
        assert_eq!(list_item_marker(*ordered_items[0]), "5. ");
        assert_eq!(list_item_marker(*ordered_items[1]), "10. ");
        assert_eq!(list_item_marker(*ordered_items[2]), "9. ");
        assert!(is_list_item_block_segment(
            *select_first(
                &parse_document_node("<blockquote><p>Quote</p></blockquote>"),
                "blockquote",
            )
            .expect("blockquote")
        ));
        assert!(!is_list_item_block_segment(
            *select_first(&parse_document_node("<span>Inline</span>"), "span").expect("span")
        ));

        assert_eq!(
            collapse_inline_whitespace("  Hello   world "),
            "Hello world"
        );
        assert!(!needs_space("", "world"));
        assert!(!needs_space("Hello", ""));
        assert!(!needs_space("(", "world"));
        assert!(!needs_space("Hello", "."));
        assert!(needs_space("Hello", "world"));

        let mut output = "Hello".to_owned();
        push_newline(&mut output, 2);
        assert_eq!(output, "Hello\n\n");
        assert_eq!(
            apply_whitespace_mode("Alpha\n\n\nBeta\n", WhitespaceMode::Normalize),
            "Alpha\n\nBeta"
        );
        assert_eq!(
            apply_whitespace_mode("Alpha\n\nBeta\n", WhitespaceMode::Preserve),
            "Alpha\n\nBeta"
        );
        assert_eq!(normalize_structured_line("   "), "");
        assert_eq!(
            remove_immediate_heading_echoes("# Heading\n\nHeading\n\n\nBody"),
            "# Heading\n\n\nBody"
        );
        assert_eq!(
            remove_immediate_heading_echoes("# Heading\n\nHeading\nBody"),
            "# Heading\n\nBody"
        );
    }
}
