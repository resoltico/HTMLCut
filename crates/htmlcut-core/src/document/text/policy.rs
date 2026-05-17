use ego_tree::NodeRef as DomNodeRef;
use scraper::{ElementRef, Node};

use super::render::{collapse_inline_whitespace, extract_heading_text, needs_space};
use super::signals::{
    element_looks_like_utility_chrome, structural_signal_tokens, token_match_count,
};
use super::vocabulary::{
    AUXILIARY_SECTION_HEADINGS, NOTE_FRAGMENT_PREFIXES, READER_AUXILIARY_TOKENS,
    READER_CONTENT_HINT_TOKENS, READER_NOTICE_PHRASES, READER_NOTICE_STRONG_PHRASES,
    READER_NOTICE_TOKENS, SOURCE_ATTRIBUTION_PREFIXES,
};
use crate::document::heading_level;

use super::render::TextRenderIntent;

pub(super) fn element_has_hidden_style(element: &ElementRef<'_>) -> bool {
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

pub(super) fn element_should_skip_in_reader_text(element: &ElementRef<'_>) -> bool {
    if element_has_hidden_style(element) {
        return true;
    }
    if element_is_explicitly_hidden(element) {
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

pub(super) fn should_skip_rendered_element(
    element: &ElementRef<'_>,
    intent: TextRenderIntent,
    selected_root: bool,
) -> bool {
    if element_has_hidden_style(element) || element_is_explicitly_hidden(element) {
        return true;
    }

    if selected_root && matches!(intent, TextRenderIntent::SelectedFragment) {
        return false;
    }

    element_looks_like_reader_auxiliary(element)
        || element_looks_like_brief_reader_notice(element)
        || element_looks_like_source_attribution(element)
        || element_looks_like_auxiliary_section(element)
        || element_looks_like_utility_chrome(element)
}

pub(super) fn node_starts_terminal_non_narrative_section(node: DomNodeRef<'_, Node>) -> bool {
    let Some(element) = ElementRef::wrap(node) else {
        return false;
    };

    element_starts_terminal_utility_section(&element)
        || leading_section_heading(&element)
            .map(|heading| normalize_auxiliary_heading(&heading))
            .is_some_and(|heading| AUXILIARY_SECTION_HEADINGS.contains(&heading.as_str()))
}

fn element_is_explicitly_hidden(element: &ElementRef<'_>) -> bool {
    element.value().attr("hidden").is_some()
        || element
            .value()
            .attr("aria-hidden")
            .is_some_and(|value| value.eq_ignore_ascii_case("true"))
}

pub(super) fn element_looks_like_reader_auxiliary(element: &ElementRef<'_>) -> bool {
    let tag_name = element.value().name();
    let tokens = structural_signal_tokens(element);
    let auxiliary_count = token_match_count(&tokens, &READER_AUXILIARY_TOKENS);
    let content_count = token_match_count(&tokens, &READER_CONTENT_HINT_TOKENS);

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

pub(super) fn element_looks_like_brief_reader_notice(element: &ElementRef<'_>) -> bool {
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

pub(super) fn element_looks_like_source_attribution(element: &ElementRef<'_>) -> bool {
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

pub(super) fn element_looks_like_auxiliary_section(element: &ElementRef<'_>) -> bool {
    if !matches!(element.value().name(), "aside" | "div" | "nav" | "section") {
        return false;
    }

    leading_section_heading(element)
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

pub(super) fn looks_like_note_fragment_anchor(element: &ElementRef<'_>) -> bool {
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

pub(super) fn collect_notice_text(element: DomNodeRef<'_, Node>, limit: usize) -> String {
    let mut rendered = String::new();
    collect_notice_node_text(element, limit, &mut rendered);
    collapse_inline_whitespace(rendered.trim())
}

pub(super) fn collect_notice_node_text(
    node: DomNodeRef<'_, Node>,
    limit: usize,
    output: &mut String,
) {
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

pub(super) fn tokenize_notice_text(input: &str) -> Vec<String> {
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

pub(super) fn is_note_fragment_href(href: &str) -> bool {
    let href = href.trim().to_ascii_lowercase();
    NOTE_FRAGMENT_PREFIXES
        .iter()
        .any(|prefix| href.starts_with(prefix))
}
