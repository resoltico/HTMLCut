//! Link/image metadata inside preformatted content is emitted outside the unchanged code.

use super::markdown_writer::MarkdownWriter;
use super::*;

#[derive(Default)]
pub(super) struct CodeReferences<'a> {
    pub(super) links: Vec<ElementRef<'a>>,
    pub(super) images: Vec<ElementRef<'a>>,
}

pub(super) fn annotations(
    output: &mut MarkdownWriter<'_>,
    references: &mut CodeReferences<'_>,
    excluded: &HashSet<NodeId>,
    resolve: bool,
    base: Option<&str>,
    maximum: usize,
    budget: &SelectorWorkBudget,
) -> Result<(), ExtractionError> {
    for link in references.links.drain(..) {
        output.boundary(1)?;
        output.syntax("- [")?;
        let label = label(link, excluded, maximum, budget)?;
        output.text(label.trim_matches(|c: char| c.is_ascii_whitespace()))?;
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
        output.boundary(1)?;
        output.syntax("- ")?;
        if let Some(src) = image.attr("src") {
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
    if !output.prefix.is_empty() {
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
    let mut skipped = 0;
    for edge in root.traverse() {
        crate::execution::charge(budget, 1)?;
        match edge {
            Edge::Open(node) => {
                if skipped > 0
                    || excluded.contains(&node.id())
                    || node
                        .value()
                        .as_element()
                        .is_some_and(super::markdown::hidden_payload)
                {
                    skipped += 1;
                    continue;
                }
                if let Node::Text(text) = node.value() {
                    output.text(&text.text, false, true)?;
                }
            }
            Edge::Close(_) if skipped > 0 => skipped -= 1,
            _ => (),
        }
    }
    Ok(output.finish())
}
