use scraper::{ElementRef, Html};

use crate::contracts::WhitespaceMode;

use super::super::parse::{first_body, parse_wrapped_fragment};

pub(crate) const ELLIPSIS: &str = "...";
pub(super) const BLOCK_TAGS: [&str; 21] = [
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
pub(super) const SKIP_TAGS: [&str; 5] = ["head", "noscript", "script", "style", "template"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TextRenderIntent {
    ReaderDocument,
    SelectedFragment,
}

pub(crate) fn render_html_as_text(fragment: &str, whitespace: WhitespaceMode) -> String {
    let document = parse_wrapped_fragment(fragment);
    render_document_body_as_text(&document, whitespace)
}

pub(crate) fn render_document_body_as_text(document: &Html, whitespace: WhitespaceMode) -> String {
    render_document_body_as_text_with_intent(document, whitespace, TextRenderIntent::ReaderDocument)
}

pub(crate) fn render_selected_document_body_as_text(
    document: &Html,
    whitespace: WhitespaceMode,
) -> String {
    render_document_body_as_text_with_intent(
        document,
        whitespace,
        TextRenderIntent::SelectedFragment,
    )
}

fn render_document_body_as_text_with_intent(
    document: &Html,
    whitespace: WhitespaceMode,
    intent: TextRenderIntent,
) -> String {
    if let Some(body) = first_body(document) {
        tree::render_children_as_text(body.children(), whitespace, intent)
    } else {
        tree::render_children_as_text(document.root_element().children(), whitespace, intent)
    }
}

pub(crate) fn render_element_children_as_text(
    node: &ElementRef<'_>,
    whitespace: WhitespaceMode,
) -> String {
    tree::render_children_as_text(
        node.children(),
        whitespace,
        TextRenderIntent::ReaderDocument,
    )
}

pub(crate) fn render_element_as_text(node: &ElementRef<'_>, whitespace: WhitespaceMode) -> String {
    let mut output = String::new();
    tree::render_node_with_intent(
        **node,
        &mut output,
        false,
        false,
        TextRenderIntent::SelectedFragment,
        true,
    );
    format::normalize_rendered_output(output, whitespace)
}

pub(crate) fn extract_heading_text(node: &ElementRef<'_>) -> Option<String> {
    let mut rendered = String::new();
    for child in node.children() {
        tree::render_heading_text_node(child, &mut rendered, false);
    }

    let heading_text = format::normalize_heading_text(&rendered);
    (!heading_text.is_empty()).then_some(heading_text)
}

pub(crate) use format::{apply_whitespace_mode, collapse_inline_whitespace, needs_space};
#[cfg(test)]
pub(crate) use format::{collapse_blank_lines_for_tests, push_newline};
#[cfg(test)]
pub(crate) use tree::render_node;

mod format;
mod list;
mod math;
mod media;
mod table;
mod tree;

#[cfg(test)]
mod tests;
