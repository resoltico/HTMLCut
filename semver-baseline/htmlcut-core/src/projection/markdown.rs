// SPDX-License-Identifier: MPL-2.0
//! Conventional Markdown from the original immutable DOM, with explicit source context.

use super::markdown_annotations::{AnnotationContext, CodeReferences, annotations};
use super::*;
use super::{context::ListContext, markdown_writer::MarkdownWriter};

use super::context::html;
use super::markdown_inline::{self, Style, Styles};
use super::markdown_traversal::{omitted_payload, payload_edges};
fn block(name: &str) -> bool {
    matches!(
        name,
        "p" | "div"
            | "article"
            | "section"
            | "header"
            | "footer"
            | "main"
            | "aside"
            | "figure"
            | "figcaption"
            | "dl"
            | "dt"
            | "dd"
            | "details"
            | "summary"
            | "address"
            | "caption"
    )
}
fn complex_link(
    root: ElementRef<'_>,
    excluded: &HashSet<NodeId>,
    budget: &SelectorWorkBudget,
) -> Result<bool, ExtractionError> {
    // The caller supplies an HTML anchor; its own tag is never a block role.
    for edge in payload_edges(root, excluded, budget) {
        if let Edge::Open(node) = edge?
            && let Some(e) = node.value().as_element()
            && html(e)
            && (block(e.name())
                || matches!(
                    e.name(),
                    "br" | "pre"
                        | "table"
                        | "ul"
                        | "ol"
                        | "h1"
                        | "h2"
                        | "h3"
                        | "h4"
                        | "h5"
                        | "h6"
                        | "blockquote"
                ))
        {
            return Ok(true);
        }
    }
    Ok(false)
}

#[derive(Default)]
struct Frame<'a> {
    prefix: usize,
    name: &'a str,
    link: bool,
    complex: bool,
    styles: Styles,
    closes_code: bool,
}

enum CodeMode<'a> {
    Block(usize),
    Inline(ValueBuffer<'a>),
}

pub(super) fn render(
    root: ElementRef<'_>,
    excluded: &HashSet<NodeId>,
    resolve: bool,
    base: Option<&str>,
    maximum: usize,
    budget: &SelectorWorkBudget,
) -> Result<String, ExtractionError> {
    #[cfg(test)]
    super::record_projection(2);
    let mut output = MarkdownWriter::new(maximum, budget);
    let mut frames = Vec::new();
    let mut mode = None;
    let mut references = CodeReferences::default();
    let annotation_context = AnnotationContext {
        excluded,
        resolve,
        base,
        maximum,
        budget,
    };
    let mut lists = ListContext::default();
    let original = markdown_inline::ancestry(root, budget)?;
    output.styles = original.styles;
    let inherited = (original.pre.is_some() || original.code)
        && !excluded.contains(&root.id())
        && !omitted_payload(root.value());
    if inherited {
        if let Some(pre) = original.pre {
            let fence = fence_length(root, excluded, budget)?;
            output.fence(fence)?;
            if let Some(language) = markdown_inline::language(pre, budget)? {
                output.literal(&language)?;
            }
            output.literal("\n")?;
            mode = Some(CodeMode::Block(fence));
        } else {
            mode = Some(CodeMode::Inline(ValueBuffer::new(maximum, budget)));
        }
    }
    if !inherited && html(root.value()) && matches!(root.value().name(), "td" | "th") {
        output.syntax("-")?;
        output.prefix.push_str("  ");
        output.boundary(1)?;
    }
    for edge in payload_edges(root, excluded, budget) {
        let edge = edge?;
        match edge {
            Edge::Open(node) => {
                if let Node::Text(text) = node.value() {
                    match &mut mode {
                        Some(CodeMode::Block(_)) => output.literal(&text.text)?,
                        Some(CodeMode::Inline(payload)) => payload.text(&text.text, true, false)?,
                        None => output.text(&text.text)?,
                    }
                    continue;
                }
                let Some(e) = node.value().as_element() else {
                    continue;
                };
                let mut frame = Frame {
                    prefix: output.prefix.len(),
                    styles: output.styles,
                    ..Default::default()
                };
                if html(e) {
                    frame.name = e.name();
                }
                if mode.is_some() {
                    if frame.name == "a" && e.attr("href").is_some() {
                        references
                            .links
                            .push(ElementRef::wrap(node).expect("element"));
                    }
                    if frame.name == "img" && (e.attr("alt").is_some() || e.attr("src").is_some()) {
                        references
                            .images
                            .push(ElementRef::wrap(node).expect("element"));
                    }
                    // Formatting within code is literal text, not Markdown syntax.
                    frames.push(frame);
                    continue;
                }
                match frame.name {
                    "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                        output.block_start(2)?;
                        output.syntax(&"#".repeat((e.name().as_bytes()[1] - b'0') as usize))?;
                        output.syntax(" ")?;
                    }
                    "blockquote" => {
                        let empty = output.empty_item();
                        output.block_start(2)?;
                        if empty {
                            output.syntax("> ")?;
                        }
                        output.prefix.push_str("> ");
                    }
                    "ul" | "ol" => {
                        output.block_start(if output.prefix.is_empty() { 2 } else { 1 })?
                    }
                    "table" => output.block_start(2)?,
                    "li" => {
                        output.boundary(1)?;
                        output.syntax("- ")?;
                        if let Some(number) =
                            lists.ordinal(ElementRef::wrap(node).expect("element"), budget)?
                        {
                            output.syntax(&format!("{number}\\. "))?;
                        }
                        output.prefix.push_str("  ");
                    }
                    "tr" => {
                        output.boundary(1)?;
                        output.syntax("-")?;
                        output.prefix.push_str("  ");
                        output.boundary(1)?;
                    }
                    "td" | "th" => {
                        output.boundary(1)?;
                        output.syntax("- ")?;
                        output.prefix.push_str("  ");
                        if frame.name == "th" {
                            output.styles = output.styles.with(Style::Strong);
                        }
                    }
                    "br" => output.boundary(1)?,
                    "pre" => {
                        output.block_start(2)?;
                        let fence = fence_length(
                            ElementRef::wrap(node).expect("element"),
                            excluded,
                            budget,
                        )?;
                        output.fence(fence)?;
                        if let Some(language) = markdown_inline::language(
                            ElementRef::wrap(node).expect("element"),
                            budget,
                        )? {
                            output.literal(&language)?;
                        }
                        output.literal("\n")?;
                        mode = Some(CodeMode::Block(fence));
                        frame.closes_code = true;
                    }
                    "code" => {
                        mode = Some(CodeMode::Inline(ValueBuffer::new(maximum, budget)));
                        frame.closes_code = true;
                    }
                    "em" | "i" => output.styles = output.styles.with(Style::Emphasis),
                    "strong" | "b" => output.styles = output.styles.with(Style::Strong),
                    "a" if e.attr("href").is_some() => {
                        frame.link = true;
                        frame.complex = complex_link(
                            ElementRef::wrap(node).expect("element"),
                            excluded,
                            budget,
                        )?;
                        if !frame.complex {
                            output.begin_inline()?;
                            output.syntax("[")?;
                        }
                    }
                    "img" => {
                        output.begin_inline()?;
                        if let Some(src) = e.attr("src") {
                            output.syntax("![")?;
                            output.text(e.attr("alt").unwrap_or(""))?;
                            output.syntax("](")?;
                            let destination = if resolve {
                                std::borrow::Cow::Owned(resolve_url(src, base, maximum)?)
                            } else {
                                std::borrow::Cow::Borrowed(src)
                            };
                            output.destination(&destination)?;
                            output.syntax(")")?;
                            output.mark_atom();
                        } else {
                            output.text(e.attr("alt").unwrap_or(""))?;
                        }
                    }
                    name if block(name) => output.block_start(2)?,
                    _ => (),
                }
                frames.push(frame);
            }
            Edge::Close(node) => {
                let Some(e) = node.value().as_element() else {
                    continue;
                };
                let frame = frames.pop().expect("each included element has a frame");
                if frame.closes_code {
                    finish_code(
                        &mut output,
                        mode.take().expect("opening frame owns code"),
                        &mut references,
                        &annotation_context,
                    )?;
                } else if mode.is_none() {
                    if frame.link {
                        output.settle_styles()?;
                        if frame.complex {
                            output.boundary(2)?;
                            output.begin_inline()?;
                            output.syntax("[link](")?;
                        } else {
                            output.syntax("](")?;
                        }
                        let href = e.attr("href").expect("link frame requires href");
                        let destination = if resolve {
                            std::borrow::Cow::Owned(resolve_url(href, base, maximum)?)
                        } else {
                            std::borrow::Cow::Borrowed(href)
                        };
                        output.destination(&destination)?;
                        output.syntax(")")?;
                    }
                    if block(frame.name)
                        || matches!(
                            frame.name,
                            "h1" | "h2"
                                | "h3"
                                | "h4"
                                | "h5"
                                | "h6"
                                | "blockquote"
                                | "ul"
                                | "ol"
                                | "table"
                        )
                    {
                        output.boundary(2)?;
                    } else if matches!(frame.name, "li" | "tr" | "td" | "th") {
                        output.boundary(1)?;
                    }
                }
                output.prefix.truncate(frame.prefix);
                output.styles = frame.styles;
            }
        }
    }
    if inherited {
        finish_code(
            &mut output,
            mode.take().expect("inherited code remains open"),
            &mut references,
            &annotation_context,
        )?;
    }
    output.finish()
}

fn finish_code(
    output: &mut MarkdownWriter<'_>,
    mode: CodeMode<'_>,
    references: &mut CodeReferences<'_>,
    context: &AnnotationContext<'_>,
) -> Result<(), ExtractionError> {
    let inline = match mode {
        CodeMode::Block(fence) => {
            output.literal("\n")?;
            output.fence(fence)?;
            output.boundary(2)?;
            false
        }
        CodeMode::Inline(payload) => {
            output.code_span(&payload.finish())?;
            true
        }
    };
    annotations(output, references, context, inline)
}

fn fence_length(
    root: ElementRef<'_>,
    excluded: &HashSet<NodeId>,
    budget: &SelectorWorkBudget,
) -> Result<usize, ExtractionError> {
    let mut longest = 0;
    let mut run = 0;
    for edge in payload_edges(root, excluded, budget) {
        if let Edge::Open(node) = edge?
            && let Node::Text(text) = node.value()
        {
            crate::execution::charge(budget, text.text.len().div_ceil(64))?;
            for c in text.text.chars() {
                if c == '`' {
                    run += 1;
                    longest = longest.max(run);
                } else {
                    run = 0;
                }
            }
        }
    }
    Ok(3.max(longest + 1))
}
