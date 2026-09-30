//! Faithful structural formatting; no class, ID, visibility or caption heuristics.

use super::*;

struct List {
    ordered: bool,
    next: Option<i64>,
    reversed: bool,
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
    let mut output = ValueBuffer::new(maximum, budget);
    let mut skipped = 0_u32;
    let mut pending = false;
    let mut lists: Vec<List> = Vec::new();
    let mut tables: Vec<Table> = Vec::new();
    let mut pre_fences: Vec<String> = Vec::new();
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
                let name = element.name();
                match name {
                    "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                        output.boundary()?;
                        pending = false;
                        output.push(&"#".repeat((name.as_bytes()[1] - b'0') as usize))?;
                        output.push(" ")?;
                    }
                    "p" | "div" | "article" | "section" | "header" | "footer" | "main"
                    | "aside" | "blockquote" | "figure" | "figcaption" | "caption" => {
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
                        let reversed = element.attr("reversed").is_some();
                        let default_start = if reversed {
                            let mut count = 0_i64;
                            for child in node.children() {
                                crate::execution::charge(budget, 1)?;
                                if !excluded.contains(&child.id())
                                    && child.value().as_element().is_some_and(|e| e.name() == "li")
                                {
                                    count += 1;
                                }
                            }
                            count
                        } else {
                            1
                        };
                        lists.push(List {
                            ordered: name == "ol",
                            reversed,
                            next: Some(ordinal(element.attr("start"))?.unwrap_or(default_start)),
                        });
                    }
                    "li" => {
                        output.boundary()?;
                        pending = false;
                        output.push(&"  ".repeat(lists.len().saturating_sub(1)))?;
                        if let Some(list) = lists.last_mut().filter(|list| list.ordered) {
                            let number = ordinal(element.attr("value"))?
                                .or(list.next)
                                .ok_or_else(|| ExtractionError::limit("rendering"))?;
                            output.push(&format!("{number}. "))?;
                            list.next = if list.reversed {
                                number.checked_sub(1)
                            } else {
                                number.checked_add(1)
                            };
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
                    "table" => {
                        output.boundary()?;
                        output.push("[table]")?;
                        pending = true;
                        tables.push(Table { cells: 0 });
                    }
                    "tr" => {
                        output.boundary()?;
                        pending = false;
                        if let Some(table) = tables.last_mut() {
                            table.cells = 0;
                        }
                    }
                    "td" | "th" => {
                        if let Some(table) = tables.last_mut() {
                            if table.cells > 0 {
                                output.push(" | ")?;
                            }
                            table.cells += 1;
                        }
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
                        output.text(destination, false, false, true)?;
                        output.push(")")?;
                    }
                    "pre" => {
                        output.boundary()?;
                        output.push(&pre_fences.pop().unwrap())?;
                        pending = true;
                    }
                    "ol" | "ul" => {
                        lists.pop();
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
                    | "figure" | "figcaption" | "caption" | "li" | "tr" => pending = true,
                    _ => (),
                }
            }
        }
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

fn ordinal(value: Option<&str>) -> Result<Option<i64>, ExtractionError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let value = value.trim_start_matches(|c: char| c.is_ascii_whitespace());
    let sign = usize::from(value.starts_with('+') || value.starts_with('-'));
    let digits = value[sign..].bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 {
        return Ok(None);
    }
    value[..sign + digits]
        .parse()
        .map(Some)
        .map_err(|_| ExtractionError::limit("rendering"))
}

#[cfg(test)]
#[path = "../tests/document_text_internal.rs"]
mod tests;
