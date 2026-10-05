// SPDX-License-Identifier: MPL-2.0
//! Read-only projections with bounded construction and explicit transforms.

use std::collections::HashSet;

use ego_tree::{NodeId, iter::Edge};
use scraper::{ElementRef, Node};
use selectors::work_budget::SelectorWorkBudget;

use crate::{ErrorCode, ExtractionError, Transform, ValueProjection};

mod context;
mod markdown;
mod markdown_annotations;
mod markdown_inline;
mod markdown_traversal;
mod markdown_writer;

pub(crate) struct ValueBuffer<'a> {
    value: String,
    maximum: usize,
    budget: &'a SelectorWorkBudget,
    pending_space: bool,
}

impl<'a> ValueBuffer<'a> {
    pub(crate) fn new(maximum: usize, budget: &'a SelectorWorkBudget) -> Self {
        Self {
            value: String::new(),
            maximum,
            budget,
            pending_space: false,
        }
    }
    pub(crate) fn push(&mut self, value: &str) -> Result<(), ExtractionError> {
        if value.len() > self.maximum.saturating_sub(self.value.len()) {
            return Err(ExtractionError::limit("projection"));
        }
        let size = self.value.len() + value.len(); // The preceding remaining-capacity check proves no overflow.
        crate::execution::charge(
            self.budget,
            size.div_ceil(64) - self.value.len().div_ceil(64),
        )?;
        self.value.push_str(value);
        Ok(())
    }
    pub(crate) fn text(
        &mut self,
        value: &str,
        pre: bool,
        normalize: bool,
    ) -> Result<(), ExtractionError> {
        crate::execution::charge(self.budget, value.len().div_ceil(64))?;
        if pre || !normalize {
            if !value.is_empty() {
                if self.pending_space {
                    self.push(" ")?;
                }
                self.pending_space = false;
                self.push(value)?;
            }
            return Ok(());
        }
        for c in value.chars() {
            if c.is_whitespace() {
                self.pending_space = !self.value.is_empty();
                continue;
            }
            if self.pending_space {
                self.push(" ")?;
                self.pending_space = false;
            }
            self.push(c.encode_utf8(&mut [0; 4]))?;
        }
        Ok(())
    }
    pub(crate) fn separator(&mut self) {
        self.pending_space = !self.value.is_empty();
    }
    pub(crate) fn finish(self) -> String {
        self.value
    }
}

pub(crate) fn project(
    root: ElementRef<'_>,
    projection: &ValueProjection,
    excluded: &HashSet<NodeId>,
    transforms: &[Transform],
    base: Option<&str>,
    maximum: usize,
    budget: &SelectorWorkBudget,
) -> Result<String, ExtractionError> {
    let normalize = transforms.contains(&Transform::NormalizeWhitespace {});
    let resolve = transforms.contains(&Transform::ResolveUrls {});
    match projection {
        ValueProjection::Attribute { name } => {
            #[cfg(test)]
            record_projection(0);
            let value = root.attr(name).ok_or_else(|| {
                ExtractionError::new(
                    ErrorCode::MissingAttribute,
                    "projection",
                    "A requested attribute is absent.",
                )
            })?;
            crate::execution::charge(budget, value.len().div_ceil(64) + 1)?;
            if value.len() > maximum {
                return Err(ExtractionError::limit("projection"));
            }
            let value = if resolve {
                resolve_url(value, base, maximum)?
            } else {
                value.to_owned()
            };
            // Untransformed values were bounded above; resolve_url bounds the final URL.
            Ok(value)
        }
        ValueProjection::DomText {} => dom_text(root, excluded, normalize, maximum, budget),
        ValueProjection::Markdown {} => {
            markdown::render(root, excluded, resolve, base, maximum, budget)
        }
        ValueProjection::InnerHtml {} | ValueProjection::OuterHtml {} => {
            #[cfg(test)]
            record_projection(3);
            let mut writer = HtmlBuffer {
                bytes: Vec::new(),
                maximum,
            };
            root.write_filtered_html(
                &mut writer,
                matches!(projection, ValueProjection::OuterHtml {}),
                excluded,
                budget,
            )
            .map_err(|_| ExtractionError::limit("serialization"))?;
            // The HTML serializer emits UTF-8; the owned buffer cannot be externally corrupted.
            Ok(String::from_utf8(writer.bytes).expect("HTML serializer produces UTF-8"))
        }
    }
}

fn dom_text(
    root: ElementRef<'_>,
    excluded: &HashSet<NodeId>,
    normalize: bool,
    maximum: usize,
    budget: &SelectorWorkBudget,
) -> Result<String, ExtractionError> {
    #[cfg(test)]
    record_projection(1);
    let mut value = ValueBuffer::new(maximum, budget);
    let mut skipped = 0_u32;
    let mut pre = u32::from(normalize && context::inherited_pre(root, budget)?);
    for edge in root.traverse() {
        crate::execution::charge(budget, 1)?;
        match edge {
            Edge::Open(node) => {
                if skipped > 0 || excluded.contains(&node.id()) {
                    skipped += 1;
                    continue;
                }
                if node
                    .value()
                    .as_element()
                    .is_some_and(|element| element.name() == "pre" && context::html(element))
                {
                    pre += 1;
                }
                if normalize && text_boundary(node.value()) {
                    value.separator();
                }
                if let Node::Text(text) = node.value() {
                    value.text(&text.text, pre > 0, normalize)?;
                }
            }
            Edge::Close(node) => {
                if skipped > 0 {
                    skipped -= 1;
                    continue;
                }
                if node
                    .value()
                    .as_element()
                    .is_some_and(|element| element.name() == "pre" && context::html(element))
                {
                    pre -= 1;
                }
                if normalize && text_boundary(node.value()) {
                    value.separator();
                }
            }
        }
    }
    Ok(value.finish())
}

fn text_boundary(node: &Node) -> bool {
    node.as_element().is_some_and(|element| {
        context::html(element)
            && matches!(
                element.name(),
                "br" | "p"
                    | "div"
                    | "article"
                    | "section"
                    | "header"
                    | "footer"
                    | "main"
                    | "aside"
                    | "figure"
                    | "figcaption"
                    | "blockquote"
                    | "pre"
                    | "ul"
                    | "ol"
                    | "li"
                    | "dl"
                    | "dt"
                    | "dd"
                    | "table"
                    | "caption"
                    | "tr"
                    | "th"
                    | "td"
                    | "h1"
                    | "h2"
                    | "h3"
                    | "h4"
                    | "h5"
                    | "h6"
            )
    })
}

pub(crate) fn resolve_url(
    value: &str,
    base: Option<&str>,
    maximum: usize,
) -> Result<String, ExtractionError> {
    if value.len() > crate::limits::MAX_URL_INPUT_BYTES {
        return Err(ExtractionError::limit("url"));
    }
    let parsed = if let Ok(url) = url::Url::parse(value) {
        url
    } else {
        let base = base.ok_or_else(|| {
            ExtractionError::new(
                ErrorCode::InvalidBaseUrl,
                "projection",
                "Resolving a relative URL requires explicit effective base metadata.",
            )
        })?;
        url::Url::parse(base)
            .and_then(|base| base.join(value))
            .map_err(|_| {
                ExtractionError::new(
                    ErrorCode::InvalidBaseUrl,
                    "projection",
                    "The requested URL could not be resolved.",
                )
            })?
    };
    if parsed.as_str().len() > maximum.min(crate::limits::MAX_URL_PROCESSING_BYTES) {
        return Err(ExtractionError::limit("url"));
    }
    Ok(parsed.into())
}

struct HtmlBuffer {
    bytes: Vec<u8>,
    maximum: usize,
}

impl std::io::Write for HtmlBuffer {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > self.maximum.saturating_sub(self.bytes.len()) {
            return Err(std::io::Error::other("Serialized value limit exceeded."));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
#[path = "tests/projection_writer.rs"]
mod writer_tests;

#[cfg(test)]
std::thread_local! {
    static PROJECTION_CALLS: std::cell::Cell<[u32; 4]> = const { std::cell::Cell::new([0; 4]) };
}

#[cfg(test)]
pub(crate) fn record_projection(index: usize) {
    PROJECTION_CALLS.with(|calls| {
        let mut values = calls.get();
        values[index] += 1;
        calls.set(values);
    });
}

#[cfg(test)]
pub(crate) fn take_projection_calls() -> [u32; 4] {
    PROJECTION_CALLS.with(|calls| calls.replace([0; 4]))
}
