//! Conventional Markdown from the original immutable DOM, with explicit source context.

use super::markdown_annotations::{CodeReferences, annotations};
use super::*;
use super::{context::ListContext, markdown_writer::MarkdownWriter};

use super::context::html;
pub(super) fn hidden_payload(element: &scraper::node::Element) -> bool {
    let namespace: &str = element.name.ns.as_ref();
    (html(element) && matches!(element.name(), "script" | "style" | "template"))
        || (namespace == "http://www.w3.org/2000/svg"
            && matches!(element.name(), "script" | "style"))
}
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
    let mut skipped = 0;
    for edge in root.traverse() {
        crate::execution::charge(budget, 1)?;
        match edge {
            Edge::Open(node) => {
                if skipped > 0
                    || excluded.contains(&node.id())
                    || node.value().as_element().is_some_and(hidden_payload)
                {
                    skipped += 1;
                    continue;
                }
                if node.id() == root.id() {
                    continue;
                }
                if let Some(e) = node.value().as_element()
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
            Edge::Close(_) if skipped > 0 => skipped -= 1,
            _ => (),
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
    pre: bool,
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
    let mut skipped = 0;
    let mut pre = 0;
    let mut fence = String::new();
    let mut code = CodeReferences::default();
    let mut lists = ListContext::default();
    let inherited = context::inherited_pre(root, budget)?
        && !excluded.contains(&root.id())
        && !hidden_payload(root.value());
    if inherited {
        fence = pre_fence(root, excluded, maximum, budget)?;
        output.syntax(&fence)?;
        output.literal("\n")?;
        pre = 1;
    }
    if !inherited && html(root.value()) && matches!(root.value().name(), "td" | "th") {
        output.syntax("-")?;
        output.prefix.push_str("  ");
        output.boundary(1)?;
    }
    for edge in root.traverse() {
        crate::execution::charge(budget, 1)?;
        match edge {
            Edge::Open(node) => {
                if skipped > 0
                    || excluded.contains(&node.id())
                    || node.value().as_element().is_some_and(hidden_payload)
                {
                    skipped += 1;
                    continue;
                }
                if let Node::Text(text) = node.value() {
                    if pre > 0 {
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
                if pre > 0 {
                    if frame.name == "a" && e.attr("href").is_some() {
                        code.links.push(ElementRef::wrap(node).expect("element"));
                    }
                    if frame.name == "img" && (e.attr("alt").is_some() || e.attr("src").is_some()) {
                        code.images.push(ElementRef::wrap(node).expect("element"));
                    }
                    // Formatting within code is literal text, not Markdown syntax.
                    if frame.name == "pre" {
                        pre += 1;
                        frame.pre = true;
                    }
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
                        fence = pre_fence(
                            ElementRef::wrap(node).expect("element"),
                            excluded,
                            maximum,
                            budget,
                        )?;
                        output.syntax(&fence)?;
                        output.literal("\n")?;
                        pre = 1;
                        frame.pre = true;
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
                if skipped > 0 {
                    skipped -= 1;
                    continue;
                }
                let Some(e) = node.value().as_element() else {
                    continue;
                };
                let frame = frames.pop().expect("each included element has a frame");
                if frame.pre {
                    pre -= 1;
                    if pre == 0 {
                        output.literal("\n")?;
                        output.syntax(&fence)?;
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
                } else if pre == 0 {
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
        output.syntax(&fence)?;
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

fn pre_fence(
    root: ElementRef<'_>,
    excluded: &HashSet<NodeId>,
    maximum: usize,
    budget: &SelectorWorkBudget,
) -> Result<String, ExtractionError> {
    let mut longest = 0;
    let mut run = 0;
    let mut skipped = 0;
    for edge in root.traverse() {
        crate::execution::charge(budget, 1)?;
        match edge {
            Edge::Open(node) => {
                if skipped > 0
                    || excluded.contains(&node.id())
                    || node.value().as_element().is_some_and(hidden_payload)
                {
                    skipped += 1;
                    continue;
                }
                if let Node::Text(text) = node.value() {
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
            Edge::Close(_) if skipped > 0 => skipped -= 1,
            _ => (),
        }
    }
    let length = 3.max(longest + 1);
    if length > maximum {
        return Err(ExtractionError::limit("projection"));
    }
    Ok("`".repeat(length))
}
