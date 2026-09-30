//! Read-only projections with bounded construction and explicit transforms.

use std::collections::HashSet;

use ego_tree::{NodeId, iter::Edge};
use scraper::{ElementRef, Node};
use selectors::work_budget::SelectorWorkBudget;

use crate::{ErrorCode, ExtractionError, Projection, Transform};

mod document_text;

pub(crate) struct ValueBuffer<'a> {
    value: String,
    maximum: usize,
    budget: &'a SelectorWorkBudget,
    source_space: bool,
}

impl<'a> ValueBuffer<'a> {
    pub(crate) fn new(maximum: usize, budget: &'a SelectorWorkBudget) -> Self {
        Self {
            value: String::new(),
            maximum,
            budget,
            source_space: false,
        }
    }
    pub(crate) fn push(&mut self, value: &str) -> Result<(), ExtractionError> {
        let size = self
            .value
            .len()
            .checked_add(value.len())
            .ok_or_else(|| ExtractionError::limit("projection"))?;
        if size > self.maximum {
            return Err(ExtractionError::limit("projection"));
        }
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
        escape: bool,
    ) -> Result<(), ExtractionError> {
        crate::execution::charge(self.budget, value.len().div_ceil(64))?;
        if (!normalize || pre) && !escape {
            self.source_space = false;
            return self.push(value);
        }
        for c in value.chars() {
            if normalize && !pre && c.is_ascii_whitespace() {
                if !self.source_space {
                    self.push(" ")?;
                    self.source_space = true;
                }
                continue;
            }
            self.source_space = false;
            if escape && !pre && "\\[]()".contains(c) {
                self.push("\\")?;
            }
            self.push(c.encode_utf8(&mut [0; 4]))?;
        }
        Ok(())
    }
    pub(crate) fn boundary(&mut self) -> Result<(), ExtractionError> {
        if !self.value.is_empty() && !self.value.ends_with('\n') {
            self.push("\n")?;
        }
        self.source_space = false;
        Ok(())
    }
    pub(crate) fn finish(self) -> String {
        self.value
    }
}

pub(crate) fn project(
    root: ElementRef<'_>,
    projection: &Projection,
    excluded: &HashSet<NodeId>,
    transforms: &[Transform],
    base: Option<&str>,
    maximum: usize,
    budget: &SelectorWorkBudget,
) -> Result<String, ExtractionError> {
    let normalize = transforms.contains(&Transform::NormalizeWhitespace);
    let resolve = transforms.contains(&Transform::ResolveUrls);
    match projection {
        Projection::Attribute { name } => {
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
                resolve_url(value, base)?
            } else {
                value.to_owned()
            };
            if value.len() > maximum {
                return Err(ExtractionError::limit("projection"));
            }
            Ok(value)
        }
        Projection::DomText => dom_text(root, excluded, normalize, maximum, budget),
        Projection::DocumentText => {
            document_text::render(root, excluded, normalize, resolve, base, maximum, budget)
        }
        Projection::InnerHtml | Projection::OuterHtml => {
            let mut writer = HtmlBuffer {
                bytes: Vec::new(),
                maximum,
            };
            root.write_filtered_html(
                &mut writer,
                matches!(projection, Projection::OuterHtml),
                excluded,
                budget,
            )
            .map_err(|_| ExtractionError::limit("serialization"))?;
            String::from_utf8(writer.bytes).map_err(|_| {
                ExtractionError::new(
                    ErrorCode::InternalInvariant,
                    "serialization",
                    "The DOM serializer produced invalid UTF-8.",
                )
            })
        }
        Projection::Source => Err(ExtractionError::new(
            ErrorCode::InternalInvariant,
            "projection",
            "Source projection must use the slice execution path.",
        )),
    }
}

fn dom_text(
    root: ElementRef<'_>,
    excluded: &HashSet<NodeId>,
    normalize: bool,
    maximum: usize,
    budget: &SelectorWorkBudget,
) -> Result<String, ExtractionError> {
    let mut value = ValueBuffer::new(maximum, budget);
    let mut skipped = 0_u32;
    let mut pre = 0_u32;
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
                    .is_some_and(|element| element.name() == "pre")
                {
                    pre += 1;
                }
                if let Node::Text(text) = node.value() {
                    value.text(&text.text, pre > 0, normalize, false)?;
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
                    .is_some_and(|element| element.name() == "pre")
                {
                    pre -= 1;
                }
            }
        }
    }
    Ok(value.finish())
}

pub(crate) fn resolve_url(value: &str, base: Option<&str>) -> Result<String, ExtractionError> {
    if let Ok(url) = url::Url::parse(value) {
        return Ok(url.into());
    }
    let base = base.ok_or_else(|| {
        ExtractionError::new(
            ErrorCode::InvalidBaseUrl,
            "projection",
            "Resolving a relative URL requires explicit effective base metadata.",
        )
    })?;
    url::Url::parse(base)
        .and_then(|base| base.join(value))
        .map(String::from)
        .map_err(|_| {
            ExtractionError::new(
                ErrorCode::InvalidBaseUrl,
                "projection",
                "The requested URL could not be resolved.",
            )
        })
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
