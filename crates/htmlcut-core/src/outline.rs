// SPDX-License-Identifier: MPL-2.0
//! Bounded semantic outlines and their single canonical text encoder.
use crate::dom::ElementRef;
use crate::{
    Document, Error, ErrorCode,
    eval::{Scope, candidates},
    limits::ExecutionContext,
    select::Selector,
};
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    io::Write,
};
/// Reader-facing outline options.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct OutlineOptions {
    /// An optional selector; outlines its first match and reports the complete count.
    pub select: Option<String>,
    /// Last expanded element level, starting at zero.
    pub depth: usize,
    /// Maximum printed lines before the omitted-line summary.
    pub max_lines: usize,
    /// Includes collapsed, clipped attribute values.
    pub attrs: bool,
}
impl Default for OutlineOptions {
    fn default() -> Self {
        Self {
            select: None,
            depth: 6,
            max_lines: 80,
            attrs: false,
        }
    }
}
impl OutlineOptions {
    /// Validates selector admission before a host acquires HTML bytes.
    pub fn validate(&self, context: &crate::ExecutionContext) -> Result<(), Error> {
        context.check()?;
        self.compile_selector()?;
        context.check()
    }
    fn compile_selector(&self) -> Result<Option<Selector>, Error> {
        self.select
            .as_deref()
            .map(|text| {
                Selector::compile(text, "").map_err(|mut error| {
                    error.path = None;
                    error
                })
            })
            .transpose()
    }
}
#[derive(Hash, Eq, PartialEq)]
struct Signature {
    tag: String,
    classes: Vec<String>,
    children: Vec<String>,
}

/// Bounded semantic outline; no frontend must parse canonical text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutlineReport {
    /// Selected CSS, when requested.
    pub selection: Option<String>,
    /// Complete match count, independently of the line allowance.
    pub selection_count: Option<usize>,
    /// Whether the selector header consumed an emitted line.
    pub header_emitted: bool,
    /// Ordered grouped element facts.
    pub rows: Vec<OutlineRow>,
    /// Omitted element rows plus any omitted selector header.
    pub omitted_lines: usize,
}
/// A bounded grouped element row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutlineRow {
    /// Element expansion depth.
    pub depth: usize,
    /// Local tag name.
    pub tag: String,
    /// Namespace URI.
    pub namespace: String,
    /// Representative ID, omitted for grouped elements.
    pub id: Option<String>,
    /// At most four representative classes.
    pub classes: Vec<String>,
    /// Additional class count.
    pub omitted_classes: usize,
    /// At most six representative attributes.
    pub attributes: Vec<OutlineAttribute>,
    /// Additional attribute count.
    pub omitted_attributes: usize,
    /// Normalized clipped own-text preview.
    pub preview: String,
    /// Number of sibling elements represented.
    pub grouped_count: usize,
    /// Descendants collapsed at the depth/foreign/template boundary.
    pub collapsed_descendants: usize,
}
/// One representative attribute, with an optional clipped value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutlineAttribute {
    /// Qualified name.
    pub name: String,
    /// Present only when attribute previews were requested.
    pub value: Option<String>,
}

struct TextOutput<'a> {
    bytes: Vec<u8>,
    maximum: usize,
    clock: &'a ExecutionContext,
    error: Option<Error>,
}
impl<'a> TextOutput<'a> {
    fn new(maximum: usize, clock: &'a ExecutionContext) -> Self {
        Self {
            bytes: Vec::new(),
            maximum,
            clock,
            error: None,
        }
    }
    fn refusal(&mut self) -> Error {
        self.error
            .take()
            .unwrap_or_else(|| Error::new(ErrorCode::Internal, "Outline encoding failed."))
    }
    fn piece(&mut self, text: &str) -> Result<(), Error> {
        self.write_all(text.as_bytes()).map_err(|_| self.refusal())
    }
    fn json(&mut self, text: &str) -> Result<(), Error> {
        serde_json::to_writer(&mut *self, text).map_err(|_| self.refusal())
    }
    fn identifier(&mut self, text: &str) -> Result<(), Error> {
        cssparser::serialize_identifier(text, self).map_err(|_| self.refusal())
    }
    fn text(self) -> String {
        String::from_utf8(self.bytes).expect("canonical text encoders emit UTF-8")
    }
}
impl Write for TextOutput<'_> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if let Err(error) = self.clock.tick() {
            self.error = Some(error);
            return Err(std::io::Error::other("outline stopped"));
        }
        if bytes.len() > self.maximum.saturating_sub(self.bytes.len()) {
            self.error = Some(Error::limit("output_bytes", self.maximum));
            return Err(std::io::Error::other("outline byte allowance exceeded"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
impl std::fmt::Write for TextOutput<'_> {
    fn write_str(&mut self, text: &str) -> std::fmt::Result {
        self.write_all(text.as_bytes()).map_err(|_| std::fmt::Error)
    }
}
struct AttributeView<'a> {
    prefix: Option<&'a str>,
    name: &'a str,
    value: Option<Cow<'a, str>>,
}
struct RowView<'a> {
    depth: usize,
    tag: &'a str,
    namespace: &'a str,
    id: Option<&'a str>,
    classes: Vec<&'a str>,
    omitted_classes: usize,
    attributes: Vec<AttributeView<'a>>,
    omitted_attributes: usize,
    preview: Cow<'a, str>,
    grouped_count: usize,
    collapsed_descendants: usize,
}
impl<'a> RowView<'a> {
    fn element(
        element: ElementRef<'a>,
        depth: usize,
        count: usize,
        attrs: bool,
        inside: usize,
        clock: &ExecutionContext,
    ) -> Result<Self, Error> {
        let mut classes = Vec::new();
        let mut omitted_classes = 0;
        for class in element
            .value()
            .attr("class")
            .unwrap_or_default()
            .split_ascii_whitespace()
        {
            clock.tick()?;
            if classes.len() < 4 {
                classes.push(class);
            } else {
                omitted_classes += 1;
            }
        }
        let mut attributes = Vec::new();
        let mut omitted_attributes = 0;
        for (key, value) in &element.value().attrs {
            clock.tick()?;
            if key.prefix.is_none() && matches!(&*key.local, "id" | "class") {
                continue;
            }
            if attributes.len() == 6 {
                omitted_attributes += 1;
                continue;
            }
            attributes.push(AttributeView {
                prefix: key.prefix.as_deref(),
                name: &key.local,
                value: if attrs {
                    Some(Cow::Owned(preview(value, 40, clock)?))
                } else {
                    None
                },
            });
        }
        let own = if matches!(element.value().name(), "script" | "style") {
            String::new()
        } else {
            preview_parts(
                element.children().filter_map(|node| match node.value() {
                    crate::dom::Node::Text(text) => Some(text.as_str()),
                    _ => None,
                }),
                60,
                clock,
            )?
        };
        Ok(Self {
            depth,
            tag: element.value().name(),
            namespace: &element.value().name.ns,
            id: if count == 1 {
                element.value().id().filter(|id| !id.is_empty())
            } else {
                None
            },
            classes,
            omitted_classes,
            attributes,
            omitted_attributes,
            preview: Cow::Owned(own),
            grouped_count: count,
            collapsed_descendants: inside,
        })
    }
    fn row(row: &'a OutlineRow) -> Self {
        Self {
            depth: row.depth,
            tag: &row.tag,
            namespace: &row.namespace,
            id: row.id.as_deref(),
            classes: row.classes.iter().map(String::as_str).collect(),
            omitted_classes: row.omitted_classes,
            attributes: row
                .attributes
                .iter()
                .map(|attribute| AttributeView {
                    prefix: None,
                    name: &attribute.name,
                    value: attribute.value.as_deref().map(Cow::Borrowed),
                })
                .collect(),
            omitted_attributes: row.omitted_attributes,
            preview: Cow::Borrowed(&row.preview),
            grouped_count: row.grouped_count,
            collapsed_descendants: row.collapsed_descendants,
        }
    }
    fn write(&self, output: &mut TextOutput<'_>) -> Result<(), Error> {
        for _ in 0..self.depth {
            output.piece("  ")?;
        }
        output.identifier(self.tag)?;
        if let Some(id) = self.id {
            output.piece("#")?;
            output.identifier(id)?;
        }
        for class in &self.classes {
            output.piece(".")?;
            output.identifier(class)?;
        }
        if self.omitted_classes > 0 {
            output.piece(&format!(".+{}", self.omitted_classes))?;
        }
        if !self.attributes.is_empty() || self.omitted_attributes > 0 {
            output.piece(" [")?;
            for (index, attribute) in self.attributes.iter().enumerate() {
                if index > 0 {
                    output.piece(" ")?;
                }
                if let Some(prefix) = attribute.prefix {
                    output.piece(prefix)?;
                    output.piece(":")?;
                }
                output.piece(attribute.name)?;
                if let Some(value) = &attribute.value {
                    output.piece("=")?;
                    output.json(value)?;
                }
            }
            if self.omitted_attributes > 0 {
                if !self.attributes.is_empty() {
                    output.piece(" ")?;
                }
                output.piece("…")?;
            }
            output.piece("]")?;
        }
        if self.grouped_count > 1 {
            output.piece(&format!(" ×{}", self.grouped_count))?;
        }
        if !self.preview.is_empty() {
            output.piece(" ")?;
            output.json(&self.preview)?;
        }
        if self.collapsed_descendants > 0 {
            output.piece(&format!(" (+{} inside)", self.collapsed_descendants))?;
        }
        output.piece("\n")
    }
    fn owned(self) -> OutlineRow {
        OutlineRow {
            depth: self.depth,
            tag: self.tag.into(),
            namespace: self.namespace.into(),
            id: self.id.map(str::to_owned),
            classes: self.classes.into_iter().map(str::to_owned).collect(),
            omitted_classes: self.omitted_classes,
            attributes: self
                .attributes
                .into_iter()
                .map(|attribute| OutlineAttribute {
                    name: attribute
                        .prefix
                        .map(|prefix| format!("{prefix}:{}", attribute.name))
                        .unwrap_or_else(|| attribute.name.into()),
                    value: attribute.value.map(Cow::into_owned),
                })
                .collect(),
            omitted_attributes: self.omitted_attributes,
            preview: self.preview.into_owned(),
            grouped_count: self.grouped_count,
            collapsed_descendants: self.collapsed_descendants,
        }
    }
}
fn header(output: &mut TextOutput<'_>, count: usize, selection: &str) -> Result<(), Error> {
    output.piece(&format!(
        "{count} match{} for ",
        if count == 1 { "" } else { "es" }
    ))?;
    if selection.chars().any(char::is_control) {
        output.json(selection)?;
    } else {
        output.piece(selection)?;
    }
    output.piece("\n")
}
fn summary(output: &mut TextOutput<'_>, omitted: usize) -> Result<(), Error> {
    if omitted > 0 {
        output.piece(&format!("… {omitted} more lines; narrow the selection, reduce depth or increase the line allowance\n"))?;
    }
    Ok(())
}
impl OutlineReport {
    /// Renders canonical text under the current operation's byte and stop policy.
    pub fn render(
        &self,
        maximum: usize,
        context: &crate::ExecutionContext,
    ) -> Result<String, Error> {
        context.check()?;
        let clock = (context).clone();
        let mut output = TextOutput::new(maximum, &clock);
        if self.header_emitted {
            header(
                &mut output,
                self.selection_count.unwrap_or(0),
                self.selection.as_deref().unwrap_or_default(),
            )?;
        }
        for row in &self.rows {
            RowView::row(row).write(&mut output)?;
        }
        summary(&mut output, self.omitted_lines)?;
        context.check()?;
        Ok(output.text())
    }
}
impl Document {
    /// Builds bounded semantic outline facts without retaining hidden DOM strings.
    pub fn outline(
        &self,
        options: &OutlineOptions,
        limits: &crate::ExecutionPolicy,
        context: &crate::ExecutionContext,
    ) -> Result<OutlineReport, Error> {
        let clock = (context).clone();
        clock.check()?;
        let scope = Scope::one(self.html.root_element());
        let mut output = TextOutput::new(limits.max_output_bytes, &clock);
        let mut lines = 0;
        let mut caches = Default::default();
        let mut report = OutlineReport {
            selection: None,
            selection_count: None,
            header_emitted: false,
            rows: Vec::new(),
            omitted_lines: 0,
        };
        let root = if let Some(text) = &options.select {
            let selector = options.compile_selector()?.expect("selection supplied");
            let matches = candidates(Some(&selector), &scope, &clock, &mut caches)?;
            // Selector admission bounds this metadata to the selector grammar.
            // Only an emitted header is charged to canonical output bytes.
            if options.max_lines > 0 {
                header(&mut output, matches.len(), text)?;
                report.header_emitted = true;
            }
            report.selection = Some(text.clone());
            report.selection_count = Some(matches.len());
            lines = 1;
            matches.first().copied()
        } else {
            Some(scope.anchor)
        };
        let mut pending = root
            .map(|root| vec![(root, 0usize, 1usize)])
            .unwrap_or_default();
        while let Some((element, level, count)) = pending.pop() {
            clock.tick()?;
            let children = element_children(element, &clock)?;
            let stopped = level >= options.depth
                || matches!(element.value().name(), "svg" | "math" | "template");
            if lines < options.max_lines {
                let mut inside = 0;
                if stopped {
                    for node in element.descendants() {
                        clock.tick()?;
                        if node.id() != element.id() && node.value().is_element() {
                            inside += 1;
                        }
                    }
                }
                let row = RowView::element(element, level, count, options.attrs, inside, &clock)?;
                row.write(&mut output)?; // The same encoder admits before source strings are copied.
                report.rows.push(row.owned());
            }
            lines += 1;
            if stopped {
                continue;
            }
            let mut indexes = HashMap::<Signature, usize>::new();
            let mut groups = Vec::<(ElementRef<'_>, usize)>::new();
            for child in children {
                clock.tick()?;
                let mut seen = HashSet::new();
                let mut tags = Vec::new();
                for descendant in element_children(child, &clock)? {
                    clock.tick()?;
                    let tag = descendant.value().name();
                    if seen.insert(tag) {
                        tags.push(tag.into());
                    }
                }
                let signature = Signature {
                    tag: child.value().name().into(),
                    classes: classes(child, &clock)?,
                    children: tags,
                };
                if let Some(&index) = indexes.get(&signature) {
                    groups[index].1 += 1;
                } else {
                    indexes.insert(signature, groups.len());
                    groups.push((child, 1));
                }
            }
            pending.extend(
                groups
                    .into_iter()
                    .rev()
                    .map(|(element, count)| (element, level + 1, count)),
            );
        }
        report.omitted_lines = lines.saturating_sub(options.max_lines);
        summary(&mut output, report.omitted_lines)?;
        clock.check()?;
        Ok(report)
    }
}
fn element_children<'a>(
    element: ElementRef<'a>,
    clock: &ExecutionContext,
) -> Result<Vec<ElementRef<'a>>, Error> {
    let mut children = Vec::new();
    for child in element.children() {
        clock.tick()?;
        if let Some(element) = ElementRef::wrap(child) {
            children.push(element);
        }
    }
    Ok(children)
}

fn classes(element: ElementRef<'_>, clock: &ExecutionContext) -> Result<Vec<String>, Error> {
    let mut classes = Vec::new();
    for class in element
        .value()
        .attr("class")
        .unwrap_or_default()
        .split_ascii_whitespace()
    {
        clock.tick()?;
        classes.push(class.to_owned());
    }
    Ok(classes)
}

fn preview(value: &str, maximum: usize, clock: &ExecutionContext) -> Result<String, Error> {
    preview_parts(std::iter::once(value), maximum, clock)
}
fn preview_parts<'a>(
    parts: impl Iterator<Item = &'a str>,
    maximum: usize,
    clock: &ExecutionContext,
) -> Result<String, Error> {
    let mut result = String::new();
    let mut space = false;
    let mut emitted = 0;
    for part in parts {
        for c in part.chars() {
            clock.tick()?;
            if c.is_whitespace() {
                space = emitted > 0;
                continue;
            }
            if space {
                if emitted == maximum {
                    result.push('…');
                    return Ok(result);
                }
                result.push(' ');
                emitted += 1;
                space = false;
            }
            if emitted == maximum {
                result.push('…');
                return Ok(result);
            }
            result.push(c);
            emitted += 1;
        }
    }
    Ok(result)
}
#[cfg(test)]
fn identifier(value: &str, maximum: usize, clock: &ExecutionContext) -> Result<String, Error> {
    let mut output = TextOutput::new(maximum, clock);
    output.identifier(value)?;
    Ok(output.text())
}
#[cfg(test)]
fn append(output: &mut String, line: &str, maximum: usize) -> Result<(), Error> {
    if line.len() + 1 > maximum.saturating_sub(output.len()) {
        return Err(Error::limit("output_bytes", maximum));
    }
    let clock = ExecutionContext::new(crate::Limits::default().time_limit).unwrap();
    let mut sink = TextOutput::new(maximum, &clock);
    sink.piece(output)?;
    sink.piece(line)?;
    sink.piece("\n")?;
    *output = sink.text();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Limits, Source};
    #[test]
    fn description_refusals_report_the_global_output_policy_consistently() {
        let limits = Limits {
            max_output_bytes: 20,
            ..Limits::default()
        };
        let document = Document::parse(
            Source::new(b"<main id=identifier-longer-than-the-remaining-line></main>"),
            &limits.preparation(),
            &crate::ExecutionContext::new(limits.time_limit).unwrap(),
        )
        .unwrap();
        let options = OutlineOptions {
            select: Some("main".into()),
            ..OutlineOptions::default()
        };
        let error = document
            .outline(
                &options,
                &limits.execution(),
                &crate::ExecutionContext::new((limits).time_limit).unwrap(),
            )
            .and_then(|report| {
                report.render(
                    limits.max_output_bytes,
                    &crate::ExecutionContext::new((limits).time_limit).unwrap(),
                )
            })
            .unwrap_err();
        assert_eq!(error.error, ErrorCode::LimitExceeded);
        assert_eq!(error.limit.as_deref(), Some("output_bytes"));
        assert_eq!(error.bound, Some(20.into()));
        assert_eq!(error.message, "The output_bytes limit of 20 was exceeded.");
    }

    #[test]
    fn qualified_attribute_names_preserve_prefixes_at_exact_outline_byte_boundaries() {
        let html =
            b"<svg xmlns:xlink='http://www.w3.org/1999/xlink' xlink:href=target xml:lang=en></svg>";
        let expected = "1 match for svg\nsvg [xmlns:xlink xlink:href xml:lang]\n";
        let options = OutlineOptions {
            select: Some("svg".into()),
            ..OutlineOptions::default()
        };
        for maximum in [expected.len() - 1, expected.len(), expected.len() + 1] {
            let limits = Limits {
                max_output_bytes: maximum,
                ..Limits::default()
            };
            let document = Document::parse(
                Source::new(html),
                &limits.preparation(),
                &crate::ExecutionContext::new(limits.time_limit).unwrap(),
            )
            .unwrap();
            if maximum >= expected.len() {
                assert_eq!(
                    document
                        .outline(
                            &options,
                            &limits.execution(),
                            &crate::ExecutionContext::new((limits).time_limit).unwrap()
                        )
                        .and_then(|report| report.render(
                            limits.max_output_bytes,
                            &crate::ExecutionContext::new((limits).time_limit).unwrap()
                        ))
                        .unwrap(),
                    expected
                );
            } else {
                let error = document
                    .outline(
                        &options,
                        &limits.execution(),
                        &crate::ExecutionContext::new((limits).time_limit).unwrap(),
                    )
                    .and_then(|report| {
                        report.render(
                            limits.max_output_bytes,
                            &crate::ExecutionContext::new((limits).time_limit).unwrap(),
                        )
                    })
                    .unwrap_err();
                assert_eq!(error.limit.as_deref(), Some("output_bytes"));
                assert_eq!(error.bound, Some(maximum.into()));
                assert_eq!(
                    error.message,
                    format!("The output_bytes limit of {maximum} was exceeded.")
                );
            }
        }
    }

    #[test]
    fn description_admission_refuses_an_oversized_source_attribute_name() {
        let html = format!("<main {}='value'></main>", "attribute".repeat(8192));
        let limits = Limits::default();
        let document = Document::parse(
            Source::new(html.as_bytes()),
            &limits.preparation(),
            &crate::ExecutionContext::new(limits.time_limit).unwrap(),
        )
        .unwrap();
        let element = document
            .html
            .select(&Selector::parse("main").unwrap())
            .next()
            .unwrap();
        let clock = ExecutionContext::new(limits.time_limit).unwrap();
        let row = RowView::element(element, 0, 1, false, 0, &clock).unwrap();
        let error = row.write(&mut TextOutput::new(8, &clock)).unwrap_err();
        assert_eq!(error.limit.as_deref(), Some("output_bytes"));
        assert_eq!(error.bound, Some(8.into()));
    }

    #[test]
    fn previews_stream_normalized_parts_without_allocating_source_values() {
        let limits = Limits::default();
        let clock = ExecutionContext::new(limits.time_limit).unwrap();
        assert_eq!(
            preview_parts([" a  ", " b "].into_iter(), 3, &clock).unwrap(),
            "a b"
        );
        assert_eq!(
            preview_parts(["a  ", " b c"].into_iter(), 3, &clock).unwrap(),
            "a b…"
        );
        assert_eq!(
            preview(
                &format!("{}x{}", " ".repeat(1_000_000), " ".repeat(1_000_000)),
                1,
                &clock
            )
            .unwrap(),
            "x"
        );
        let error = identifier(&"x".repeat(1_000_000), 8, &clock).unwrap_err();
        assert_eq!(error.limit.as_deref(), Some("output_bytes"));
        assert_eq!(error.bound, Some(8.into()));
    }
    #[test]
    fn append_admits_complete_utf8_lines_and_preserves_buffer_on_refusal() {
        let initial = "old\n";
        let line = "é雪";
        let expected = "old\né雪\n";
        for maximum in [expected.len(), expected.len() + 1] {
            let mut output = initial.to_owned();
            append(&mut output, line, maximum).unwrap();
            assert_eq!(output, expected);
        }
        for maximum in [expected.len() - 1, expected.len() - 2] {
            let mut output = initial.to_owned();
            let error = append(&mut output, line, maximum).unwrap_err();
            assert_eq!(error.error, ErrorCode::LimitExceeded);
            assert_eq!(error.limit.as_deref(), Some("output_bytes"));
            assert_eq!(error.bound, Some(maximum.into()));
            assert_eq!(output, initial);
        }
    }
    #[test]
    fn hidden_selection_header_is_counted_even_without_a_root_match() {
        let document = Document::parse(
            Source::new(b"<p>x</p>"),
            &Limits::default().preparation(),
            &crate::ExecutionContext::new(Limits::default().time_limit).unwrap(),
        )
        .unwrap();
        let mut options = OutlineOptions {
            select: Some("main".into()),
            max_lines: 0,
            ..OutlineOptions::default()
        };
        assert_eq!(
            document
                .outline(
                    &options,
                    &crate::Limits::default().execution(),
                    &crate::ExecutionContext::new((crate::Limits::default()).time_limit).unwrap()
                )
                .and_then(|report| report.render(
                    crate::Limits::default().max_output_bytes,
                    &crate::ExecutionContext::new((crate::Limits::default()).time_limit).unwrap()
                ))
                .unwrap(),
            "… 1 more lines; narrow the selection, reduce depth or increase the line allowance\n"
        );
        options.max_lines = 1;
        assert_eq!(
            document
                .outline(
                    &options,
                    &crate::Limits::default().execution(),
                    &crate::ExecutionContext::new((crate::Limits::default()).time_limit).unwrap()
                )
                .and_then(|report| report.render(
                    crate::Limits::default().max_output_bytes,
                    &crate::ExecutionContext::new((crate::Limits::default()).time_limit).unwrap()
                ))
                .unwrap(),
            "0 matches for main\n"
        );
        options.select = None;
        options.depth = 0;
        options.max_lines = 0;
        assert_eq!(
            document
                .outline(
                    &options,
                    &crate::Limits::default().execution(),
                    &crate::ExecutionContext::new((crate::Limits::default()).time_limit).unwrap()
                )
                .and_then(|report| report.render(
                    crate::Limits::default().max_output_bytes,
                    &crate::ExecutionContext::new((crate::Limits::default()).time_limit).unwrap()
                ))
                .unwrap(),
            "… 1 more lines; narrow the selection, reduce depth or increase the line allowance\n"
        );
        options.max_lines = 1;
        assert_eq!(
            document
                .outline(
                    &options,
                    &crate::Limits::default().execution(),
                    &crate::ExecutionContext::new((crate::Limits::default()).time_limit).unwrap()
                )
                .and_then(|report| report.render(
                    crate::Limits::default().max_output_bytes,
                    &crate::ExecutionContext::new((crate::Limits::default()).time_limit).unwrap()
                ))
                .unwrap(),
            "html (+3 inside)\n"
        );
    }
    #[test]
    fn shapes_order_classes_source_attributes_and_clipping_are_preserved() {
        let html = "<main><p id=a class='z A b c d' z='1' a='2' b='3' c='4' d='5' e='6' f='7'>quoted \"text\"</p><p class='z A b c d'><b>shape</b></p><p class='z A b c d'>same shape</p></main>";
        let document = Document::parse(
            Source::new(html.as_bytes()),
            &Limits::default().preparation(),
            &crate::ExecutionContext::new(Limits::default().time_limit).unwrap(),
        )
        .unwrap();
        let options = OutlineOptions {
            select: Some("main".into()),
            attrs: true,
            ..OutlineOptions::default()
        };
        assert_eq!(
            document
                .outline(
                    &options,
                    &crate::Limits::default().execution(),
                    &crate::ExecutionContext::new((crate::Limits::default()).time_limit).unwrap()
                )
                .and_then(|report| report.render(
                    crate::Limits::default().max_output_bytes,
                    &crate::ExecutionContext::new((crate::Limits::default()).time_limit).unwrap()
                ))
                .unwrap(),
            "1 match for main\nmain\n  p.z.A.b.c.+1 [z=\"1\" a=\"2\" b=\"3\" c=\"4\" d=\"5\" e=\"6\" …] ×2 \"quoted \\\"text\\\"\"\n  p.z.A.b.c.+1\n    b \"shape\"\n"
        );
    }
    #[test]
    fn foreign_roots_stop_and_output_limits_refuse_instead_of_truncating_bytes() {
        let html = "<svg id=x><g><path></path></g></svg>";
        let limits = Limits {
            max_output_bytes: 1,
            ..Limits::default()
        };
        let document = Document::parse(
            Source::new(html.as_bytes()),
            &limits.preparation(),
            &crate::ExecutionContext::new(limits.time_limit).unwrap(),
        )
        .unwrap();
        let options = OutlineOptions {
            select: Some("svg".into()),
            ..OutlineOptions::default()
        };
        assert_eq!(
            document
                .outline(
                    &options,
                    &limits.execution(),
                    &crate::ExecutionContext::new((limits).time_limit).unwrap()
                )
                .and_then(|report| report.render(
                    limits.max_output_bytes,
                    &crate::ExecutionContext::new((limits).time_limit).unwrap()
                ))
                .unwrap_err()
                .limit
                .as_deref(),
            Some("output_bytes")
        );
        let limits = Limits::default();
        let document = Document::parse(
            Source::new(html.as_bytes()),
            &Limits::default().preparation(),
            &crate::ExecutionContext::new(Limits::default().time_limit).unwrap(),
        )
        .unwrap();
        assert_eq!(
            document
                .outline(
                    &options,
                    &limits.execution(),
                    &crate::ExecutionContext::new((limits).time_limit).unwrap()
                )
                .and_then(|report| report.render(
                    limits.max_output_bytes,
                    &crate::ExecutionContext::new((limits).time_limit).unwrap()
                ))
                .unwrap(),
            "1 match for svg\nsvg#x (+2 inside)\n"
        );
    }
    #[test]
    fn control_characters_cannot_add_outline_lines() {
        let document = Document::parse(
            Source::new(b"<main><p>x</p></main>"),
            &crate::Limits::default().preparation(),
            &crate::ExecutionContext::new(crate::Limits::default().time_limit).unwrap(),
        )
        .unwrap();
        let options = OutlineOptions {
            select: Some("main\np".into()),
            max_lines: 1,
            ..OutlineOptions::default()
        };
        assert_eq!(
            document
                .outline(
                    &options,
                    &crate::Limits::default().execution(),
                    &crate::ExecutionContext::new((crate::Limits::default()).time_limit).unwrap()
                )
                .and_then(|report| report.render(
                    crate::Limits::default().max_output_bytes,
                    &crate::ExecutionContext::new((crate::Limits::default()).time_limit).unwrap()
                ))
                .unwrap(),
            "1 match for \"main\\np\"\n… 1 more lines; narrow the selection, reduce depth or increase the line allowance\n"
        );
    }
    #[test]
    fn hidden_selector_initial_admission_matches_later_rendering_at_exact_bounds() {
        let limits = Limits::default();
        let context = ExecutionContext::new(None).unwrap();
        let document = Document::parse(
            Source::new(b"<p>value</p>"),
            &limits.preparation(),
            &context,
        )
        .unwrap();
        let options = OutlineOptions {
            select: Some(format!("p{}", " ".repeat(200))),
            max_lines: 0,
            ..OutlineOptions::default()
        };
        let report = document
            .outline(&options, &limits.execution(), &context)
            .unwrap();
        let expected =
            "… 2 more lines; narrow the selection, reduce depth or increase the line allowance\n";
        assert_eq!(report.render(expected.len(), &context).unwrap(), expected);
        assert_eq!(report.selection.as_deref(), options.select.as_deref());
        assert!(!report.header_emitted);
        assert!(report.rows.is_empty());
        for maximum in [expected.len() - 1, expected.len(), expected.len() + 1, 100] {
            let mut policy = limits.execution();
            policy.max_output_bytes = maximum;
            let initial = document
                .outline(&options, &policy, &context)
                .and_then(|report| report.render(maximum, &context));
            let saved = report.render(maximum, &context);
            match (initial, saved) {
                (Ok(initial), Ok(saved)) => assert_eq!(initial, saved),
                (Err(initial), Err(saved)) => {
                    assert_eq!(initial.error, saved.error);
                    assert_eq!(initial.limit, saved.limit);
                    assert_eq!(initial.bound, saved.bound);
                }
                _ => panic!("initial/render admission differs for {maximum}"),
            }
        }
    }
    #[test]
    fn hidden_grouping_class_loops_check_each_iteration_cadence_and_cancel() {
        use std::sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        };
        let limits = Limits::default();
        let prepare = ExecutionContext::new(None).unwrap();
        let tokens = 200_000;
        let source = format!("<p class='{}'>value</p>", "a ".repeat(tokens));
        let document = Document::parse(
            Source::new(source.as_bytes()),
            &limits.preparation(),
            &prepare,
        )
        .unwrap();
        let options = OutlineOptions {
            max_lines: 0,
            ..OutlineOptions::default()
        };
        let checks = Arc::new(AtomicUsize::new(0));
        let observed = checks.clone();
        let context = ExecutionContext::with_checkpoint(move || {
            observed.fetch_add(1, Ordering::Relaxed);
            Ok(())
        });
        document
            .outline(&options, &limits.execution(), &context)
            .unwrap();
        assert!(checks.load(Ordering::Relaxed) >= tokens / 1024);
        let checks = Arc::new(AtomicUsize::new(0));
        let observed = checks.clone();
        let context = ExecutionContext::with_checkpoint(move || {
            if observed.fetch_add(1, Ordering::Relaxed) >= 3 {
                Err(Error::new(
                    ErrorCode::Cancelled,
                    "controlled cancellation during grouping",
                ))
            } else {
                Ok(())
            }
        });
        let error = document
            .outline(&options, &limits.execution(), &context)
            .unwrap_err();
        assert_eq!(error.error, ErrorCode::Cancelled);
        assert!(error.limit.is_none());
        assert!(error.bound.is_none());
        assert_eq!(checks.load(Ordering::Relaxed), 4);
    }
}
