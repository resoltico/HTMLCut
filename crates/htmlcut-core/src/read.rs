// SPDX-License-Identifier: MPL-2.0 AND ISC
//! Deterministic structural text and original-tree readings.
use crate::dom::Node;
use crate::{Document, Error, ErrorCode, contract::Reading, eval::Scope, limits::Clock};
use ego_tree::{NodeId, NodeRef, iter::Edge};
use html5ever::serialize::{Serialize, SerializeOpts, Serializer, TraversalScope, serialize};
use std::{collections::HashSet, io};

pub(crate) fn read(
    document: &Document,
    scope: &Scope<'_>,
    reading: &Reading,
    excluded: &HashSet<NodeId>,
    clock: &Clock,
    output: Option<(usize, usize)>,
) -> Result<String, Error> {
    clock.check()?;
    let mut sink = ReadingSink::new(document.limits.max_reading_bytes, output, clock)?;
    let result = match reading {
        Reading::Attr(name) | Reading::Url(name) => {
            let value = attribute(scope.anchor.value(), name).ok_or_else(|| {
                Error::new(
                    ErrorCode::MissingAttribute,
                    format!(
                        "The selected <{}> element has no {name:?} attribute.",
                        scope.anchor.value().name()
                    ),
                )
            })?;
            let result: String = if matches!(reading, Reading::Url(_)) {
                crate::document::resolve(value, document.base_url.as_ref(), document.query_encoding)
                    .map(|u| u.into())
                    .map_err(|cause| {
                        Error::new(
                            ErrorCode::UnresolvableUrl,
                            format!(
                                "URL {:?} cannot be resolved: {}.",
                                value.chars().take(120).collect::<String>(),
                                match cause {
                                    url::ParseError::RelativeUrlWithoutBase =>
                                        "relative URL requires a usable document base URL"
                                            .to_owned(),
                                    url::ParseError::RelativeUrlWithCannotBeABaseBase => "relative URL cannot use an opaque document base; supply a usable base URL".to_owned(),
                                    cause => format!("malformed URL ({cause})"),
                                }
                            ),
                        )
                        .observed(value)
                    })?
            } else {
                sink.push(value)?;
                return Ok(sink.value);
            };
            sink.push(&result)?;
            sink.value
        }
        Reading::TextContent => {
            text_content(scope, excluded, clock, &mut sink)?;
            sink.value
        }
        Reading::Text => {
            text(scope, excluded, clock, &mut sink)?;
            sink.value
        }
        Reading::OuterHtml => {
            outer_html(scope, excluded, clock, &mut sink)?;
            sink.value
        }
    };
    clock.check()?;
    Ok(result)
}
fn attribute<'a>(element: &'a crate::dom::Element, name: &str) -> Option<&'a str> {
    element.attrs.iter().find_map(|(key, value)| {
        let matches = match &key.prefix {
            Some(prefix) => name
                .strip_prefix(&**prefix)
                .is_some_and(|tail| tail.strip_prefix(':') == Some(&*key.local)),
            None => name == &*key.local,
        };
        matches.then_some(value.as_ref())
    })
}
fn text_content(
    scope: &Scope<'_>,
    excluded: &HashSet<NodeId>,
    clock: &Clock,
    result: &mut ReadingSink,
) -> Result<(), Error> {
    for root in scope.roots() {
        let mut skipped = 0;
        for edge in root.traverse() {
            clock.tick()?;
            match edge {
                Edge::Open(node)
                    if skipped > 0
                        || excluded.contains(&node.id())
                        || (node
                            .value()
                            .as_element()
                            .is_some_and(|e| html(e) && e.name() == "template")
                            && !(scope.single() && node.id() == scope.anchor.id())) =>
                {
                    skipped += 1;
                }
                Edge::Open(node) => {
                    if let Node::Text(text) = node.value() {
                        result.push(text)?;
                    }
                }
                Edge::Close(_) if skipped > 0 => skipped -= 1,
                Edge::Close(_) => (),
            }
        }
    }
    Ok(())
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
fn html(element: &crate::dom::Element) -> bool {
    element.name.ns == html5ever::ns!(html)
}
fn preformatted(element: &crate::dom::Element) -> bool {
    html(element)
        && matches!(
            element.name(),
            "pre" | "listing" | "plaintext" | "xmp" | "textarea"
        )
}
// Borrowed DOM spans avoid allocating discarded whitespace or an oversized source scalar.
struct Lines<'d, 's, 'c> {
    cells: Vec<Vec<&'d str>>,
    verbatim: bool,
    has_line: bool,
    sink: &'s mut ReadingSink<'c>,
}
impl<'d, 's, 'c> Lines<'d, 's, 'c> {
    fn new(sink: &'s mut ReadingSink<'c>) -> Self {
        Self {
            cells: vec![Vec::new()],
            verbatim: false,
            has_line: false,
            sink,
        }
    }
    fn end(&mut self, keep_blank: bool, clock: &Clock) -> Result<(), Error> {
        let cells = std::mem::replace(&mut self.cells, vec![Vec::new()]);
        let mut content = false;
        for cell in &cells {
            for span in cell {
                for c in span.chars() {
                    clock.tick()?;
                    if self.verbatim || !c.is_whitespace() {
                        content = true;
                        break;
                    }
                }
                if content {
                    break;
                }
            }
            if content {
                break;
            }
        }
        if content || keep_blank {
            if self.has_line {
                self.sink.push("\n")?;
            }
            self.has_line = true;
            for (i, cell) in cells.iter().enumerate() {
                if i > 0 {
                    self.sink.push("\t")?;
                }
                let mut emitted = false;
                let mut space = false;
                for span in cell {
                    for c in span.chars() {
                        clock.tick()?;
                        if !self.verbatim && c.is_whitespace() {
                            space = emitted;
                        } else {
                            if space {
                                self.sink.push(" ")?;
                            }
                            let mut bytes = [0; 4];
                            self.sink.push(c.encode_utf8(&mut bytes))?;
                            emitted = true;
                            space = false;
                        }
                    }
                }
            }
        }
        self.verbatim = false;
        Ok(())
    }
    fn add(&mut self, text: &'d str, verbatim: bool, clock: &Clock) -> Result<(), Error> {
        if !verbatim {
            self.cells.last_mut().unwrap().push(text);
            return Ok(());
        }
        let mut start = 0;
        for (index, c) in text.char_indices() {
            clock.tick()?;
            self.verbatim = true;
            if c == '\n' {
                self.cells.last_mut().unwrap().push(&text[start..index]);
                self.end(true, clock)?;
                start = index + 1;
            }
        }
        if start < text.len() {
            self.verbatim = true;
            self.cells.last_mut().unwrap().push(&text[start..]);
        }
        Ok(())
    }
}

struct ReadingSink<'c> {
    clock: &'c Clock,
    value: String,
    pending_utf8: [u8; 4],
    pending_length: usize,
    reading_maximum: usize,
    encoded: usize,
    output: Option<(usize, usize)>,
}
impl<'c> ReadingSink<'c> {
    fn new(
        reading_maximum: usize,
        output: Option<(usize, usize)>,
        clock: &'c Clock,
    ) -> Result<Self, Error> {
        let sink = Self {
            clock,
            value: String::new(),
            pending_utf8: [0; 4],
            pending_length: 0,
            reading_maximum,
            encoded: 2,
            output,
        };
        if let Some((remaining, maximum)) = output
            && remaining < 2
        {
            return Err(Error::limit("output_bytes", maximum));
        }
        Ok(sink)
    }
    fn push(&mut self, text: &str) -> Result<(), Error> {
        for c in text.chars() {
            self.clock.tick()?;
            if c.len_utf8() > self.reading_maximum.saturating_sub(self.value.len()) {
                return Err(Error::limit("reading_bytes", self.reading_maximum));
            }
            let bytes = match c {
                '"' | '\\' | '\n' | '\r' | '\t' | '\u{8}' | '\u{c}' => 2,
                c if c < '\u{20}' => 6,
                c => c.len_utf8(),
            };
            if let Some((remaining, maximum)) = self.output
                && bytes > remaining.saturating_sub(self.encoded)
            {
                return Err(Error::limit("output_bytes", maximum));
            }
            self.encoded += bytes;
            self.value.push(c);
        }
        Ok(())
    }
}
impl io::Write for ReadingSink<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let mut position = 0;
        while self.pending_length > 0 && position < bytes.len() {
            self.pending_utf8[self.pending_length] = bytes[position];
            self.pending_length += 1;
            position += 1;
            match std::str::from_utf8(&self.pending_utf8[..self.pending_length]) {
                Ok(text) => {
                    let mut buffer = [0; 4];
                    let c = text.chars().next().unwrap();
                    self.push(c.encode_utf8(&mut buffer))
                        .map_err(io::Error::other)?;
                    self.pending_length = 0;
                }
                Err(error) if error.error_len().is_none() && self.pending_length < 4 => (),
                Err(error) => return Err(io::Error::other(error)),
            }
        }
        let rest = &bytes[position..];
        match std::str::from_utf8(rest) {
            Ok(text) => self.push(text).map_err(io::Error::other)?,
            Err(error) => {
                self.push(std::str::from_utf8(&rest[..error.valid_up_to()]).unwrap())
                    .map_err(io::Error::other)?;
                if error.error_len().is_some() {
                    return Err(io::Error::other(error));
                }
                let tail = &rest[error.valid_up_to()..];
                self.pending_utf8[..tail.len()].copy_from_slice(tail);
                self.pending_length = tail.len();
            }
        }
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        if self.pending_length == 0 {
            Ok(())
        } else {
            Err(io::Error::other("incomplete serialized UTF-8 scalar"))
        }
    }
}

fn text(
    scope: &Scope<'_>,
    excluded: &HashSet<NodeId>,
    clock: &Clock,
    sink: &mut ReadingSink,
) -> Result<(), Error> {
    if scope.single() && matches!(scope.anchor.value().name(), "script" | "style" | "template") {
        return text_content(scope, excluded, clock, sink);
    }
    let sole = (scope.single()).then_some(scope.anchor.id());
    let mut lines = Lines::new(sink);
    for root in scope.roots() {
        let mut skipped = 0;
        let mut verbatim = 0;
        let mut ancestor = root.parent();
        while let Some(node) = ancestor {
            clock.tick()?;
            if node.value().as_element().is_some_and(preformatted) {
                verbatim += 1;
            }
            ancestor = node.parent();
        }
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
                                lines.end(false, clock)?;
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
                                        && !excluded.contains(&sibling.id())
                                    {
                                        lines.cells.push(Vec::new());
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
    Ok(())
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
    output: &mut ReadingSink,
) -> Result<(), Error> {
    for root in scope.roots() {
        serialize(
            &mut *output,
            &Filtered {
                root,
                excluded,
                clock,
            },
            SerializeOpts::default(),
        )
        .map_err(|e| {
            e.get_ref()
                .and_then(|e| e.downcast_ref::<Error>())
                .cloned()
                .unwrap_or_else(|| {
                    Error::new(
                        ErrorCode::Internal,
                        format!("HTML serialization failed: {e}."),
                    )
                })
        })?;
    }
    Ok(())
}

#[cfg(test)]
pub(crate) fn read_html(root: crate::dom::ElementRef<'_>, clock: &Clock) -> Result<String, Error> {
    {
        let mut sink = ReadingSink::new(usize::MAX, None, clock)?;
        outer_html(&Scope::one(root), &HashSet::new(), clock, &mut sink)?;
        Ok(sink.value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn text_after_a_trailing_verbatim_line_feed_returns_to_normal_collapsing() {
        assert_eq!(
            text_of("<p id=x><textarea>a\n</textarea>  b  c </p>", "#x"),
            "a\nb c"
        );
    }

    #[test]
    fn canonical_control_escape_admission_precedes_appending_the_reading() {
        let limits = Limits::default();
        let clock = Clock::new(&limits);
        for maximum in [7, 8, 9] {
            let mut sink = ReadingSink::new(1, Some((maximum, maximum)), &clock).unwrap();
            let result = sink.push("\u{1}");
            if maximum >= 8 {
                result.unwrap();
                assert_eq!(sink.value, "\u{1}");
            } else {
                let error = result.unwrap_err();
                assert_eq!(error.limit.as_deref(), Some("output_bytes"));
                assert_eq!(error.bound, Some(7.into()));
                assert!(
                    sink.value.is_empty(),
                    "refused source must not enter the retained reading"
                );
            }
        }
        let contract = Contract::from_json(r#"{"select":"p","read":"text-content"}"#).unwrap();
        let document = Document::parse(Source::new(b"<p>&#1;</p>"), &limits).unwrap();
        assert_eq!(
            document.extract(&contract).unwrap().to_json(8).unwrap(),
            b"\"\\u0001\""
        );
    }

    #[test]
    fn outer_html_reassembles_the_native_serializers_split_c2_scalars() {
        let document = Document::parse(
            Source::new("<p title='¡\u{85}&nbsp;'>¡\u{85}&nbsp;é雪😀</p>".as_bytes()),
            &Limits::default(),
        )
        .unwrap();
        let contract = Contract::from_json(r#"{"select":"p","read":"outer-html"}"#).unwrap();
        assert_eq!(
            document.extract(&contract).unwrap(),
            Value::String("<p title=\"¡\u{85}&nbsp;\">¡\u{85}&nbsp;é雪😀</p>".into())
        );
    }

    #[test]
    fn serialized_utf8_writer_preserves_fragmented_scalars_and_rejects_malformed_continuations() {
        use std::io::Write;
        let limits = Limits::default();
        let clock = Clock::new(&limits);
        for scalar in ["¡", "雪", "😀"] {
            for split in 1..scalar.len() {
                let mut sink = ReadingSink::new(scalar.len(), None, &clock).unwrap();
                sink.write_all(&scalar.as_bytes()[..split]).unwrap();
                assert!(
                    sink.flush().is_err(),
                    "incomplete {scalar:?} split at {split}"
                );
                sink.write_all(&scalar.as_bytes()[split..]).unwrap();
                sink.flush().unwrap();
                assert_eq!(sink.value, scalar);
            }
            let mut sink = ReadingSink::new(scalar.len(), None, &clock).unwrap();
            for byte in scalar.as_bytes() {
                sink.write_all(&[*byte]).unwrap();
            }
            sink.flush().unwrap();
            assert_eq!(sink.value, scalar);
        }
        for (prefix, invalid) in [
            (&[0xc2][..], b"x".as_slice()),
            (&[0xf0, 0x9f][..], &[0xff][..]),
        ] {
            let mut sink = ReadingSink::new(8, None, &clock).unwrap();
            sink.write_all(prefix).unwrap();
            assert!(sink.write(invalid).is_err());
            assert!(sink.value.is_empty());
        }
    }

    use crate::{Contract, Limits, Source, Value};
    fn value(html: &str, contract: serde_json::Value) -> Value {
        let document = Document::parse(Source::new(html.as_bytes()), &Limits::default()).unwrap();
        document
            .extract(&Contract::from_json(&contract.to_string()).unwrap())
            .unwrap()
    }
    fn text_of(html: &str, select: &str) -> String {
        match value(html, serde_json::json!({"select":select})) {
            Value::String(value) => value,
            _ => panic!("not a string"),
        }
    }
    #[test]
    fn single_unrendered_roots_return_their_text_content() {
        for (html, expected) in [
            ("<script id=x>  a\n b</script>", "  a\n b"),
            ("<style id=x>  a\n b</style>", "  a\n b"),
            ("<template id=x><b>a</b><!--c--><i>b</i></template>", "ab"),
        ] {
            assert_eq!(text_of(html, "#x"), expected);
        }
    }
    #[test]
    fn descendants_skip_unrendered_namespaces_and_comments() {
        let html = "<!doctype html><head><title>hidden</title><style>bad</style></head><body><article id=x>a<!--comment--><script>bad</script><style>bad</style><template>bad</template><svg><script>bad</script><style>bad</style><text>b</text></svg>c</article></body>";
        assert_eq!(text_of(html, "html"), "abc");
        assert_eq!(
            value(html, serde_json::json!({"select":"#x","exclude":["svg"]})),
            Value::String("ac".into())
        );
    }
    #[test]
    fn inline_runs_share_a_line_and_cells_preserve_empty_values() {
        assert_eq!(
            text_of("<p id=x><span>a</span> b <i>c</i></p>", "#x"),
            "a b c"
        );
        assert_eq!(
            text_of(
                "<table id=x><tr><td>a</td><td></td><td>c</td></tr></table>",
                "#x"
            ),
            "a\t\tc"
        );
    }
    #[test]
    fn html_blocks_bound_lines_without_using_author_styles() {
        assert_eq!(
            text_of(
                "<article id=x><h2 style='display:inline'>a</h2><p>b</p><ul><li>c</li><li>d</li></ul></article>",
                "#x"
            ),
            "a\nb\nc\nd"
        );
    }
    #[test]
    fn break_elements_end_lines_and_drop_lines_without_text() {
        assert_eq!(text_of("<p id=x>a<br><br>b</p>", "#x"), "a\nb");
        assert_eq!(text_of("<pre id=x><br><br></pre>", "#x"), "");
    }
    #[test]
    fn cells_use_a_prior_unexcluded_cell_except_when_selected_alone() {
        let html = "<table id=x><tr><td>a</td><td class=omit>x</td><td id=last>b</td></tr></table>";
        assert_eq!(
            value(html, serde_json::json!({"select":"#x","exclude":[".omit"]})),
            Value::String("a\tb".into())
        );
        assert_eq!(text_of(html, "#last"), "b");
    }
    #[test]
    fn preformatted_line_feeds_keep_blank_lines_and_ancestor_context() {
        assert_eq!(text_of("<pre id=x>  a\n\n\tb</pre>", "#x"), "  a\n\n\tb");
        assert_eq!(
            text_of("<pre><code id=x>  a\n b</code></pre>", "#x"),
            "  a\n b"
        );
        assert_eq!(
            text_of("<textarea id=x>  a\n\n b</textarea>", "#x"),
            "  a\n\n b"
        );
    }
    #[test]
    fn a_verbatim_line_preserves_every_cell_and_whitespace_only_content() {
        assert_eq!(
            text_of("<p id=x>a  <textarea> b </textarea>  c</p>", "#x"),
            "a   b   c"
        );
        assert_eq!(text_of("<pre id=x>  </pre>", "#x"), "  ");
    }
    #[test]
    fn normal_lines_collapse_unicode_white_space_and_drop_empty_rows() {
        assert_eq!(
            text_of(
                "<div id=x>\u{a0} a\u{85}\u{2028}b \u{a0}<p> \t </p><p>c</p> </div>",
                "#x"
            ),
            "a b\nc"
        );
        // C0 record separators are not Unicode White_Space, unlike Python's \s.
        assert_eq!(text_of("<p id=x>a\u{1c}b</p>", "#x"), "a\u{1c}b");
    }
    #[test]
    fn line_joining_adds_no_framing_line_feed() {
        assert_eq!(text_of("<div id=x><p>a</p><p>b</p></div>", "#x"), "a\nb");
        assert_eq!(text_of("<pre id=x>a\n</pre>", "#x"), "a");
        // A final preserved blank line contributes a separator, not an added framing LF.
        assert_eq!(text_of("<pre id=x>a\n\n</pre>", "#x"), "a\n");
    }
    #[test]
    fn qualified_attribute_names_are_exact_and_urls_accept_any_attribute() {
        let mut source = Source::new(
            b"<svg id=x viewBox='0 0' xlink:href='../doc'></svg><meta id=m content=' img.png '>",
        );
        source.base_url = Some("https://example.test/app/page");
        let document = Document::parse(source, &Limits::default()).unwrap();
        for (read, expected) in [
            ("attr:viewBox", "0 0"),
            ("attr:xlink:href", "../doc"),
            ("url:xlink:href", "https://example.test/doc"),
        ] {
            let c =
                Contract::from_json(&serde_json::json!({"select":"#x","read":read}).to_string())
                    .unwrap();
            assert_eq!(
                document.extract(&c).unwrap(),
                Value::String(expected.into())
            );
        }
        let c = Contract::from_json(r##"{"select":"#x","read":"attr:viewbox"}"##).unwrap();
        assert_eq!(
            document.extract(&c).unwrap_err().error,
            ErrorCode::MissingAttribute
        );
        let c = Contract::from_json(r##"{"select":"#m","read":"url:content"}"##).unwrap();
        assert_eq!(
            document.extract(&c).unwrap(),
            Value::String("https://example.test/app/img.png".into())
        );
    }
    #[test]
    fn document_url_query_uses_effective_encoding_for_base_and_attributes_only() {
        use crate::{Contract, Value};
        let contract = Contract::from_json(r#"{"select":"a","read":"url:href"}"#).unwrap();
        for (bytes, encoding, fallback, expected) in [
            (
                b"<meta charset=windows-1252><base href='?q=\xe9'><a href='#id'>x".to_vec(),
                None,
                "https://origin.test/page",
                "https://origin.test/page?q=%E9#id",
            ),
            (
                b"<a href='?q=\xe9'>x".to_vec(),
                Some("windows-1252"),
                "https://origin.test/page",
                "https://origin.test/page?q=%E9",
            ),
            (
                "<meta charset=windows-1252><a href='?q=é'>x"
                    .as_bytes()
                    .to_vec(),
                Some("utf-8"),
                "https://origin.test/page",
                "https://origin.test/page?q=%C3%A9",
            ),
            (
                "\u{feff}<meta charset=windows-1252><a href='?q=é'>x"
                    .as_bytes()
                    .to_vec(),
                Some("windows-1252"),
                "https://origin.test/page",
                "https://origin.test/page?q=%C3%A9",
            ),
            (
                b"<meta charset=windows-1252><a href='#id'>x".to_vec(),
                None,
                "https://origin.test/page?q=é",
                "https://origin.test/page?q=%C3%A9#id",
            ),
            (
                b"<meta charset=windows-1252><a href='/\xe9?q=\xe9#\xe9'>x".to_vec(),
                None,
                "https://origin.test/page",
                "https://origin.test/%C3%A9?q=%E9#%C3%A9",
            ),
            (
                b"<meta charset=windows-1252><a href='wss://origin.test/\xe9?q=\xe9#\xe9'>x"
                    .to_vec(),
                None,
                "https://origin.test/page",
                "wss://origin.test/%C3%A9?q=%C3%A9#%C3%A9",
            ),
        ] {
            let mut source = Source::new(&bytes);
            source.encoding = encoding;
            source.base_url = Some(fallback);
            let document = Document::parse(source, &Limits::default()).unwrap();
            assert_eq!(
                document.extract(&contract).unwrap(),
                Value::String(expected.into()),
                "encoding {encoding:?}"
            );
        }
        for little_endian in [true, false] {
            let text = "<meta charset=windows-1252><a href='?q=é'>x";
            let mut bytes = if little_endian {
                vec![0xff, 0xfe]
            } else {
                vec![0xfe, 0xff]
            };
            for unit in text.encode_utf16() {
                bytes.extend(if little_endian {
                    unit.to_le_bytes()
                } else {
                    unit.to_be_bytes()
                });
            }
            let mut source = Source::new(&bytes);
            source.encoding = Some("windows-1252");
            source.base_url = Some("https://origin.test/page");
            let document = Document::parse(source, &Limits::default()).unwrap();
            assert_eq!(
                document.extract(&contract).unwrap(),
                Value::String("https://origin.test/page?q=%C3%A9".into())
            );
        }
    }

    #[test]
    fn document_url_query_escapes_only_actual_encoding_errors() {
        use crate::{Contract, Value};
        let contract = Contract::from_json(r#"{"select":"a","read":"url:href"}"#).unwrap();
        for (label, href, expected) in [
            (
                "windows-1252",
                "?q=&#20013;",
                "https://origin.test/page?q=%26%2320013%3B",
            ),
            (
                "windows-1252",
                "?q=&#20013;&#128512;&amp;x=%26%2320013%3B",
                "https://origin.test/page?q=%26%2320013%3B%26%23128512%3B&x=%26%2320013%3B",
            ),
            (
                "windows-1252",
                "?q=&amp;#20013;",
                "https://origin.test/page?q=&#20013;",
            ),
            (
                "shift_jis",
                "?q=&#8801;&#8253;",
                "https://origin.test/page?q=%81%DF%26%238253%3B",
            ),
            (
                "iso-2022-jp",
                "?q=&#165;&#128512;&#165;",
                r"https://origin.test/page?q=%1B(J\%26%23128512%3B\%1B(B",
            ),
            (
                "iso-2022-jp",
                "?q=&#12354;&#128512;&#12354;",
                "https://origin.test/page?q=%1B$B$%22%1B(B%26%23128512%3B%1B$B$%22%1B(B",
            ),
        ] {
            let html = format!("<meta charset={label}><a href='{href}'>x");
            let mut source = Source::new(html.as_bytes());
            source.base_url = Some("https://origin.test/page");
            let document = Document::parse(source, &Limits::default()).unwrap();
            assert_eq!(
                document.extract(&contract).unwrap(),
                Value::String(expected.into()),
                "{label}: {href}"
            );
        }
    }

    #[test]
    fn document_url_query_state_survives_removed_controls_and_encoder_output_buffers() {
        use crate::{Contract, Value};
        let contract = Contract::from_json(r#"{"select":"a","read":"url:href"}"#).unwrap();
        for control in ["&#9;", "&#10;", "&#13;"] {
            let html = format!("<meta charset=iso-2022-jp><a href='?q=&#165;{control}&#165;'>x");
            let mut source = Source::new(html.as_bytes());
            source.base_url = Some("https://origin.test/page");
            let document = Document::parse(source, &Limits::default()).unwrap();
            assert_eq!(
                document.extract(&contract).unwrap(),
                Value::String(r"https://origin.test/page?q=%1B(J\\%1B(B".into())
            );
        }
        let html = format!(
            "<meta charset=iso-2022-jp><a href='?q={}'>x",
            "&#165;".repeat(1025)
        );
        let mut source = Source::new(html.as_bytes());
        source.base_url = Some("https://origin.test/page");
        let document = Document::parse(source, &Limits::default()).unwrap();
        let expected = format!("https://origin.test/page?q=%1B(J{}%1B(B", "\\".repeat(1025));
        assert_eq!(
            document.extract(&contract).unwrap(),
            Value::String(expected)
        );
    }

    #[test]
    fn filtered_serialization_keeps_the_original_tree_and_source_attribute_order() {
        let html = "<main id=x><p z='2' a='1'>a &amp; b</p><!--keep--><b class=omit>gone</b><noscript><i>x</i></noscript></main>";
        let document = Document::parse(Source::new(html.as_bytes()), &Limits::default()).unwrap();
        let source_html = document.html.html();
        let c = Contract::from_json(r##"{"select":"#x","read":"outer-html","exclude":[".omit"]}"##)
            .unwrap();
        assert_eq!(document.extract(&c).unwrap(), Value::String("<main id=\"x\"><p z=\"2\" a=\"1\">a &amp; b</p><!--keep--><noscript><i>x</i></noscript></main>".into()));
        assert_eq!(document.html.html(), source_html);
        assert_eq!(document.extract(&c).unwrap(), document.extract(&c).unwrap());
    }
    #[test]
    fn foreign_block_and_cell_names_do_not_gain_html_layout() {
        assert_eq!(
            text_of(
                "<svg id=x><address>a</address><address>b</address><td>c</td><td>d</td></svg>",
                "#x"
            ),
            "abcd"
        );
        assert_eq!(
            text_of(
                "<math id=x><address>a</address><address>b</address><td>c</td><td>d</td></math>",
                "#x"
            ),
            "abcd"
        );
    }
    #[test]
    fn comments_and_excluded_cells_do_not_create_a_leading_column() {
        let html = "<table id=x><tr><!--before--><td class=omit>x</td><!--between--><td>a</td><th>b</th></tr></table>";
        assert_eq!(
            value(html, serde_json::json!({"select":"#x","exclude":[".omit"]})),
            Value::String("a\tb".into())
        );
        assert_eq!(
            text_of(
                "<table id=x><tr><!--before--><td>a</td><th>b</th></tr></table>",
                "#x"
            ),
            "a\tb"
        );
    }
    #[test]
    fn unrendered_element_siblings_do_not_create_a_table_column() {
        let html = "<table id=x><tr><td>first</td></tr><tr><script>ignored</script><template><td>inert</td></template><td>a</td><th>b</th></tr></table>";
        let document = Document::parse(Source::new(html.as_bytes()), &Limits::default()).unwrap();
        // HTML retains these non-cell elements in the row, unlike foster-parented span/div nodes.
        let row = document
            .html
            .select(&crate::select::Selector::parse("tr").unwrap())
            .nth(1)
            .unwrap();
        let names: Vec<_> = row
            .children()
            .filter_map(crate::dom::ElementRef::wrap)
            .map(|e| e.value().name())
            .collect();
        assert_eq!(names, ["script", "template", "td", "th"]);
        assert_eq!(text_of(html, "#x"), "first\na\tb");
    }
    #[test]
    fn doctypes_cannot_enter_element_or_following_sibling_readings() {
        for html in [
            "<!doctype html><html><head></head><body><main>a<!doctype html>b<h2>x</h2>c<!doctype html>d<h2>stop</h2></main></body></html>",
            "<!doctype html><html><head></head><body><main>ab<h2>x</h2>cd<h2>stop</h2></main></body></html><!doctype html>",
        ] {
            let document =
                Document::parse(Source::new(html.as_bytes()), &Limits::default()).unwrap();
            // The parser's document doctype precedes the root element; interior and trailing doctypes are discarded.
            let doctypes: Vec<_> = document
                .html
                .tree
                .nodes()
                .filter(|n| matches!(n.value(), Node::Doctype))
                .collect();
            assert_eq!(doctypes.len(), 1);
            assert!(doctypes[0].parent().unwrap().value().is_document());
            assert_eq!(value(html, serde_json::json!({"select":"html","read":"outer-html"})), Value::String("<html><head></head><body><main>ab<h2>x</h2>cd<h2>stop</h2></main></body></html>".into()));
            assert_eq!(
                value(
                    html,
                    serde_json::json!({"select":"h2","nth":1,"until":"h2","read":"outer-html"})
                ),
                Value::String("<h2>x</h2>cd".into())
            );
        }
        assert_eq!(
            value(
                "<!doctype html><template id=x><!doctype html><b>x</b></template>",
                serde_json::json!({"select":"#x","read":"outer-html"})
            ),
            Value::String("<template id=\"x\"><b>x</b></template>".into())
        );
    }
    #[test]
    fn output_policy_applies_to_the_final_capture_without_an_intermediate_value_limit() {
        let html = format!("<p>{}<b>x</b></p>", "a".repeat(10_000));
        let limits = Limits {
            max_output_bytes: 3,
            ..Limits::default()
        };
        let document = Document::parse(Source::new(html.as_bytes()), &limits).unwrap();
        let contract = Contract::from_json(
            r#"{"select":"p","read":"outer-html","pattern":"<b>(?<value>x)</b>"}"#,
        )
        .unwrap();
        assert_eq!(
            document.extract(&contract).unwrap(),
            Value::String("x".into())
        );
    }
}
