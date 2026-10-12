// SPDX-License-Identifier: MPL-2.0 AND ISC
//! Deterministic structural text and original-tree readings.
use crate::{Code, Document, Error, contract::Reading, eval::Scope, limits::Clock};
use ego_tree::{NodeId, NodeRef, iter::Edge};
use html5ever::serialize::{Serialize, SerializeOpts, Serializer, TraversalScope, serialize};
use scraper::Node;
use std::{collections::HashSet, io};

pub(crate) fn read(
    document: &Document,
    scope: &Scope<'_>,
    reading: &Reading,
    excluded: &HashSet<NodeId>,
    clock: &Clock,
) -> Result<String, Error> {
    clock.check()?;
    let result = match reading {
        Reading::Attr(name) | Reading::Url(name) => {
            let value = attribute(scope.anchor.value(), name).ok_or_else(|| {
                Error::new(
                    Code::MissingAttribute,
                    format!(
                        "The selected <{}> element has no {name:?} attribute.",
                        scope.anchor.value().name()
                    ),
                )
            })?;
            if matches!(reading, Reading::Url(_)) {
                crate::document::resolve(value, document.base_url.as_ref())
                    .map(|u| u.into())
                    .ok_or_else(|| {
                        Error::new(
                            Code::UnresolvableUrl,
                            format!(
                                "URL {:?} cannot be resolved without a usable document base URL.",
                                value.chars().take(120).collect::<String>()
                            ),
                        )
                        .observed(value)
                    })?
            } else {
                value.into()
            }
        }
        Reading::TextContent => text_content(scope, excluded, clock)?,
        Reading::Text => text(scope, excluded, clock)?,
        Reading::OuterHtml => outer_html(scope, excluded, clock)?,
    };
    clock.check()?;
    Ok(result)
}
fn attribute<'a>(element: &'a scraper::node::Element, name: &str) -> Option<&'a str> {
    element.attrs.iter().find_map(|(key, value)| {
        let matches = match &key.prefix {
            Some(prefix) => name
                .strip_prefix(prefix.as_ref())
                .is_some_and(|tail| tail.strip_prefix(':') == Some(key.local.as_ref())),
            None => name == key.local.as_ref(),
        };
        matches.then_some(value.as_ref())
    })
}
fn text_content(
    scope: &Scope<'_>,
    excluded: &HashSet<NodeId>,
    clock: &Clock,
) -> Result<String, Error> {
    let mut result = String::new();
    for root in &scope.extent {
        let mut skipped = 0;
        for edge in root.traverse() {
            clock.tick()?;
            match edge {
                Edge::Open(node) if skipped > 0 || excluded.contains(&node.id()) => skipped += 1,
                Edge::Open(node) => {
                    if let Node::Text(text) = node.value() {
                        result.push_str(text);
                    }
                }
                Edge::Close(_) if skipped > 0 => skipped -= 1,
                Edge::Close(_) => (),
            }
        }
    }
    Ok(result)
}

const BLOCKS: &[&str] = &[
    "address",
    "article",
    "aside",
    "blockquote",
    "body",
    "caption",
    "center",
    "dd",
    "details",
    "dialog",
    "dir",
    "div",
    "dl",
    "dt",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "header",
    "hgroup",
    "hr",
    "html",
    "legend",
    "li",
    "listing",
    "main",
    "menu",
    "nav",
    "ol",
    "optgroup",
    "option",
    "p",
    "plaintext",
    "pre",
    "search",
    "section",
    "summary",
    "table",
    "tbody",
    "tfoot",
    "thead",
    "tr",
    "ul",
    "xmp",
];
fn html(element: &scraper::node::Element) -> bool {
    element.name.ns == html5ever::ns!(html)
}
fn preformatted(element: &scraper::node::Element) -> bool {
    html(element)
        && matches!(
            element.name(),
            "pre" | "listing" | "plaintext" | "xmp" | "textarea"
        )
}
struct Lines {
    lines: Vec<String>,
    cells: Vec<String>,
    verbatim: bool,
}
impl Lines {
    fn new() -> Self {
        Self {
            lines: Vec::new(),
            cells: vec![String::new()],
            verbatim: false,
        }
    }
    fn end(&mut self, keep_blank: bool, clock: &Clock) -> Result<(), Error> {
        let mut cells = std::mem::replace(&mut self.cells, vec![String::new()]);
        if !self.verbatim {
            for cell in &mut cells {
                *cell = collapsed(cell, clock)?;
            }
        }
        let line = cells.join("\t");
        if cells.iter().any(|c| !c.is_empty()) || keep_blank {
            self.lines.push(line);
        }
        self.verbatim = false;
        Ok(())
    }
    fn add(&mut self, text: &str, verbatim: bool, clock: &Clock) -> Result<(), Error> {
        if !verbatim {
            self.cells.last_mut().unwrap().push_str(text);
            return Ok(());
        }
        for c in text.chars() {
            clock.tick()?;
            self.verbatim = true;
            if c == '\n' {
                self.end(true, clock)?;
            } else {
                self.cells.last_mut().unwrap().push(c);
            }
        }
        Ok(())
    }
}
pub(crate) fn collapsed(text: &str, clock: &Clock) -> Result<String, Error> {
    let mut result = String::new();
    let mut space = false;
    for c in text.chars() {
        clock.tick()?;
        if c.is_whitespace() {
            space = !result.is_empty();
        } else {
            if space {
                result.push(' ');
            }
            result.push(c);
            space = false;
        }
    }
    Ok(result)
}
fn text(scope: &Scope<'_>, excluded: &HashSet<NodeId>, clock: &Clock) -> Result<String, Error> {
    if scope.extent.len() == 1
        && matches!(scope.anchor.value().name(), "script" | "style" | "template")
    {
        return text_content(scope, excluded, clock);
    }
    let sole = (scope.extent.len() == 1).then_some(scope.anchor.id());
    let mut lines = Lines::new();
    for root in &scope.extent {
        let mut skipped = 0;
        let mut verbatim = 0;
        for edge in root.traverse() {
            clock.tick()?;
            match edge {
                Edge::Open(node) => {
                    let element = node.value().as_element();
                    let hidden = sole != Some(node.id())
                        && element.is_some_and(|e| {
                            (html(e)
                                && matches!(e.name(), "script" | "style" | "template" | "head"))
                                || (e.name.ns == html5ever::ns!(svg)
                                    && matches!(e.name(), "script" | "style"))
                        });
                    if skipped > 0 || excluded.contains(&node.id()) || hidden {
                        skipped += 1;
                        continue;
                    }
                    match node.value() {
                        Node::Text(text) => lines.add(text, verbatim > 0, clock)?,
                        Node::Element(e) => {
                            if html(e) && e.name() == "br" {
                                lines.end(verbatim > 0, clock)?;
                            }
                            if html(e) && BLOCKS.contains(&e.name()) {
                                lines.end(false, clock)?;
                            }
                            if html(e) && matches!(e.name(), "td" | "th") && sole != Some(node.id())
                            {
                                let mut previous = node.prev_sibling();
                                while let Some(sibling) = previous {
                                    clock.tick()?;
                                    if sibling
                                        .value()
                                        .as_element()
                                        .is_some_and(|e| html(e) && matches!(e.name(), "td" | "th"))
                                    {
                                        if !excluded.contains(&sibling.id()) {
                                            lines.cells.push(String::new());
                                        }
                                        break;
                                    }
                                    previous = sibling.prev_sibling();
                                }
                            }
                            if preformatted(e) {
                                verbatim += 1;
                            }
                        }
                        _ => (),
                    }
                }
                Edge::Close(_) if skipped > 0 => skipped -= 1,
                Edge::Close(node) => {
                    if let Some(e) = node.value().as_element() {
                        if preformatted(e) {
                            verbatim -= 1;
                        }
                        if html(e) && BLOCKS.contains(&e.name()) {
                            lines.end(false, clock)?;
                        }
                    }
                }
            }
        }
    }
    lines.end(false, clock)?;
    Ok(lines.lines.join("\n"))
}

struct Filtered<'a, 'd> {
    root: NodeRef<'d, Node>,
    excluded: &'a HashSet<NodeId>,
    clock: &'a Clock,
}
impl Serialize for Filtered<'_, '_> {
    fn serialize<S: Serializer>(&self, serializer: &mut S, _: TraversalScope) -> io::Result<()> {
        let mut skipped = 0;
        for edge in self.root.traverse() {
            self.clock.tick().map_err(io::Error::other)?;
            match edge {
                Edge::Open(node) if skipped > 0 || self.excluded.contains(&node.id()) => {
                    skipped += 1
                }
                Edge::Open(node) => match node.value() {
                    Node::Doctype(value) => serializer.write_doctype(value.name())?,
                    Node::Comment(value) => serializer.write_comment(value)?,
                    Node::Text(value) => serializer.write_text(value)?,
                    Node::Element(value) => serializer.start_elem(
                        value.name.clone(),
                        value.attrs.iter().map(|(key, value)| (key, &value[..])),
                    )?,
                    _ => (),
                },
                Edge::Close(_) if skipped > 0 => skipped -= 1,
                Edge::Close(node) => {
                    if let Some(value) = node.value().as_element() {
                        serializer.end_elem(value.name.clone())?;
                    }
                }
            }
        }
        Ok(())
    }
}
fn outer_html(
    scope: &Scope<'_>,
    excluded: &HashSet<NodeId>,
    clock: &Clock,
) -> Result<String, Error> {
    let mut output = Vec::new();
    for &root in &scope.extent {
        serialize(
            &mut output,
            &Filtered {
                root,
                excluded,
                clock,
            },
            SerializeOpts {
                traversal_scope: TraversalScope::IncludeNode,
                ..Default::default()
            },
        )
        .map_err(|e| {
            e.get_ref()
                .and_then(|e| e.downcast_ref::<Error>())
                .cloned()
                .unwrap_or_else(|| {
                    Error::new(Code::Internal, format!("HTML serialization failed: {e}."))
                })
        })?;
    }
    String::from_utf8(output)
        .map_err(|_| Error::new(Code::Internal, "HTML serialization produced invalid UTF-8."))
}
