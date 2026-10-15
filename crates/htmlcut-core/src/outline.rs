// SPDX-License-Identifier: MPL-2.0
//! Bounded structural outlines, with deterministic sibling grouping.
use crate::dom::ElementRef;
use crate::{
    Document, Error, ErrorCode,
    eval::{Scope, candidates},
    limits::Clock,
    select::Selector,
};
use std::collections::{HashMap, HashSet};

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
#[derive(Hash, Eq, PartialEq)]
struct Signature {
    tag: String,
    classes: Vec<String>,
    children: Vec<String>,
}

impl Document {
    /// Prints a bounded deterministic skeleton with a fresh time limit.
    pub fn outline(&self, options: &OutlineOptions) -> Result<String, Error> {
        let clock = Clock::new(&self.limits);
        clock.check()?;
        let scope = Scope::one(self.html.root_element());
        let mut output = String::new();
        let mut lines = 0;
        let mut caches = Default::default();
        let root = if let Some(text) = &options.select {
            let selector = Selector::compile(text, "").map_err(|mut error| {
                error.path = None;
                error
            })?;
            let matches = candidates(Some(&selector), &scope, &clock, &mut caches)?;
            let displayed = if text.chars().any(char::is_control) {
                serde_json::to_string(text).unwrap()
            } else {
                text.clone()
            };
            lines = 1;
            if options.max_lines > 0 {
                append(
                    &mut output,
                    &format!(
                        "{} match{} for {displayed}",
                        matches.len(),
                        if matches.len() == 1 { "" } else { "es" }
                    ),
                    self.limits.max_output_bytes,
                )?;
            }
            matches.first().copied()
        } else {
            Some(scope.anchor)
        };
        let mut pending = root
            .map(|root| vec![(root, 0usize, 1usize)])
            .unwrap_or_default();
        while let Some((element, level, count)) = pending.pop() {
            clock.tick()?;
            let children = element_children(element);
            let stopped = level >= options.depth
                || matches!(element.value().name(), "svg" | "math" | "template");
            if lines < options.max_lines {
                let mut line = "  ".repeat(level);
                line.push_str(
                    &describe(
                        element,
                        count,
                        options.attrs,
                        &clock,
                        self.limits.max_output_bytes.saturating_sub(output.len()),
                    )
                    .map_err(|mut error| {
                        if error.limit.as_deref() == Some("output_bytes") {
                            error.bound = Some(self.limits.max_output_bytes.into());
                        }
                        error
                    })?,
                );
                if stopped {
                    let mut inside = 0;
                    for node in element.descendants() {
                        clock.tick()?;
                        if node.id() != element.id() && node.value().is_element() {
                            inside += 1;
                        }
                    }
                    if inside > 0 {
                        line.push_str(&format!(" (+{inside} inside)"));
                    }
                }
                append(&mut output, &line, self.limits.max_output_bytes)?;
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
                for descendant in element_children(child) {
                    clock.tick()?;
                    let tag = descendant.value().name();
                    if seen.insert(tag) {
                        tags.push(tag.into());
                    }
                }
                let signature = Signature {
                    tag: child.value().name().into(),
                    classes: classes(child),
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
        if lines > options.max_lines {
            let summary = format!(
                "… {} more lines; narrow --select, lower --depth or raise --max-lines",
                lines - options.max_lines
            );
            append(&mut output, &summary, self.limits.max_output_bytes)?;
        }
        clock.check()?;
        Ok(output)
    }
}
fn append(output: &mut String, line: &str, maximum: usize) -> Result<(), Error> {
    if line.len() + 1 > maximum.saturating_sub(output.len()) {
        return Err(Error::limit("output_bytes", maximum));
    }
    output.push_str(line);
    output.push('\n');
    Ok(())
}
fn element_children(element: ElementRef<'_>) -> Vec<ElementRef<'_>> {
    element.children().filter_map(ElementRef::wrap).collect()
}
fn classes(element: ElementRef<'_>) -> Vec<String> {
    element
        .value()
        .attr("class")
        .unwrap_or_default()
        .split_ascii_whitespace()
        .map(String::from)
        .collect()
}
fn identifier(value: &str, maximum: usize, clock: &Clock) -> Result<String, Error> {
    struct Sink<'a> {
        value: String,
        maximum: usize,
        clock: &'a Clock,
        error: Option<Error>,
    }
    impl std::fmt::Write for Sink<'_> {
        fn write_str(&mut self, value: &str) -> std::fmt::Result {
            if let Err(error) = self.clock.tick() {
                self.error = Some(error);
                return Err(std::fmt::Error);
            }
            if value.len() > self.maximum.saturating_sub(self.value.len()) {
                self.error = Some(Error::limit("output_bytes", self.maximum));
                return Err(std::fmt::Error);
            }
            self.value.push_str(value);
            Ok(())
        }
    }
    let mut sink = Sink {
        value: String::new(),
        maximum,
        clock,
        error: None,
    };
    cssparser::serialize_identifier(value, &mut sink).map_err(|_| {
        sink.error.unwrap_or_else(|| {
            Error::new(
                ErrorCode::Internal,
                "An outline identifier could not be encoded.",
            )
        })
    })?;
    Ok(sink.value)
}
fn preview(value: &str, maximum: usize, clock: &Clock) -> Result<String, Error> {
    preview_parts(std::iter::once(value), maximum, clock)
}
fn preview_parts<'a>(
    parts: impl Iterator<Item = &'a str>,
    maximum: usize,
    clock: &Clock,
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

fn describe(
    element: ElementRef<'_>,
    count: usize,
    attrs: bool,
    clock: &Clock,
    maximum: usize,
) -> Result<String, Error> {
    let mut line = identifier(element.value().name(), maximum, clock)?;
    if count == 1
        && let Some(id) = element.value().id().filter(|id| !id.is_empty())
    {
        line.push('#');
        line.push_str(&identifier(id, maximum.saturating_sub(line.len()), clock)?);
    }
    let classes = classes(element);
    for class in classes.iter().take(4) {
        line.push('.');
        line.push_str(&identifier(
            class,
            maximum.saturating_sub(line.len()),
            clock,
        )?);
    }
    if classes.len() > 4 {
        line.push_str(&format!(".+{}", classes.len() - 4));
    }
    let mut names = Vec::new();
    for (key, value) in &element.value().attrs {
        clock.tick()?;
        if key.prefix.is_none() && matches!(&*key.local, "id" | "class") {
            continue;
        }
        if names.len() >= 6 {
            if names.len() == 6 {
                names.push("…".into());
            }
            continue;
        }
        let reserved = names
            .iter()
            .map(|name: &String| name.len() + 1)
            .sum::<usize>();
        if key.local.len() + key.prefix.as_ref().map_or(0, |p| p.len() + 1)
            > maximum.saturating_sub(line.len() + reserved)
        {
            return Err(Error::limit("output_bytes", maximum));
        }
        let name = key
            .prefix
            .as_ref()
            .map(|prefix| format!("{prefix}:{}", key.local))
            .unwrap_or_else(|| key.local.to_string());
        names.push(if attrs {
            format!(
                "{name}={}",
                serde_json::to_string(&preview(value, 40, clock)?).unwrap()
            )
        } else {
            name
        });
    }
    if !names.is_empty() {
        line.push_str(&format!(" [{}]", names.join(" ")));
    }
    if count > 1 {
        line.push_str(&format!(" ×{count}"));
    }
    if !matches!(element.value().name(), "script" | "style") {
        let own = preview_parts(
            element.children().filter_map(|node| match node.value() {
                crate::dom::Node::Text(text) => Some(&**text),
                _ => None,
            }),
            60,
            clock,
        )?;
        if !own.is_empty() {
            line.push(' ');
            line.push_str(&serde_json::to_string(&own).unwrap());
        }
    }
    Ok(line)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Limits, Source};
    #[test]
    fn previews_stream_normalized_parts_without_allocating_source_values() {
        let limits = Limits::default();
        let clock = Clock::new(&limits);
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
        let document = Document::parse(Source::new(b"<p>x</p>"), &Limits::default()).unwrap();
        let mut options = OutlineOptions {
            select: Some("main".into()),
            max_lines: 0,
            ..OutlineOptions::default()
        };
        assert_eq!(
            document.outline(&options).unwrap(),
            "… 1 more lines; narrow --select, lower --depth or raise --max-lines\n"
        );
        options.max_lines = 1;
        assert_eq!(document.outline(&options).unwrap(), "0 matches for main\n");
        options.select = None;
        options.depth = 0;
        options.max_lines = 0;
        assert_eq!(
            document.outline(&options).unwrap(),
            "… 1 more lines; narrow --select, lower --depth or raise --max-lines\n"
        );
        options.max_lines = 1;
        assert_eq!(document.outline(&options).unwrap(), "html (+3 inside)\n");
    }
    #[test]
    fn shapes_order_classes_source_attributes_and_clipping_are_preserved() {
        let html = "<main><p id=a class='z A b c d' z='1' a='2' b='3' c='4' d='5' e='6' f='7'>quoted \"text\"</p><p class='z A b c d'><b>shape</b></p><p class='z A b c d'>same shape</p></main>";
        let document = Document::parse(Source::new(html.as_bytes()), &Limits::default()).unwrap();
        let options = OutlineOptions {
            select: Some("main".into()),
            attrs: true,
            ..OutlineOptions::default()
        };
        assert_eq!(
            document.outline(&options).unwrap(),
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
        let document = Document::parse(Source::new(html.as_bytes()), &limits).unwrap();
        let options = OutlineOptions {
            select: Some("svg".into()),
            ..OutlineOptions::default()
        };
        assert_eq!(
            document.outline(&options).unwrap_err().limit.as_deref(),
            Some("output_bytes")
        );
        let document = Document::parse(Source::new(html.as_bytes()), &Limits::default()).unwrap();
        assert_eq!(
            document.outline(&options).unwrap(),
            "1 match for svg\nsvg#x (+2 inside)\n"
        );
    }
    #[test]
    fn control_characters_cannot_add_outline_lines() {
        let document = Document::parse(
            Source::new(b"<main><p>x</p></main>"),
            &crate::Limits::default(),
        )
        .unwrap();
        let options = OutlineOptions {
            select: Some("main\np".into()),
            max_lines: 1,
            ..OutlineOptions::default()
        };
        assert_eq!(
            document.outline(&options).unwrap(),
            "1 match for \"main\\np\"\n… 1 more lines; narrow --select, lower --depth or raise --max-lines\n"
        );
    }
}
