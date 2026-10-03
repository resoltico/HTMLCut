//! Conventional Markdown from the original immutable DOM, with explicit source context.

use super::markdown_annotations::{CodeReferences, annotations};
use super::*;
use super::{context::ListContext, markdown_writer::MarkdownWriter};

use super::context::html;
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
                    "pre"
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
    header: bool,
    closes_code: bool,
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
    let mut code_open = false;
    let mut fence = 3;
    let mut code = CodeReferences::default();
    let mut lists = ListContext::default();
    let inherited = context::inherited_pre(root, budget)?
        && !excluded.contains(&root.id())
        && !omitted_payload(root.value());
    if inherited {
        fence = fence_length(root, excluded, budget)?;
        output.fence(fence)?;
        output.literal("\n")?;
        code_open = true;
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
                    if code_open {
                        output.literal(&text.text)?;
                    } else {
                        output.text(&text.text)?;
                    }
                    continue;
                }
                let Some(e) = node.value().as_element() else {
                    continue;
                };
                let mut frame = Frame {
                    prefix: output.prefix.len(),
                    ..Default::default()
                };
                if html(e) {
                    frame.name = e.name();
                }
                if code_open {
                    if frame.name == "a" && e.attr("href").is_some() {
                        code.links.push(ElementRef::wrap(node).expect("element"));
                    }
                    if frame.name == "img" && (e.attr("alt").is_some() || e.attr("src").is_some()) {
                        code.images.push(ElementRef::wrap(node).expect("element"));
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
                            output.headers += 1;
                            frame.header = true;
                        }
                    }
                    "br" => output.boundary(1)?,
                    "pre" => {
                        output.block_start(2)?;
                        fence = fence_length(
                            ElementRef::wrap(node).expect("element"),
                            excluded,
                            budget,
                        )?;
                        output.fence(fence)?;
                        output.literal("\n")?;
                        code_open = true;
                        frame.closes_code = true;
                    }
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
                    code_open = false;
                    output.literal("\n")?;
                    output.fence(fence)?;
                    output.boundary(2)?;
                    annotations(
                        &mut output,
                        &mut code,
                        excluded,
                        resolve,
                        base,
                        maximum,
                        budget,
                    )?;
                } else if !code_open {
                    if frame.link {
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
                    if frame.header {
                        output.close_bold()?;
                        output.headers -= 1;
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
            }
        }
    }
    if inherited {
        output.literal("\n")?;
        output.fence(fence)?;
        output.boundary(2)?;
        annotations(
            &mut output,
            &mut code,
            excluded,
            resolve,
            base,
            maximum,
            budget,
        )?;
    }
    output.finish()
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
