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
const READER_AUXILIARY_TOKENS: [&str; 20] = [
    "backlink",
    "bibliography",
    "citation",
    "citations",
    "cite",
    "disambiguation",
    "editsection",
    "footnote",
    "footnotes",
    "hatnote",
    "noteref",
    "reflist",
    "reference",
    "references",
    "redirect",
    "shortdescription",
    "subjectbar",
    "subjectpageheader",
    "sister",
    "toc",
];
const CONTENT_HINT_TOKENS: [&str; 6] = ["article", "body", "content", "main", "story", "text"];
const READER_NOTICE_TOKENS: [&str; 14] = [
    "advertising",
    "affiliate",
    "commission",
    "copyright",
    "cookie",
    "cookies",
    "disclosure",
    "earn",
    "links",
    "policy",
    "privacy",
    "purchase",
    "sponsored",
    "terms",
];
const READER_NOTICE_PHRASES: [&str; 8] = [
    "affiliate commission",
    "fair use",
    "here's how it works",
    "how it works",
    "terms apply",
    "we may earn",
    "when you purchase through links",
    "when you purchase through links on our site",
];
const READER_NOTICE_STRONG_PHRASES: [&str; 4] = [
    "copyright act",
    "reviewed by an editor",
    "this article was generated",
    "used for the purpose of news reporting",
];
const AUXILIARY_SECTION_HEADINGS: [&str; 7] = [
    "citations",
    "external links",
    "further reading",
    "notes",
    "other sources",
    "references",
    "see also",
];
const SOURCE_ATTRIBUTION_PREFIXES: [&str; 3] = ["source:", "sources:", "via:"];
const NOTE_FRAGMENT_PREFIXES: [&str; 8] = [
    "#bib",
    "#cite",
    "#fn",
    "#footnote",
    "#note",
    "#ref",
    "#refs",
    "#r",
];

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
    render_child_nodes(children, &mut output, false, false, true);

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
                render_child_nodes(node.children(), output, false, false, false);
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
                render_child_nodes(node.children(), output, false, false, false);
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
                render_child_nodes(node.children(), output, false, false, false);
                push_newline(output, 1);
                return;
            }

            if tag_name == "dd" {
                push_newline(output, 1);
                output.push_str(": ");
                render_child_nodes(node.children(), output, false, true, false);
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
            render_child_nodes(node.children(), output, child_in_pre, false, true);

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

fn push_inline_text(output: &mut String, text: &str) {
    if text.is_empty() {
        return;
    }
    if needs_space(output, text) {
        output.push(' ');
    }
    output.push_str(text);
}

fn hidden_math_replacement(element: &ElementRef<'_>) -> Option<String> {
    if !element_has_hidden_style(element) {
        return None;
    }

    element
        .descendants()
        .filter_map(ElementRef::wrap)
        .find(|descendant| descendant.value().name() == "math")
        .and_then(|math| render_math_element(&math))
}

fn element_has_hidden_style(element: &ElementRef<'_>) -> bool {
    let Some(style) = element.value().attr("style") else {
        return false;
    };

    style.split(';').any(|declaration| {
        let Some((property, value)) = declaration.split_once(':') else {
            return false;
        };
        let property = property.trim();
        let value = value.trim().to_ascii_lowercase();
        matches!(property, "display" | "visibility")
            && (value.contains("none") || value.contains("hidden"))
    })
}

fn element_looks_like_reader_auxiliary(element: &ElementRef<'_>) -> bool {
    let tag_name = element.value().name();
    let tokens = structural_signal_tokens(element);
    let auxiliary_count = token_match_count(&tokens, &READER_AUXILIARY_TOKENS);
    let content_count = token_match_count(&tokens, &CONTENT_HINT_TOKENS);

    if auxiliary_count == 0 && !looks_like_note_fragment_anchor(element) {
        return false;
    }

    if matches!(tag_name, "a" | "sup" | "sub" | "span") && looks_like_note_fragment_anchor(element)
    {
        return true;
    }

    if matches!(tag_name, "span" | "div") && token_match_count(&tokens, &["cite", "backlink"]) > 0 {
        return true;
    }

    auxiliary_count > content_count
        && matches!(
            tag_name,
            "a" | "aside"
                | "div"
                | "li"
                | "nav"
                | "ol"
                | "p"
                | "section"
                | "span"
                | "sub"
                | "sup"
                | "ul"
        )
}

fn element_looks_like_brief_reader_notice(element: &ElementRef<'_>) -> bool {
    if !matches!(
        element.value().name(),
        "aside" | "div" | "li" | "p" | "section" | "span"
    ) {
        return false;
    }

    let text = collect_notice_text(**element, 420);
    if text.is_empty() {
        return false;
    }

    let normalized = collapse_inline_whitespace(text.trim()).to_ascii_lowercase();
    let strong_phrase_hit = READER_NOTICE_STRONG_PHRASES
        .iter()
        .any(|phrase| normalized.contains(phrase));
    let character_count = normalized.chars().count();
    if character_count > 420 {
        return false;
    }
    if character_count > 240 && !strong_phrase_hit {
        return false;
    }

    if normalized
        .chars()
        .filter(|character| character.is_alphabetic())
        .count()
        < 18
    {
        return false;
    }

    let token_hits = tokenize_notice_text(&normalized)
        .into_iter()
        .filter(|token| READER_NOTICE_TOKENS.contains(&token.as_str()))
        .count();
    let phrase_hit = READER_NOTICE_PHRASES
        .iter()
        .any(|phrase| normalized.contains(phrase));
    let has_link = element
        .descendants()
        .filter_map(ElementRef::wrap)
        .any(|descendant| descendant.value().name() == "a");

    if !has_link {
        return false;
    }

    strong_phrase_hit || phrase_hit || token_hits >= 3
}

fn element_should_skip_in_reader_text(element: &ElementRef<'_>) -> bool {
    if element_has_hidden_style(element) {
        return true;
    }
    if element_looks_like_reader_auxiliary(element) {
        return true;
    }
    if element_looks_like_brief_reader_notice(element) {
        return true;
    }
    if element_looks_like_source_attribution(element) {
        return true;
    }
    if element_looks_like_auxiliary_section(element) {
        return true;
    }
    false
}

fn element_looks_like_source_attribution(element: &ElementRef<'_>) -> bool {
    if !matches!(
        element.value().name(),
        "div" | "li" | "p" | "section" | "span"
    ) {
        return false;
    }

    let text = collect_notice_text(**element, 220);
    if text.is_empty() {
        return false;
    }

    let normalized = collapse_inline_whitespace(text.trim()).to_ascii_lowercase();
    let link_count = element
        .descendants()
        .filter_map(ElementRef::wrap)
        .filter(|descendant| descendant.value().name() == "a")
        .count();
    link_count > 0
        && normalized.chars().count() <= 220
        && SOURCE_ATTRIBUTION_PREFIXES
            .iter()
            .any(|prefix| normalized.starts_with(prefix))
}

fn element_looks_like_auxiliary_section(element: &ElementRef<'_>) -> bool {
    if !matches!(element.value().name(), "aside" | "div" | "nav" | "section") {
        return false;
    }

    leading_section_heading(element)
        .map(|heading| normalize_auxiliary_heading(&heading))
        .is_some_and(|heading| AUXILIARY_SECTION_HEADINGS.contains(&heading.as_str()))
}

fn render_child_nodes<'a>(
    children: impl Iterator<Item = DomNodeRef<'a, Node>>,
    output: &mut String,
    in_pre: bool,
    list_item: bool,
    stop_at_terminal_auxiliary: bool,
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
        render_node(child, output, in_pre, list_item);
        if output.len() > before_len {
            rendered_substantive = true;
        }
    }
}

fn node_starts_terminal_non_narrative_section(node: DomNodeRef<'_, Node>) -> bool {
    let Some(element) = ElementRef::wrap(node) else {
        return false;
    };

    element_starts_terminal_utility_section(&element)
        || leading_section_heading(&element)
            .map(|heading| normalize_auxiliary_heading(&heading))
            .is_some_and(|heading| AUXILIARY_SECTION_HEADINGS.contains(&heading.as_str()))
}

fn element_starts_terminal_utility_section(element: &ElementRef<'_>) -> bool {
    matches!(element.value().name(), "aside" | "nav" | "section")
        && element_looks_like_utility_chrome(element)
}

fn leading_section_heading(element: &ElementRef<'_>) -> Option<String> {
    if let Some(heading) = heading_text_if_heading(element) {
        return Some(heading);
    }

    for child in element.children().filter_map(ElementRef::wrap).take(4) {
        if let Some(heading) = heading_text_if_heading(&child) {
            return Some(heading);
        }

        if !matches!(child.value().name(), "div" | "header" | "section") {
            continue;
        }

        for grandchild in child.children().filter_map(ElementRef::wrap).take(4) {
            if let Some(heading) = heading_text_if_heading(&grandchild) {
                return Some(heading);
            }
        }
    }

    None
}

fn heading_text_if_heading(element: &ElementRef<'_>) -> Option<String> {
    heading_level(element.value().name())
        .and_then(|_| extract_heading_text(element))
        .filter(|heading| !heading.trim().is_empty())
}

fn looks_like_note_fragment_anchor(element: &ElementRef<'_>) -> bool {
    if element
        .value()
        .attr("href")
        .is_some_and(is_note_fragment_href)
    {
        return true;
    }

    element
        .descendants()
        .filter_map(ElementRef::wrap)
        .filter(|descendant| descendant.id() != element.id())
        .any(|descendant| {
            descendant
                .value()
                .attr("href")
                .is_some_and(is_note_fragment_href)
        })
}

fn collect_notice_text(element: DomNodeRef<'_, Node>, limit: usize) -> String {
    let mut rendered = String::new();
    collect_notice_node_text(element, limit, &mut rendered);
    collapse_inline_whitespace(rendered.trim())
}

fn collect_notice_node_text(node: DomNodeRef<'_, Node>, limit: usize, output: &mut String) {
    if output.chars().count() >= limit {
        return;
    }

    match node.value() {
        Node::Text(contents) => {
            let text = collapse_inline_whitespace(contents);
            if text.is_empty() {
                return;
            }
            if needs_space(output, &text) {
                output.push(' ');
            }
            output.push_str(&text);
        }
        Node::Element(data) => {
            if matches!(
                data.name(),
                "head" | "noscript" | "script" | "style" | "template"
            ) {
                return;
            }

            for child in node.children() {
                collect_notice_node_text(child, limit, output);
                if output.chars().count() >= limit {
                    return;
                }
            }
        }
        _ => {
            for child in node.children() {
                collect_notice_node_text(child, limit, output);
                if output.chars().count() >= limit {
                    return;
                }
            }
        }
    }
}

fn tokenize_notice_text(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();

    for character in input.chars() {
        if character.is_ascii_alphanumeric() {
            current.push(character.to_ascii_lowercase());
        } else if !current.is_empty() {
            tokens.push(std::mem::take(&mut current));
        }
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

fn normalize_auxiliary_heading(input: &str) -> String {
    collapse_inline_whitespace(input.trim())
        .trim_end_matches(':')
        .to_ascii_lowercase()
}

fn is_note_fragment_href(href: &str) -> bool {
    let href = href.trim().to_ascii_lowercase();
    NOTE_FRAGMENT_PREFIXES
        .iter()
        .any(|prefix| href.starts_with(prefix))
}

fn render_math_element(element: &ElementRef<'_>) -> Option<String> {
    let mut rendered = String::new();
    render_math_node(**element, &mut rendered);
    let normalized = collapse_inline_whitespace(rendered.trim());
    if !normalized.is_empty() {
        return Some(normalized);
    }

    element
        .value()
        .attr("alttext")
        .map(collapse_inline_whitespace)
        .filter(|alt| !alt.is_empty())
}

fn render_math_node(node: DomNodeRef<'_, Node>, output: &mut String) {
    match node.value() {
        Node::Text(contents) => {
            let text = collapse_inline_whitespace(contents);
            if text.is_empty() {
                return;
            }
            push_inline_text(output, &text);
        }
        Node::Element(data) => {
            let tag_name = data.name();
            if matches!(tag_name, "annotation" | "annotation-xml") {
                return;
            }

            match tag_name {
                "mfrac" => {
                    if let Some(rendered) = render_math_fraction(node) {
                        push_inline_text(output, &rendered);
                        return;
                    }
                }
                "msub" => {
                    if let Some(rendered) = render_math_binary_operator(node, "_") {
                        push_inline_text(output, &rendered);
                        return;
                    }
                }
                "msup" => {
                    if let Some(rendered) = render_math_binary_operator(node, "^") {
                        push_inline_text(output, &rendered);
                        return;
                    }
                }
                "msubsup" => {
                    if let Some(rendered) = render_math_subsup(node) {
                        push_inline_text(output, &rendered);
                        return;
                    }
                }
                "msqrt" => {
                    if let Some(rendered) = render_math_wrapped(node, "sqrt(", ")") {
                        push_inline_text(output, &rendered);
                        return;
                    }
                }
                "mroot" => {
                    if let Some(rendered) = render_math_root(node) {
                        push_inline_text(output, &rendered);
                        return;
                    }
                }
                _ => {}
            }

            for child in node.children() {
                render_math_node(child, output);
            }
        }
        _ => {
            for child in node.children() {
                render_math_node(child, output);
            }
        }
    }
}

fn render_math_fraction(node: DomNodeRef<'_, Node>) -> Option<String> {
    let children = direct_child_elements(node);
    if children.len() < 2 {
        return None;
    }

    let numerator = render_math_node_to_string(*children[0]);
    let denominator = render_math_node_to_string(*children[1]);
    if numerator.is_empty() || denominator.is_empty() {
        return None;
    }

    Some(format!(
        "{}/{}",
        wrap_math_operand(&numerator),
        wrap_math_operand(&denominator)
    ))
}

fn render_math_binary_operator(node: DomNodeRef<'_, Node>, operator: &str) -> Option<String> {
    let children = direct_child_elements(node);
    if children.len() < 2 {
        return None;
    }

    let left = render_math_node_to_string(*children[0]);
    let right = render_math_node_to_string(*children[1]);
    if left.is_empty() || right.is_empty() {
        return None;
    }

    Some(format!("{left}{operator}{}", wrap_math_operand(&right)))
}

fn render_math_subsup(node: DomNodeRef<'_, Node>) -> Option<String> {
    let children = direct_child_elements(node);
    if children.len() < 3 {
        return None;
    }

    let base = render_math_node_to_string(*children[0]);
    let sub = render_math_node_to_string(*children[1]);
    let sup = render_math_node_to_string(*children[2]);
    if base.is_empty() || sub.is_empty() || sup.is_empty() {
        return None;
    }

    Some(format!(
        "{base}_{}^{}",
        wrap_math_operand(&sub),
        wrap_math_operand(&sup)
    ))
}

fn render_math_wrapped(node: DomNodeRef<'_, Node>, prefix: &str, suffix: &str) -> Option<String> {
    let rendered = render_math_children_to_string(node);
    if rendered.is_empty() {
        return None;
    }

    Some(format!("{prefix}{rendered}{suffix}"))
}

fn render_math_root(node: DomNodeRef<'_, Node>) -> Option<String> {
    let children = direct_child_elements(node);
    if children.len() < 2 {
        return None;
    }

    let value = render_math_node_to_string(*children[0]);
    let degree = render_math_node_to_string(*children[1]);
    if value.is_empty() || degree.is_empty() {
        return None;
    }

    Some(format!("root({value}, {degree})"))
}

fn render_math_children_to_string(node: DomNodeRef<'_, Node>) -> String {
    let mut rendered = String::new();
    for child in node.children() {
        render_math_node(child, &mut rendered);
    }
    collapse_inline_whitespace(rendered.trim())
}

fn render_math_node_to_string(node: DomNodeRef<'_, Node>) -> String {
    let mut rendered = String::new();
    render_math_node(node, &mut rendered);
    collapse_inline_whitespace(rendered.trim())
}

fn wrap_math_operand(operand: &str) -> String {
    if operand.chars().any(|character| character.is_whitespace())
        || operand.contains('/')
        || operand.contains('^')
        || operand.contains('_')
    {
        format!("({operand})")
    } else {
        operand.to_owned()
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
        WhitespaceMode::Rendered => input.trim_matches('\n').to_owned(),
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
        {
            if lines.get(index + 1) == Some(&"")
                && lines
                    .get(index + 2)
                    .is_some_and(|line| line.trim() == heading_text)
            {
                index += 3;
                index += usize::from(lines.get(index) == Some(&""));
                output.push(String::new());
                continue;
            }

            let mut duplicate_index = index + 1;
            while lines.get(duplicate_index) == Some(&"") {
                duplicate_index += 1;
            }
            if lines
                .get(duplicate_index)
                .is_some_and(|line| line.trim() == current.trim())
            {
                index = duplicate_index + 1;
                index += usize::from(lines.get(index) == Some(&""));
                output.push(String::new());
                continue;
            }
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
                WhitespaceMode::Rendered,
            ),
            "Body"
        );
        assert_eq!(
            render_html_as_text("<p>Alpha<br>Beta</p>", WhitespaceMode::Rendered),
            "Alpha\nBeta"
        );
        assert_eq!(
            render_html_as_text("<p>Alpha</p><hr><p>Beta</p>", WhitespaceMode::Rendered),
            "Alpha\n\n---\n\nBeta"
        );
        assert_eq!(
            render_html_as_text(
                "<p><a href=\"/empty\"><img alt=\"\" src=\"hero.png\"></a>After</p>",
                WhitespaceMode::Rendered,
            ),
            "After"
        );
        assert_eq!(
            render_html_as_text("<h2>   </h2><p>Body</p>", WhitespaceMode::Rendered),
            "Body"
        );
        assert_eq!(
            render_html_as_text(
                "<dl><dt>Term</dt><dd>Definition</dd></dl>",
                WhitespaceMode::Rendered,
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
                WhitespaceMode::Rendered,
            ),
            "Body."
        );
        assert_eq!(
            render_html_as_text(
                "<article><table>\n<tr><td>Alpha</td></tr>\n</table></article>",
                WhitespaceMode::Rendered,
            ),
            "Alpha"
        );
        assert_eq!(
            render_html_as_text(
                "<article><table><caption>Windows builds</caption><tr><td>Alpha</td></tr></table></article>",
                WhitespaceMode::Rendered,
            ),
            "Windows builds\nAlpha"
        );
        assert_eq!(
            render_html_as_text(
                "<article><table><caption>Caption only</caption></table></article>",
                WhitespaceMode::Rendered,
            ),
            "Caption only"
        );
        assert_eq!(
            render_html_as_text(
                "<article><figure><img alt=\"Hero\" src=\"hero.jpg\"><figcaption>Caption</figcaption></figure><div class=\"caption-box\"><img alt=\"Hero Two\" src=\"hero2.jpg\"><div class=\"caption\">Caption</div></div></article>",
                WhitespaceMode::Rendered,
            ),
            ""
        );
        assert_eq!(
            render_html_as_text(
                "<article><div><img alt=\"Hero\" src=\"hero.jpg\"><figcaption>Caption</figcaption></div></article>",
                WhitespaceMode::Rendered,
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
        assert!(empty_item.trim().is_empty());

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
            apply_whitespace_mode("Alpha\n\nBeta\n", WhitespaceMode::Rendered),
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

    #[test]
    fn reader_cleanup_and_math_helpers_cover_hidden_auxiliary_and_math_edges() {
        assert_eq!(
            render_html_as_text(
                "<article><p>Alpha <math><msup><mi>x</mi><mn>2</mn></msup></math> Beta</p></article>",
                WhitespaceMode::Rendered,
            ),
            "Alpha x^2 Beta"
        );
        assert_eq!(
            render_html_as_text(
                "<article><span style=\"display:none\"><math><mi>x</mi></math></span><span style=\"visibility:hidden\">Hidden</span><a href=\"#cite_note-1\">[1]</a><span class=\"backlink\">Back</span><p>Body</p></article>",
                WhitespaceMode::Rendered,
            ),
            "x\n\nBody"
        );

        let heading_math = parse_document_node(
            "<h2><math><mfrac><mi>a</mi><mi>b</mi></mfrac></math><span class=\"reference\">[1]</span><img alt=\"Hero\"></h2>",
        );
        let heading = select_first(&heading_math, "h2").expect("heading");
        assert_eq!(extract_heading_text(&heading).as_deref(), Some("a/b Hero"));
        let mut heading_math_rendered = String::new();
        render_heading_text_node(
            *select_first(&heading_math, "math").expect("math"),
            &mut heading_math_rendered,
            false,
        );
        assert_eq!(heading_math_rendered, "a/b");
        let hidden_heading_math = parse_document_node(
            "<h2><span style=\"display:none\"><math><mi>x</mi></math></span></h2>",
        );
        let mut hidden_heading_rendered = String::new();
        render_heading_text_node(
            *select_first(&hidden_heading_math, "span").expect("span"),
            &mut hidden_heading_rendered,
            false,
        );
        assert_eq!(hidden_heading_rendered, "x");

        let fallback_math_document = parse_document_node(
            "<math alttext=\"x squared\"><annotation>ignored</annotation></math>",
        );
        let fallback_math = select_first(&fallback_math_document, "math").expect("math");
        assert_eq!(
            render_math_element(&fallback_math).as_deref(),
            Some("x squared")
        );

        let hidden_with_math =
            parse_document_node("<span style=\"display:none\"><math><mi>z</mi></math></span>");
        let hidden_math = select_first(&hidden_with_math, "span").expect("hidden math");
        assert_eq!(hidden_math_replacement(&hidden_math).as_deref(), Some("z"));

        let hidden_style_false = parse_document_node("<span style=\"display\">Body</span>");
        let hidden_style_false_element = select_first(&hidden_style_false, "span").expect("span");
        assert!(!element_has_hidden_style(&hidden_style_false_element));

        let note_anchor = parse_document_node("<sup><a href=\"#cite_note-1\">[1]</a></sup>");
        let note_anchor_element = select_first(&note_anchor, "sup").expect("sup");
        assert!(element_looks_like_reader_auxiliary(&note_anchor_element));
        assert!(looks_like_note_fragment_anchor(&note_anchor_element));
        assert!(is_note_fragment_href("#CITE_NOTE-1"));

        let backlink = parse_document_node("<span class=\"backlink\">Back</span>");
        let backlink_element = select_first(&backlink, "span").expect("span");
        assert!(element_looks_like_reader_auxiliary(&backlink_element));
        let reference_list = parse_document_node("<ul class=\"references\"><li>Ref</li></ul>");
        let reference_list_element = select_first(&reference_list, "ul").expect("ul");
        assert!(element_looks_like_reader_auxiliary(&reference_list_element));
        let hatnote = parse_document_node(
            "<div class=\"hatnote navigation-not-searchable\">For other uses, see <a href=\"/wiki/Math_(disambiguation)\">Math (disambiguation)</a>.</div>",
        );
        let hatnote_element = select_first(&hatnote, "div").expect("hatnote");
        assert!(element_looks_like_reader_auxiliary(&hatnote_element));
        let subjectpageheader = parse_document_node(
            "<div class=\"mw-subjectpageheader\"><span>From Wikipedia, the free encyclopedia</span></div>",
        );
        let subjectpageheader_element =
            select_first(&subjectpageheader, "div").expect("subjectpageheader");
        assert!(element_looks_like_reader_auxiliary(
            &subjectpageheader_element
        ));
        let affiliate_notice = parse_document_node(
            "<span>When you purchase through links on our site, we may earn an affiliate commission. <a href=\"/terms\">Here’s how it works</a>.</span>",
        );
        let affiliate_notice_element =
            select_first(&affiliate_notice, "span").expect("affiliate notice");
        assert!(element_looks_like_brief_reader_notice(
            &affiliate_notice_element
        ));
        let ordinary_short_text = parse_document_node(
            "<span><a href=\"/guide\">Guide</a> to the latest experiment results.</span>",
        );
        let ordinary_short_text_element =
            select_first(&ordinary_short_text, "span").expect("ordinary short text");
        assert!(!element_looks_like_brief_reader_notice(
            &ordinary_short_text_element
        ));
        assert_eq!(
            collect_notice_text(*affiliate_notice_element, 240),
            "When you purchase through links on our site, we may earn an affiliate commission. Here’s how it works."
        );
        let long_strong_notice = parse_document_node(
            "<span>\
                This article was generated for demonstration purposes. \
                This article was generated for demonstration purposes. \
                This article was generated for demonstration purposes. \
                This article was generated for demonstration purposes. \
                This article was generated for demonstration purposes. \
                <a href=\"/terms\">Read more</a>.\
            </span>",
        );
        assert!(element_looks_like_brief_reader_notice(
            &select_first(&long_strong_notice, "span").expect("strong notice")
        ));
        let source_attribution = parse_document_node(
            "<p>Source: <a href=\"https://example.test/feed\">Research Feed</a></p>",
        );
        let source_attribution_element =
            select_first(&source_attribution, "p").expect("source attribution");
        assert!(element_looks_like_source_attribution(
            &source_attribution_element
        ));
        assert!(element_should_skip_in_reader_text(
            &source_attribution_element
        ));
        let auxiliary_section = parse_document_node(
            "<section><h2>References</h2><ul><li><a href=\"/source\">Source</a></li></ul></section>",
        );
        let auxiliary_section_element =
            select_first(&auxiliary_section, "section").expect("auxiliary section");
        assert!(element_looks_like_auxiliary_section(
            &auxiliary_section_element
        ));
        assert!(element_should_skip_in_reader_text(
            &auxiliary_section_element
        ));
        assert_eq!(
            tokenize_notice_text("Terms apply; we may earn affiliate commission!"),
            vec![
                "terms",
                "apply",
                "we",
                "may",
                "earn",
                "affiliate",
                "commission"
            ]
        );
        let nested_notice_document = parse_document_node(
            "<div><!--ignored--><script>skip</script><style>.x{}</style><template><span>Hidden</span></template><noscript>Fallback</noscript><span>Alpha</span><span>Beta</span></div>",
        );
        let nested_notice_element = select_first(&nested_notice_document, "div").expect("notice");
        assert_eq!(
            collect_notice_text(*nested_notice_element, 240),
            "Alpha Beta"
        );
        let whitespace_notice_document = parse_document_node("<div>   </div>");
        let whitespace_notice = select_first(&whitespace_notice_document, "div").expect("notice");
        let whitespace_text = whitespace_notice
            .children()
            .next()
            .expect("whitespace text child");
        let mut whitespace_output = String::new();
        collect_notice_node_text(whitespace_text, 240, &mut whitespace_output);
        assert!(whitespace_output.is_empty());
        let mut capped_output = "already enough".to_owned();
        collect_notice_node_text(*nested_notice_element, 5, &mut capped_output);
        assert_eq!(capped_output, "already enough");
        let mut document_root_output = String::new();
        collect_notice_node_text(
            nested_notice_document.tree.root(),
            240,
            &mut document_root_output,
        );
        assert!(document_root_output.contains("Alpha Beta"));
        let root_limited_document = parse_document_node("<div>Alpha</div><div>Beta</div>");
        let mut root_limited_output = String::new();
        collect_notice_node_text(
            root_limited_document.tree.root(),
            5,
            &mut root_limited_output,
        );
        assert_eq!(root_limited_output, "Alpha");

        let direct_math_root =
            parse_document_node("<math><msup><mi>x</mi><mn>2</mn></msup></math>");
        let mut direct_math_root_rendered = String::new();
        render_node(
            *select_first(&direct_math_root, "math").expect("math"),
            &mut direct_math_root_rendered,
            false,
            false,
        );
        assert_eq!(direct_math_root_rendered, "x^2");

        let direct_math = parse_document_node(
            "<math><msub><mi>x</mi><mi>i</mi></msub><msubsup><mi>y</mi><mi>i</mi><mn>2</mn></msubsup><msqrt><mi>z</mi></msqrt><mroot><mi>x</mi><mn>3</mn></mroot></math>",
        );
        let direct_math_element = select_first(&direct_math, "math").expect("math");
        assert_eq!(
            render_math_element(&direct_math_element).as_deref(),
            Some("x_i y_i^2 sqrt(z) root(x, 3)")
        );

        let incomplete_fraction = parse_document_node("<mfrac><mi>a</mi></mfrac>");
        assert_eq!(
            render_math_fraction(*select_first(&incomplete_fraction, "mfrac").expect("mfrac")),
            None
        );
        let incomplete_sub = parse_document_node("<msub><mi>a</mi></msub>");
        assert_eq!(
            render_math_binary_operator(*select_first(&incomplete_sub, "msub").expect("msub"), "_"),
            None
        );
        let incomplete_subsup = parse_document_node("<msubsup><mi>a</mi><mi>b</mi></msubsup>");
        assert_eq!(
            render_math_subsup(*select_first(&incomplete_subsup, "msubsup").expect("msubsup")),
            None
        );
        let empty_wrapped = parse_document_node("<msqrt><annotation>ignored</annotation></msqrt>");
        assert_eq!(
            render_math_wrapped(
                *select_first(&empty_wrapped, "msqrt").expect("msqrt"),
                "sqrt(",
                ")"
            ),
            None
        );
        let incomplete_root = parse_document_node("<mroot><mi>a</mi></mroot>");
        assert_eq!(
            render_math_root(*select_first(&incomplete_root, "mroot").expect("mroot")),
            None
        );
        let whitespace_math = parse_document_node("<math>   </math>");
        assert_eq!(
            render_math_children_to_string(*select_first(&whitespace_math, "math").expect("math")),
            ""
        );
        let numerator_empty =
            parse_document_node("<mfrac><annotation>ignored</annotation><mi>b</mi></mfrac>");
        assert_eq!(
            render_math_fraction(*select_first(&numerator_empty, "mfrac").expect("mfrac")),
            None
        );
        let left_empty =
            parse_document_node("<msub><annotation>ignored</annotation><mi>b</mi></msub>");
        assert_eq!(
            render_math_binary_operator(*select_first(&left_empty, "msub").expect("msub"), "_"),
            None
        );
        let sub_empty = parse_document_node(
            "<msubsup><mi>a</mi><annotation>ignored</annotation><mi>c</mi></msubsup>",
        );
        assert_eq!(
            render_math_subsup(*select_first(&sub_empty, "msubsup").expect("msubsup")),
            None
        );
        let root_value_empty =
            parse_document_node("<mroot><annotation>ignored</annotation><mn>3</mn></mroot>");
        assert_eq!(
            render_math_root(*select_first(&root_value_empty, "mroot").expect("mroot")),
            None
        );
        let rendered_fraction = parse_document_node("<mfrac><mi>a</mi><mi>b</mi></mfrac>");
        let mut fraction_output = String::new();
        render_math_node(
            *select_first(&rendered_fraction, "mfrac").expect("mfrac"),
            &mut fraction_output,
        );
        assert_eq!(fraction_output, "a/b");
        let rendered_sub = parse_document_node("<msub><mi>x</mi><mi>i</mi></msub>");
        let mut sub_output = String::new();
        render_math_node(
            *select_first(&rendered_sub, "msub").expect("msub"),
            &mut sub_output,
        );
        assert_eq!(sub_output, "x_i");
        let rendered_sup = parse_document_node("<msup><mi>x</mi><mn>2</mn></msup>");
        let mut sup_output = String::new();
        render_math_node(
            *select_first(&rendered_sup, "msup").expect("msup"),
            &mut sup_output,
        );
        assert_eq!(sup_output, "x^2");
        let rendered_subsup =
            parse_document_node("<msubsup><mi>y</mi><mi>i</mi><mn>2</mn></msubsup>");
        let mut subsup_output = String::new();
        render_math_node(
            *select_first(&rendered_subsup, "msubsup").expect("msubsup"),
            &mut subsup_output,
        );
        assert_eq!(subsup_output, "y_i^2");
        let rendered_sqrt = parse_document_node("<msqrt><mi>z</mi></msqrt>");
        let mut sqrt_output = String::new();
        render_math_node(
            *select_first(&rendered_sqrt, "msqrt").expect("msqrt"),
            &mut sqrt_output,
        );
        assert_eq!(sqrt_output, "sqrt(z)");
        let rendered_root = parse_document_node("<mroot><mi>x</mi><mn>3</mn></mroot>");
        let mut root_output = String::new();
        render_math_node(
            *select_first(&rendered_root, "mroot").expect("mroot"),
            &mut root_output,
        );
        assert_eq!(root_output, "root(x, 3)");
        let mut root_node_output = String::new();
        render_math_node(direct_math.tree.root(), &mut root_node_output);
        assert!(root_node_output.contains("x_i"));
        assert_eq!(wrap_math_operand("a/b"), "(a/b)");

        let mut inline = String::new();
        push_inline_text(&mut inline, "");
        assert!(inline.is_empty());
    }

    #[test]
    fn empty_list_items_do_not_emit_stray_bullet_markers() {
        assert_eq!(
            render_html_as_text(
                "<ul><li> </li><li><span>Visible item</span></li></ul>",
                WhitespaceMode::Rendered,
            ),
            "- Visible item"
        );
        let nested_only = render_html_as_text(
            "<ul><li><ul><li>Nested item</li></ul></li></ul>",
            WhitespaceMode::Rendered,
        );
        assert!(nested_only.trim_start().starts_with("- Nested item"));
        assert!(!nested_only.contains("\n- \n"));
        assert_eq!(
            render_html_as_text(
                "<ul><li><ul><li>Nested item</li></ul><ol><li>Second nested item</li></ol></li></ul>",
                WhitespaceMode::Rendered,
            ),
            "    - Nested item\n    1. Second nested item"
        );
    }

    #[test]
    fn immediate_duplicate_headings_are_collapsed_in_reader_text() {
        assert_eq!(
            render_html_as_text(
                "<section><h2>Why Apple is the best place to buy iPhone.</h2><h2>Why Apple is the best place to buy iPhone.</h2><p>Details.</p></section>",
                WhitespaceMode::Rendered,
            ),
            "## Why Apple is the best place to buy iPhone.\n\nDetails."
        );
    }

    #[test]
    fn math_fallback_paths_cover_unrenderable_nodes_and_operand_guards() {
        assert_eq!(
            render_html_as_text(
                "<article><math><annotation>ignored</annotation></math><p>Body</p></article>",
                WhitespaceMode::Rendered,
            ),
            "Body"
        );
        assert_eq!(
            render_html_as_text(
                "<h2><math><annotation>ignored</annotation></math>Heading</h2>",
                WhitespaceMode::Rendered,
            ),
            "## Heading"
        );

        let hidden_heading =
            parse_document_node("<h2><span style=\"display:none\">Hidden</span>Visible</h2>");
        let hidden_heading_element = select_first(&hidden_heading, "h2").expect("heading");
        assert_eq!(
            extract_heading_text(&hidden_heading_element).as_deref(),
            Some("Visible")
        );

        let malformed_fraction =
            parse_document_node("<mfrac><annotation>ignored</annotation><mi>b</mi></mfrac>");
        let mut malformed_fraction_output = String::new();
        render_math_node(
            *select_first(&malformed_fraction, "mfrac").expect("mfrac"),
            &mut malformed_fraction_output,
        );
        assert_eq!(malformed_fraction_output, "b");

        let malformed_sub =
            parse_document_node("<msub><mi>a</mi><annotation>ignored</annotation></msub>");
        let mut malformed_sub_output = String::new();
        render_math_node(
            *select_first(&malformed_sub, "msub").expect("msub"),
            &mut malformed_sub_output,
        );
        assert_eq!(malformed_sub_output, "a");

        let malformed_sup =
            parse_document_node("<msup><annotation>ignored</annotation><mn>2</mn></msup>");
        let mut malformed_sup_output = String::new();
        render_math_node(
            *select_first(&malformed_sup, "msup").expect("msup"),
            &mut malformed_sup_output,
        );
        assert_eq!(malformed_sup_output, "2");

        let malformed_subsup = parse_document_node(
            "<msubsup><annotation>ignored</annotation><mi>i</mi><mn>2</mn></msubsup>",
        );
        let mut malformed_subsup_output = String::new();
        render_math_node(
            *select_first(&malformed_subsup, "msubsup").expect("msubsup"),
            &mut malformed_subsup_output,
        );
        assert_eq!(malformed_subsup_output, "i 2");

        let malformed_sqrt = parse_document_node("<msqrt><annotation>ignored</annotation></msqrt>");
        let mut malformed_sqrt_output = String::new();
        render_math_node(
            *select_first(&malformed_sqrt, "msqrt").expect("msqrt"),
            &mut malformed_sqrt_output,
        );
        assert!(malformed_sqrt_output.is_empty());

        let malformed_root =
            parse_document_node("<mroot><mi>x</mi><annotation>ignored</annotation></mroot>");
        let mut malformed_root_output = String::new();
        render_math_node(
            *select_first(&malformed_root, "mroot").expect("mroot"),
            &mut malformed_root_output,
        );
        assert_eq!(malformed_root_output, "x");

        let denominator_empty =
            parse_document_node("<mfrac><mi>a</mi><annotation>ignored</annotation></mfrac>");
        assert_eq!(
            render_math_fraction(*select_first(&denominator_empty, "mfrac").expect("mfrac")),
            None
        );
        let right_empty =
            parse_document_node("<msub><mi>a</mi><annotation>ignored</annotation></msub>");
        assert_eq!(
            render_math_binary_operator(*select_first(&right_empty, "msub").expect("msub"), "_"),
            None
        );
        let base_empty = parse_document_node(
            "<msubsup><annotation>ignored</annotation><mi>b</mi><mi>c</mi></msubsup>",
        );
        assert_eq!(
            render_math_subsup(*select_first(&base_empty, "msubsup").expect("msubsup")),
            None
        );
        let sup_empty = parse_document_node(
            "<msubsup><mi>a</mi><mi>b</mi><annotation>ignored</annotation></msubsup>",
        );
        assert_eq!(
            render_math_subsup(*select_first(&sup_empty, "msubsup").expect("msubsup")),
            None
        );
        let root_degree_empty =
            parse_document_node("<mroot><mi>x</mi><annotation>ignored</annotation></mroot>");
        assert_eq!(
            render_math_root(*select_first(&root_degree_empty, "mroot").expect("mroot")),
            None
        );

        assert_eq!(wrap_math_operand("a b"), "(a b)");
        assert_eq!(wrap_math_operand("x^2"), "(x^2)");
        assert_eq!(wrap_math_operand("x_i"), "(x_i)");
    }
}
