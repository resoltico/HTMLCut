// SPDX-License-Identifier: MPL-2.0
//! Link/image metadata inside preformatted content is emitted outside the unchanged code.

use super::markdown_traversal::payload_edges;
use super::markdown_writer::MarkdownWriter;
use super::*;

#[derive(Default)]
pub(super) struct CodeReferences<'a> {
    pub(super) links: Vec<ElementRef<'a>>,
    pub(super) images: Vec<ElementRef<'a>>,
}

pub(super) struct AnnotationContext<'a> {
    pub(super) excluded: &'a HashSet<NodeId>,
    pub(super) resolve: bool,
    pub(super) base: Option<&'a str>,
    pub(super) maximum: usize,
    pub(super) budget: &'a SelectorWorkBudget,
}

pub(super) fn annotations(
    output: &mut MarkdownWriter<'_>,
    references: &mut CodeReferences<'_>,
    context: &AnnotationContext<'_>,
    inline: bool,
) -> Result<(), ExtractionError> {
    let AnnotationContext {
        excluded,
        resolve,
        base,
        maximum,
        budget,
    } = *context;
    for link in references.links.drain(..) {
        if inline {
            output.inline_separator()?;
        } else {
            output.boundary(1)?;
            output.syntax("- ")?;
        }
        output.begin_inline()?;
        output.syntax("[")?;
        if inline {
            output.text("link")?;
        } else {
            let label = label(link, excluded, maximum, budget)?;
            output.text(label.trim_matches(|c: char| c.is_ascii_whitespace()))?;
        }
        output.syntax("](")?;
        let href = link.attr("href").expect("collected link has a destination");
        let destination = if resolve {
            std::borrow::Cow::Owned(resolve_url(href, base, maximum)?)
        } else {
            std::borrow::Cow::Borrowed(href)
        };
        output.destination(&destination)?;
        output.syntax(")")?;
    }
    for image in references.images.drain(..) {
        if inline {
            output.inline_separator()?;
        } else {
            output.boundary(1)?;
            output.syntax("- ")?;
        }
        if let Some(src) = image.attr("src") {
            output.begin_inline()?;
            output.syntax("![")?;
            output.text(image.attr("alt").unwrap_or(""))?;
            output.syntax("](")?;
            let destination = if resolve {
                std::borrow::Cow::Owned(resolve_url(src, base, maximum)?)
            } else {
                std::borrow::Cow::Borrowed(src)
            };
            output.destination(&destination)?;
            output.syntax(")")?;
        } else {
            output.text(image.attr("alt").unwrap_or(""))?;
        }
    }
    if !inline && !output.prefix.is_empty() {
        output.boundary(2)?;
    }
    Ok(())
}

fn label(
    root: ElementRef<'_>,
    excluded: &HashSet<NodeId>,
    maximum: usize,
    budget: &SelectorWorkBudget,
) -> Result<String, ExtractionError> {
    let mut output = ValueBuffer::new(maximum, budget);
    for edge in payload_edges(root, excluded, budget) {
        if let Edge::Open(node) = edge?
            && let Node::Text(text) = node.value()
        {
            output.text(&text.text, false, true)?;
        }
    }
    Ok(output.finish())
}
