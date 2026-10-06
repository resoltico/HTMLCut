// SPDX-License-Identifier: MPL-2.0
//! Readings with bounded construction and one shared structural-text policy.

use std::collections::HashSet;

use ego_tree::{NodeId, iter::Edge};
use scraper::{ElementRef, Node};
use selectors::work_budget::SelectorWorkBudget;

use crate::{ErrorCode, ExtractionError, Reading};

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
            return Err(ExtractionError::resource(
                "projection",
                "value_bytes",
                self.maximum as u64,
            ));
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
    pub(crate) fn finish(self) -> String {
        self.value
    }
}

pub(crate) fn project(
    root: ElementRef<'_>,
    projection: &Reading,
    excluded: &HashSet<NodeId>,
    base: Option<&str>,
    maximum: usize,
    budget: &SelectorWorkBudget,
) -> Result<String, ExtractionError> {
    let resolve = matches!(projection, Reading::Url(_) | Reading::ResolvedMarkdown);
    match projection {
        Reading::Attribute(name) | Reading::Url(name) => {
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
                return Err(ExtractionError::resource(
                    "projection",
                    "value_bytes",
                    maximum as u64,
                ));
            }
            let value = if resolve {
                resolve_url(value, base, maximum)?
            } else {
                value.to_owned()
            };
            // Untransformed values were bounded above; resolve_url bounds the final URL.
            Ok(value)
        }
        Reading::Text | Reading::Literal => text(
            root,
            excluded,
            matches!(projection, Reading::Text),
            maximum,
            None,
            budget,
        )
        .map(|(text, _)| text),
        Reading::Markdown | Reading::ResolvedMarkdown => {
            markdown::render(root, excluded, resolve, base, maximum, budget)
        }
        Reading::InnerHtml | Reading::OuterHtml => {
            #[cfg(test)]
            record_projection(3);
            let mut writer = HtmlBuffer {
                bytes: Vec::new(),
                maximum,
                limit_exceeded: false,
            };
            root.write_filtered_html(
                &mut writer,
                matches!(projection, Reading::OuterHtml),
                excluded,
                budget,
            )
            .map_err(|_| {
                if writer.limit_exceeded {
                    ExtractionError::resource("serialization", "value_bytes", maximum as u64)
                } else {
                    // This concrete memory writer only fails at its size bound;
                    // the maintained serializer's other error origin is exhausted work.
                    ExtractionError::resource(
                        "serialization",
                        "max_work",
                        budget.configured().into(),
                    )
                }
            })?;
            // The HTML serializer emits UTF-8; the owned buffer cannot be externally corrupted.
            Ok(String::from_utf8(writer.bytes).expect("HTML serializer produces UTF-8"))
        }
    }
}

// Full extraction and abbreviated observations share the same event policy.
// A preview stops only when another emitted scalar proves incompleteness.
pub(crate) fn text(
    root: ElementRef<'_>,
    excluded: &HashSet<NodeId>,
    structural: bool,
    maximum: usize,
    preview_characters: Option<usize>,
    budget: &SelectorWorkBudget,
) -> Result<(String, bool), ExtractionError> {
    #[cfg(test)]
    record_projection(1);
    let mut output = TextOutput {
        value: String::new(),
        maximum,
        preview_characters,
        characters: 0,
        pending: false,
        budget,
    };
    let mut skipped = 0_u32;
    let mut pre = u32::from(structural && context::inherited_pre(root, budget)?);
    for edge in root.traverse() {
        crate::execution::charge(budget, 1)?;
        match edge {
            Edge::Open(node) => {
                if skipped > 0
                    || excluded.contains(&node.id())
                    || (structural && inert(node.value()))
                {
                    skipped += 1;
                    continue;
                }
                if structural && pre == 0 && text_boundary(node.value()) {
                    output.separator();
                }
                if structural && html_pre(node.value()) {
                    pre += 1;
                }
                if let Node::Text(text) = node.value() {
                    let mut examined = 0_usize;
                    for character in text.text.chars() {
                        let size = examined + character.len_utf8();
                        crate::execution::charge(
                            budget,
                            size.div_ceil(64) - examined.div_ceil(64),
                        )?;
                        examined = size;
                        if structural && pre == 0 && character.is_whitespace() {
                            output.separator();
                            continue;
                        }
                        if !output.character(character)? {
                            return Ok((output.value, false));
                        }
                    }
                }
            }
            Edge::Close(node) => {
                if skipped > 0 {
                    skipped -= 1;
                    continue;
                }
                if structural && html_pre(node.value()) {
                    pre -= 1;
                }
                if structural && pre == 0 && text_boundary(node.value()) {
                    output.separator();
                }
            }
        }
    }
    Ok((output.value, true))
}
fn html_pre(node: &Node) -> bool {
    node.as_element()
        .is_some_and(|element| context::html(element) && element.name() == "pre")
}
fn inert(node: &Node) -> bool {
    node.as_element().is_some_and(|element| {
        let namespace: &str = element.name.ns.as_ref();
        (context::html(element) && matches!(element.name(), "script" | "style" | "template"))
            || (namespace == "http://www.w3.org/2000/svg"
                && matches!(element.name(), "script" | "style"))
    })
}
struct TextOutput<'a> {
    value: String,
    maximum: usize,
    preview_characters: Option<usize>,
    characters: usize,
    pending: bool,
    budget: &'a SelectorWorkBudget,
}
impl TextOutput<'_> {
    fn separator(&mut self) {
        self.pending = self
            .value
            .chars()
            .next_back()
            .is_some_and(|c| !c.is_whitespace());
    }
    fn character(&mut self, character: char) -> Result<bool, ExtractionError> {
        // A generated boundary belongs with its following content. A preview
        // never ends with only that invented separator.
        let separator = self.pending && !character.is_whitespace();
        if self
            .preview_characters
            .is_some_and(|maximum| self.characters + 1 + usize::from(separator) > maximum)
            || (self.preview_characters.is_some()
                && character.len_utf8() + usize::from(separator)
                    > self.maximum.saturating_sub(self.value.len()))
        {
            return Ok(false);
        }
        // Parsed pre whitespace already supplies the boundary; never replace its edges.
        if separator {
            self.push(' ')?;
        }
        self.pending = false;
        self.push(character)?;
        Ok(true)
    }
    fn push(&mut self, character: char) -> Result<(), ExtractionError> {
        if character.len_utf8() > self.maximum.saturating_sub(self.value.len()) {
            return Err(ExtractionError::resource(
                "projection",
                "value_bytes",
                self.maximum as u64,
            ));
        }
        let size = self.value.len() + character.len_utf8();
        crate::execution::charge(
            self.budget,
            size.div_ceil(64) - self.value.len().div_ceil(64),
        )?;
        self.value.push(character);
        self.characters += 1;
        Ok(())
    }
}

pub(crate) fn text_boundary(node: &Node) -> bool {
    node.as_element().is_some_and(|element| {
        context::html(element)
            && matches!(
                element.name(),
                "br" | "p"
                    | "details"
                    | "summary"
                    | "address"
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
        return Err(ExtractionError::resource(
            "url",
            "url_input_bytes",
            crate::limits::MAX_URL_INPUT_BYTES as u64,
        ));
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
        return Err(if maximum < crate::limits::MAX_URL_PROCESSING_BYTES {
            ExtractionError::resource("url", "value_bytes", maximum as u64)
        } else {
            ExtractionError::resource(
                "url",
                "url_processing_bytes",
                crate::limits::MAX_URL_PROCESSING_BYTES as u64,
            )
        });
    }
    Ok(parsed.into())
}

struct HtmlBuffer {
    bytes: Vec<u8>,
    maximum: usize,
    limit_exceeded: bool,
}

impl std::io::Write for HtmlBuffer {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > self.maximum.saturating_sub(self.bytes.len()) {
            self.limit_exceeded = true;
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
