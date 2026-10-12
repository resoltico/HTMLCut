// SPDX-License-Identifier: MPL-2.0 AND ISC
//! Deterministic structural text and original-tree readings.
use crate::{Document, Error, ErrorCode, contract::Reading, eval::Scope, limits::Clock};
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
                    ErrorCode::MissingAttribute,
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
                            ErrorCode::UnresolvableUrl,
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
                                        lines.cells.push(String::new());
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
                    Error::new(
                        ErrorCode::Internal,
                        format!("HTML serialization failed: {e}."),
                    )
                })
        })?;
    }
    String::from_utf8(output).map_err(|_| {
        Error::new(
            ErrorCode::Internal,
            "HTML serialization produced invalid UTF-8.",
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn reading_conformance_matches_complete_values() {
        let corpus =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/contract");
        let mut count = 0;
        for entry in std::fs::read_dir(corpus).unwrap() {
            let path = entry.unwrap().path();
            let prefix: usize = path.file_name().unwrap().to_str().unwrap()[..2]
                .parse()
                .unwrap();
            if !matches!(prefix, 60..=67 | 69) {
                continue;
            }
            let options: serde_json::Value = std::fs::read(path.join("options.json"))
                .ok()
                .map(|v| serde_json::from_slice(&v).unwrap())
                .unwrap_or_default();
            let bytes = std::fs::read(path.join("page.html")).unwrap();
            let mut source = Source::new(&bytes);
            source.base_url = options["base_url"].as_str();
            let document = Document::parse(source, &Limits::default()).unwrap();
            let contract =
                Contract::from_json(&std::fs::read_to_string(path.join("contract.json")).unwrap())
                    .unwrap();
            let expected: serde_json::Value =
                serde_json::from_slice(&std::fs::read(path.join("expected.json")).unwrap())
                    .unwrap();
            assert_eq!(
                serde_json::to_value(document.extract(&contract).unwrap()).unwrap(),
                expected,
                "{path:?}"
            );
            count += 1;
        }
        assert_eq!(count, 10);
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
