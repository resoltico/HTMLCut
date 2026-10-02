//! Faithful structural formatting; no class, ID, visibility or caption heuristics.

use super::*;

use super::context::ListContext;

fn foreign_table_role(element: &scraper::node::Element) -> bool {
    let namespace: &str = element.name.ns.as_ref();
    matches!(
        element.name(),
        "table" | "caption" | "tr" | "td" | "th" | "tbody" | "thead" | "tfoot"
    ) && namespace != "http://www.w3.org/1999/xhtml"
}

fn missing_table_context() -> ExtractionError {
    ExtractionError::new(
        ErrorCode::InternalInvariant,
        "projection",
        "An HTML table role is missing its structural context.",
    )
}

struct Table {
    cells: usize,
}

pub(super) fn render(
    root: ElementRef<'_>,
    excluded: &HashSet<NodeId>,
    normalize: bool,
    resolve: bool,
    base: Option<&str>,
    maximum: usize,
    budget: &SelectorWorkBudget,
) -> Result<String, ExtractionError> {
    #[cfg(test)]
    record_projection(2);
    let mut output = ValueBuffer::new(maximum, budget);
    let mut skipped = 0_u32;
    let mut pending = false;
    let mut list_depth = usize::from(
        root.value().name() == "li"
            && root
                .parent()
                .and_then(ElementRef::wrap)
                .is_some_and(|e| matches!(e.value().name(), "ol" | "ul")),
    );
    let mut list_context = ListContext::default();
    let mut tables: Vec<Table> = Vec::new();
    let mut pre_fences: Vec<String> = Vec::new();
    let inherited_pre = context::inherited_pre(root, budget)?
        && !excluded.contains(&root.id())
        && !matches!(root.value().name(), "script" | "style" | "template");
    if inherited_pre {
        let fence = pre_fence(*root, excluded, maximum, budget)?;
        output.push(&fence)?;
        output.push("\n")?;
        pre_fences.push(fence);
    }
    if !foreign_table_role(root.value())
        && matches!(
            root.value().name(),
            "tr" | "tbody" | "thead" | "tfoot" | "td" | "th"
        )
    {
        tables.push(Table { cells: 0 });
    }

    for edge in root.traverse() {
        crate::execution::charge(budget, 1)?;
        match edge {
            Edge::Open(node) => {
                let element = node.value().as_element();
                if skipped > 0
                    || excluded.contains(&node.id())
                    || element.is_some_and(|element| {
                        matches!(element.name(), "script" | "style" | "template")
                    })
                {
                    skipped += 1;
                    continue;
                }
                if let Node::Text(text) = node.value() {
                    if pending && !text.text.is_empty() {
                        output.boundary()?;
                        pending = false;
                    }
                    output.text(
                        &text.text,
                        !pre_fences.is_empty(),
                        normalize,
                        pre_fences.is_empty(),
                    )?;
                    continue;
                }
                let Some(element) = element else {
                    continue;
                };
                if foreign_table_role(element) {
                    continue;
                }
                let name = element.name();
                match name {
                    "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                        output.boundary()?;
                        pending = false;
                        output.push(&"#".repeat((name.as_bytes()[1] - b'0') as usize))?;
                        output.push(" ")?;
                    }
                    "p" | "div" | "article" | "section" | "header" | "footer" | "main"
                    | "aside" | "blockquote" | "figure" | "figcaption" | "dl" | "dt" | "dd"
                    | "details" | "summary" | "address" => {
                        output.boundary()?;
                        pending = false;
                    }
                    "br" => {
                        output.push("\n")?;
                        pending = false;
                    }
                    "ul" | "ol" => {
                        output.boundary()?;
                        pending = false;
                        list_depth += 1;
                    }
                    "li" => {
                        output.boundary()?;
                        pending = false;
                        output.push(&"  ".repeat(list_depth.saturating_sub(1)))?;
                        if let Some(number) = list_context
                            .ordinal(ElementRef::wrap(node).expect("element"), budget)?
                        {
                            output.push(&format!("{number}. "))?;
                        } else {
                            output.push("- ")?;
                        }
                    }
                    "a" if element.attr("href").is_some() => {
                        if pending {
                            output.boundary()?;
                            pending = false;
                        }
                        output.push("[")?;
                    }
                    "img" => {
                        if let Some(alt) = element.attr("alt") {
                            if pending && !alt.is_empty() {
                                output.boundary()?;
                                pending = false;
                            }
                            output.text(
                                alt,
                                !pre_fences.is_empty(),
                                normalize,
                                pre_fences.is_empty(),
                            )?;
                        }
                    }
                    "pre" => {
                        output.boundary()?;
                        pending = false;
                        let fence = pre_fence(node, excluded, maximum, budget)?;
                        output.push(&fence)?;
                        output.push("\n")?;
                        pre_fences.push(fence);
                    }
                    "caption" => {
                        output.boundary()?;
                        output.push("[caption]")?;
                        pending = false;
                    }
                    "table" => {
                        output.boundary()?;
                        output.push("[table]")?;
                        pending = true;
                        tables.push(Table { cells: 0 });
                    }
                    "tr" => {
                        output.boundary()?;
                        pending = false;
                        tables.last_mut().ok_or_else(missing_table_context)?.cells = 0;
                    }
                    "td" | "th" => {
                        let table = tables.last_mut().ok_or_else(missing_table_context)?;
                        if table.cells > 0 {
                            output.push(" | ")?;
                        }
                        table.cells += 1;
                        output.push("[cell]")?;
                        if name == "th" {
                            output.push("[header] ")?;
                        }
                        for attr in ["rowspan", "colspan"] {
                            if let Some(value) = element.attr(attr) {
                                output.push("[")?;
                                output.push(attr)?;
                                output.push("=")?;
                                output.text(value, false, false, true)?;
                                output.push("] ")?;
                            }
                        }
                        pending = false;
                    }
                    _ => (),
                }
            }
            Edge::Close(node) => {
                if skipped > 0 {
                    skipped -= 1;
                    continue;
                }
                let Some(element) = node.value().as_element() else {
                    continue;
                };
                if foreign_table_role(element) {
                    continue;
                }
                match element.name() {
                    "a" if element.attr("href").is_some() => {
                        output.push("](")?;
                        let destination = element.attr("href").unwrap();
                        let resolved;
                        let destination = if resolve {
                            resolved = resolve_url(destination, base, maximum)?;
                            &resolved
                        } else {
                            destination
                        };
                        output.text(
                            destination,
                            !pre_fences.is_empty(),
                            false,
                            pre_fences.is_empty(),
                        )?;
                        output.push(")")?;
                    }
                    "pre" => {
                        // The separator belongs to framing, even when payload already ends in LF.
                        output.push("\n")?;
                        output.push(&pre_fences.pop().unwrap())?;
                        pending = true;
                    }
                    "ol" | "ul" => {
                        list_depth -= 1;
                        pending = true;
                    }
                    "td" | "th" => {
                        output.push("[/cell]")?;
                        pending = false;
                    }
                    "caption" => {
                        output.push("[/caption]")?;
                        pending = true;
                    }
                    "table" => {
                        output.boundary()?;
                        output.push("[/table]")?;
                        tables.pop();
                        pending = true;
                    }
                    "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "p" | "div" | "article"
                    | "section" | "header" | "footer" | "main" | "aside" | "blockquote"
                    | "figure" | "figcaption" | "dl" | "dt" | "dd" | "details" | "summary"
                    | "address" | "li" | "tr" => pending = true,
                    _ => (),
                }
            }
        }
    }
    if inherited_pre {
        output.push("\n")?;
        output.push(&pre_fences.pop().expect("inherited fence"))?;
    }
    Ok(output.finish())
}

fn pre_fence(
    root: ego_tree::NodeRef<'_, Node>,
    excluded: &HashSet<NodeId>,
    maximum: usize,
    budget: &SelectorWorkBudget,
) -> Result<String, ExtractionError> {
    let mut longest = 0;
    let mut run = 0;
    let mut skipped = 0;
    let mut nested_pre = 0_usize;
    let mut deepest_pre = 0_usize;
    for edge in root.traverse() {
        crate::execution::charge(budget, 1)?;
        match edge {
            Edge::Open(node) => {
                if skipped > 0
                    || excluded.contains(&node.id())
                    || node
                        .value()
                        .as_element()
                        .is_some_and(|e| matches!(e.name(), "script" | "style" | "template"))
                {
                    skipped += 1;
                    continue;
                }
                let mut segments = Vec::new();
                if let Some(element) = node.value().as_element() {
                    if element.name() == "pre" && node.id() != root.id() {
                        nested_pre += 1;
                        deepest_pre = deepest_pre.max(nested_pre);
                    }
                    if element.name() == "img"
                        && let Some(alt) = element.attr("alt")
                    {
                        segments.push(alt);
                    }
                    if element.name() == "a"
                        && let Some(href) = element.attr("href")
                    {
                        segments.push(href);
                    }
                }
                if let Node::Text(text) = node.value() {
                    segments.push(&text.text);
                }
                for segment in segments {
                    crate::execution::charge(budget, segment.len().div_ceil(64))?;
                    for c in segment.chars() {
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
            Edge::Close(node)
                if node.id() != root.id()
                    && node.value().as_element().is_some_and(|e| e.name() == "pre") =>
            {
                nested_pre -= 1
            }
            _ => (),
        }
    }
    let length = 3.max(longest + 1) + deepest_pre;
    if length > maximum {
        return Err(ExtractionError::limit("projection"));
    }
    Ok("`".repeat(length))
}

#[cfg(test)]
#[path = "../tests/document_text_internal.rs"]
mod tests;
