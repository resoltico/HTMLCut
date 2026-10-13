// SPDX-License-Identifier: MPL-2.0
//! Byte decoding and incremental upstream tree admission.
use crate::{Error, ErrorCode, Limits, limits::Clock};
use encoding_rs::{
    DecoderResult, Encoding, UTF_8, UTF_16BE, UTF_16LE, WINDOWS_1252, X_USER_DEFINED,
};
use html5ever::{
    driver,
    tendril::{StrTendril, TendrilSink},
};
use scraper::{Html, HtmlTreeSink};
use url::Url;

const CHUNK_BYTES: usize = 65_536;
const MAX_DEPTH: usize = 1_024;

/// Caller-owned document bytes and optional decoding/base metadata.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct Source<'a> {
    /// Exact input bytes.
    pub bytes: &'a [u8],
    /// A WHATWG encoding label, overridden by a byte order mark.
    pub encoding: Option<&'a str>,
    /// The document's fallback base URL.
    pub base_url: Option<&'a str>,
}
impl<'a> Source<'a> {
    /// Uses default detection and no fallback base URL.
    pub fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            encoding: None,
            base_url: None,
        }
    }
}

/// An immutable admitted document, reusable for contracts and outlines.
#[derive(Debug)]
pub struct Document {
    pub(crate) html: Html,
    pub(crate) limits: Limits,
    pub(crate) base_url: Option<Url>,
}
impl Document {
    /// Decodes without replacement, parses incrementally and admits node/depth limits.
    pub fn parse(source: Source<'_>, limits: &Limits) -> Result<Self, Error> {
        let clock = Clock::new(limits);
        if source.bytes.len() > limits.max_input_bytes {
            return Err(Error::limit("input_bytes", limits.max_input_bytes));
        }
        let fallback = source
            .base_url
            .map(|base| {
                Url::parse(base).map_err(|e| {
                    Error::new(
                        ErrorCode::InvalidBaseUrl,
                        format!("Invalid base URL {base:?}: {e}."),
                    )
                })
            })
            .transpose()?;
        let decoded = decode(source, &clock)?;
        let html = parse_admitted(&decoded, limits, CHUNK_BYTES, &clock)?;
        // The first base with href governs, even when its URL is unusable.
        let mut document_base = None;
        let mut fragment_depth = 0;
        for edge in html.tree.root().traverse() {
            clock.tick()?;
            match edge {
                ego_tree::iter::Edge::Open(node) => {
                    if fragment_depth > 0 || matches!(node.value(), scraper::Node::Fragment) {
                        fragment_depth += 1;
                    } else if let Some(e) = scraper::ElementRef::wrap(node)
                        && e.value().name.ns == html5ever::ns!(html)
                        && e.value().name() == "base"
                        && let Some(href) = e.value().attr("href")
                    {
                        document_base = resolve(href, fallback.as_ref());
                        break;
                    }
                }
                ego_tree::iter::Edge::Close(_) if fragment_depth > 0 => fragment_depth -= 1,
                ego_tree::iter::Edge::Close(_) => (),
            }
        }
        let base_url = document_base.or(fallback);
        clock.check()?;
        Ok(Self {
            html,
            limits: limits.clone(),
            base_url,
        })
    }
}

pub(crate) fn resolve(value: &str, base: Option<&Url>) -> Option<Url> {
    let value = value.trim_matches(|c: char| c.is_ascii_whitespace());
    match base {
        Some(base) => base.join(value).ok(),
        None => Url::parse(value).ok(),
    }
}

fn parse_admitted(text: &str, limits: &Limits, chunk: usize, clock: &Clock) -> Result<Html, Error> {
    let mut parser =
        driver::parse_document(HtmlTreeSink::new(Html::new_document()), Default::default());
    feed(&mut parser, text, limits, chunk, clock)?;
    clock.check()?;
    let html = parser.finish();
    admit_tree(html, limits, clock)
}

fn feed(
    parser: &mut html5ever::Parser<HtmlTreeSink>,
    text: &str,
    limits: &Limits,
    chunk: usize,
    clock: &Clock,
) -> Result<(), Error> {
    let mut remaining = text;
    while !remaining.is_empty() {
        clock.check()?;
        let mut end = chunk.min(remaining.len());
        // A split is never larger than the configured chunk, except test chunks
        // too small to contain one Unicode scalar.
        while end > 0 && !remaining.is_char_boundary(end) {
            end -= 1;
        }
        if end == 0 {
            end = remaining.chars().next().unwrap().len_utf8();
        }
        parser.process(StrTendril::from_slice(&remaining[..end]));
        remaining = &remaining[end..];
        let nodes = parser.tokenizer.sink.sink.0.borrow().tree.nodes().len();
        if nodes > limits.max_nodes {
            return Err(Error::limit("nodes", limits.max_nodes));
        }
    }
    Ok(())
}

fn admit_tree(html: Html, limits: &Limits, clock: &Clock) -> Result<Html, Error> {
    if html.tree.nodes().len() > limits.max_nodes {
        return Err(Error::limit("nodes", limits.max_nodes));
    }
    // Arena roots include retained detached nodes; all roots get the same admission.
    for root in html.tree.nodes().filter(|node| node.parent().is_none()) {
        let mut depth = 0;
        for edge in root.traverse() {
            clock.tick()?;
            match edge {
                ego_tree::iter::Edge::Open(_) => {
                    if depth > MAX_DEPTH {
                        return Err(Error::limit("depth", MAX_DEPTH));
                    }
                    depth += 1;
                }
                ego_tree::iter::Edge::Close(_) => depth -= 1,
            }
        }
    }
    clock.check()?;
    Ok(html)
}

fn decode(source: Source<'_>, clock: &Clock) -> Result<String, Error> {
    let (encoding, skip) = if let Some(bom) = Encoding::for_bom(source.bytes) {
        bom
    } else if let Some(label) = source.encoding {
        (
            Encoding::for_label(label.as_bytes()).ok_or_else(|| {
                Error::new(
                    ErrorCode::InvalidEncoding,
                    format!("Unknown encoding label {label:?}."),
                )
            })?,
            0,
        )
    } else {
        (
            prescan(&source.bytes[..source.bytes.len().min(1024)]).unwrap_or(UTF_8),
            0,
        )
    };
    let bytes = &source.bytes[skip..];
    let mut decoder = encoding.new_decoder_without_bom_handling();
    let mut output = String::new();
    let mut consumed = 0;
    loop {
        clock.check()?;
        let end = (consumed + CHUNK_BYTES).min(bytes.len());
        let last = end == bytes.len();
        let input = &bytes[consumed..end];
        output.reserve(
            decoder
                .max_utf8_buffer_length_without_replacement(input.len())
                .ok_or_else(|| Error::limit("input_bytes", source.bytes.len()))?,
        );
        let (result, read) = decoder.decode_to_string_without_replacement(input, &mut output, last);
        consumed += read;
        match result {
            DecoderResult::InputEmpty if last => return Ok(output),
            DecoderResult::InputEmpty | DecoderResult::OutputFull => (),
            DecoderResult::Malformed(length, after) => {
                let offset = skip + consumed - usize::from(length) - usize::from(after);
                let name = encoding.name().to_ascii_lowercase();
                let mut e = Error::new(
                    ErrorCode::UndecodableInput,
                    format!("Input is not valid {name} at byte {offset}."),
                );
                e.encoding = Some(name);
                e.offset = Some(offset);
                return Err(e);
            }
        }
    }
}

// https://html.spec.whatwg.org/multipage/parsing.html#prescan-a-byte-stream-to-determine-its-encoding
fn prescan(bytes: &[u8]) -> Option<&'static Encoding> {
    if bytes.starts_with(b"<\0?\0x\0") {
        return Some(UTF_16LE);
    }
    if bytes.starts_with(b"\0<\0?\0x") {
        return Some(UTF_16BE);
    }
    prescan_meta(bytes).or_else(|| xml_encoding(bytes))
}

fn prescan_meta(bytes: &[u8]) -> Option<&'static Encoding> {
    let mut position = 0;
    while position < bytes.len() {
        let tail = &bytes[position..];
        if tail.starts_with(b"<!--") {
            position += tail.windows(3).position(|v| v == b"-->")? + 3;
        } else if tail.len() > 5
            && tail[..5].eq_ignore_ascii_case(b"<meta")
            && (space(tail[5]) || tail[5] == b'/')
        {
            position += 5;
            let attrs = attributes(bytes, &mut position)?;
            let mut seen = std::collections::HashSet::new();
            let mut got_pragma = false;
            let mut need_pragma = None;
            let mut charset = None;
            for (name, value) in attrs {
                if !seen.insert(name.clone()) {
                    continue;
                }
                match name.as_slice() {
                    b"http-equiv" => got_pragma = value == b"content-type",
                    b"content" if charset.is_none() => {
                        if let Some(found) = content_encoding(&value) {
                            charset = Some(found);
                            need_pragma = Some(true);
                        }
                    }
                    b"charset" => {
                        charset = Encoding::for_label(&value);
                        need_pragma = Some(false);
                    }
                    _ => (),
                }
            }
            if (need_pragma == Some(false) || (need_pragma == Some(true) && got_pragma))
                && let Some(encoding) = charset
            {
                return Some(if encoding == UTF_16LE || encoding == UTF_16BE {
                    UTF_8
                } else if encoding == X_USER_DEFINED {
                    WINDOWS_1252
                } else {
                    encoding
                });
            }
        } else if tail[0] == b'<'
            && tail.get(1).is_some_and(|c| {
                c.is_ascii_alphabetic()
                    || (*c == b'/' && tail.get(2).is_some_and(u8::is_ascii_alphabetic))
            })
        {
            position += 1;
            while position < bytes.len() && !space(bytes[position]) && bytes[position] != b'>' {
                position += 1;
            }
            attributes(bytes, &mut position)?;
        } else if tail.starts_with(b"<!") || tail.starts_with(b"</") || tail.starts_with(b"<?") {
            position += tail.iter().position(|c| *c == b'>')? + 1;
        } else {
            position += 1;
        }
    }
    None
}

fn space(c: u8) -> bool {
    matches!(c, b'\t' | b'\n' | b'\x0c' | b'\r' | b' ')
}

fn attributes(bytes: &[u8], position: &mut usize) -> Option<Vec<(Vec<u8>, Vec<u8>)>> {
    let mut result = Vec::new();
    loop {
        while bytes
            .get(*position)
            .is_some_and(|c| space(*c) || *c == b'/')
        {
            *position += 1;
        }
        if *bytes.get(*position)? == b'>' {
            *position += 1;
            return Some(result);
        }
        let mut name = Vec::new();
        while let Some(&c) = bytes.get(*position) {
            if space(c) || c == b'/' || c == b'>' || (c == b'=' && !name.is_empty()) {
                break;
            }
            name.push(c.to_ascii_lowercase());
            *position += 1;
        }
        while bytes.get(*position).is_some_and(|c| space(*c)) {
            *position += 1;
        }
        let mut value = Vec::new();
        if *bytes.get(*position)? == b'=' {
            *position += 1;
            while bytes.get(*position).is_some_and(|c| space(*c)) {
                *position += 1;
            }
            let first = *bytes.get(*position)?;
            if matches!(first, b'\'' | b'"') {
                *position += 1;
                while *bytes.get(*position)? != first {
                    value.push(bytes[*position].to_ascii_lowercase());
                    *position += 1;
                }
                *position += 1;
            } else {
                while let Some(&c) = bytes.get(*position) {
                    if space(c) || c == b'>' {
                        break;
                    }
                    value.push(c.to_ascii_lowercase());
                    *position += 1;
                }
            }
        }
        result.push((name, value));
    }
}

fn content_encoding(bytes: &[u8]) -> Option<&'static Encoding> {
    let mut rest = bytes;
    loop {
        let offset = rest
            .windows(7)
            .position(|v| v.eq_ignore_ascii_case(b"charset"))?;
        rest = &rest[offset + 7..];
        while rest.first().is_some_and(|c| space(*c)) {
            rest = &rest[1..];
        }
        if rest.first() != Some(&b'=') {
            continue;
        }
        rest = &rest[1..];
        while rest.first().is_some_and(|c| space(*c)) {
            rest = &rest[1..];
        }
        let first = *rest.first()?;
        let label = if matches!(first, b'\'' | b'"') {
            let end = rest[1..].iter().position(|c| *c == first)? + 1;
            &rest[1..end]
        } else {
            &rest[..rest
                .iter()
                .position(|c| space(*c) || *c == b';')
                .unwrap_or(rest.len())]
        };
        return Encoding::for_label(label);
    }
}

fn xml_encoding(bytes: &[u8]) -> Option<&'static Encoding> {
    if !bytes.starts_with(b"<?xml") {
        return None;
    }
    let end = bytes.iter().position(|c| *c == b'>')?;
    let offset = bytes[..end].windows(8).position(|v| v == b"encoding")?;
    let mut rest = &bytes[offset + 8..end];
    while rest.first().is_some_and(|c| *c <= b' ') {
        rest = &rest[1..];
    }
    if rest.first() != Some(&b'=') {
        return None;
    }
    rest = &rest[1..];
    while rest.first().is_some_and(|c| *c <= b' ') {
        rest = &rest[1..];
    }
    let quote = *rest.first()?;
    if !matches!(quote, b'\'' | b'"') {
        return None;
    }
    let end = rest[1..].iter().position(|c| *c == quote)? + 1;
    let label = &rest[1..end];
    if label.iter().any(|c| *c <= b' ') {
        return None;
    }
    let encoding = Encoding::for_label(label)?;
    Some(if encoding == UTF_16LE || encoding == UTF_16BE {
        UTF_8
    } else {
        encoding
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn chunk_boundaries_preserve_authored_document_trees() {
        let documents = [
            "<!doctype html><title>é🦀</title><p>a &amp; b<!--comment--></p>".into(),
            "<table><p>fostered</p><tr><td>a<td>b</table><p><b>misnested<i>x</b>y".into(),
            "<template><p>inert</p><template><b>nested</b></template></template>".into(),
            "<svg><foreignObject><p>HTML</p></foreignObject><text>SVG</text></svg>".into(),
            "<script>if (a < b) x++;</script><noscript><b>x</b></noscript><textarea>a\nb</textarea>".into(),
            format!("<main>{}é🦀 &amp; end</main>", "x".repeat(CHUNK_BYTES - 8)),
        ];
        let limits = Limits {
            time_limit: None,
            ..Limits::default()
        };
        for document in documents {
            let one = Html::parse_document(&document);
            for chunk in [1, 7, CHUNK_BYTES] {
                let many = parse_admitted(&document, &limits, chunk, &Clock::new(&limits)).unwrap();
                assert_eq!(
                    one.tree.nodes().len(),
                    many.tree.nodes().len(),
                    "chunk {chunk}"
                );
                assert_eq!(one.html(), many.html(), "chunk {chunk}");
            }
        }
    }
    #[test]
    fn twenty_million_elements_stop_at_incremental_default_node_admission() {
        let input = "<a>".repeat(20_000_000);
        let limits = Limits {
            time_limit: None,
            ..Limits::default()
        };
        assert!(input.len() <= limits.max_input_bytes);
        let mut parser =
            driver::parse_document(HtmlTreeSink::new(Html::new_document()), Default::default());
        let error = feed(
            &mut parser,
            &input,
            &limits,
            CHUNK_BYTES,
            &Clock::new(&limits),
        )
        .unwrap_err();
        assert_eq!(error.limit.as_deref(), Some("nodes"));
        assert_eq!(error.bound, Some(limits.max_nodes.into()));
        let observed = parser.tokenizer.sink.sink.0.borrow().tree.nodes().len();
        assert!(observed > limits.max_nodes);
        assert!(
            observed < limits.max_nodes + CHUNK_BYTES / 3 + 4,
            "observed {observed}"
        );
    }
    #[test]
    fn depth_boundary_and_detached_node_admission_are_real() {
        for depth in [MAX_DEPTH, MAX_DEPTH + 1] {
            let html = format!(
                "<html><head></head><body>{}{}</body></html>",
                "<div>".repeat(depth - 2),
                "</div>".repeat(depth - 2)
            );
            let result = Document::parse(Source::new(html.as_bytes()), &Limits::default());
            if depth == MAX_DEPTH {
                assert!(result.is_ok());
            } else {
                assert_eq!(result.unwrap_err().limit.as_deref(), Some("depth"));
            }
        }
        let input = "<p></p><frameset>";
        let parsed = Html::parse_document(input);
        let attached = parsed.tree.root().descendants().count();
        assert!(parsed.tree.nodes().len() > attached);
        let limits = Limits {
            max_nodes: attached,
            ..Limits::default()
        };
        assert_eq!(
            Document::parse(Source::new(input.as_bytes()), &limits)
                .unwrap_err()
                .limit
                .as_deref(),
            Some("nodes")
        );
    }
    #[test]
    fn standard_prescan_handles_pragmas_substitutions_boundaries_and_false_declarations() {
        for (bytes, expected) in [
            (
                b"<META HTTP-EQUIV='Content-Type' content='text/html; charset=Shift_JIS'>"
                    .as_slice(),
                Some(encoding_rs::SHIFT_JIS),
            ),
            (b"<meta content='text/html; charset=windows-1252'>", None),
            (b"<meta charset='utf-16be'>", Some(UTF_8)),
            (b"<meta charset='x-user-defined'>", Some(WINDOWS_1252)),
            (
                b"<!--<meta charset=windows-1252>--><meta charset=utf-8>",
                Some(UTF_8),
            ),
            (
                b"<a title='<meta charset=windows-1252>'><meta charset=utf-8>",
                Some(UTF_8),
            ),
            (
                b"<meta charset=unknown><meta charset=windows-1252>",
                Some(WINDOWS_1252),
            ),
            (b"<meta charset=utf-8 charset=windows-1252>", Some(UTF_8)),
            (b"<?xml encoding='windows-1252'?><p>x", Some(WINDOWS_1252)),
            (b"<?xml encoding='utf-16'?><p>x", Some(UTF_8)),
        ] {
            assert_eq!(prescan(bytes), expected, "{bytes:?}");
        }
        let late = format!("{}<meta charset=windows-1252>", " ".repeat(1024));
        assert_eq!(prescan(&late.as_bytes()[..1024]), None);
    }
    #[test]
    fn malformed_offsets_cross_chunks_and_bom_precedes_the_caller_label() {
        let limits = Limits::default();
        let mut bad = vec![b'a'; CHUNK_BYTES - 1];
        bad.extend_from_slice(&[0xe2, 0x82, b'x']);
        let error = Document::parse(Source::new(&bad), &limits).unwrap_err();
        assert_eq!(error.offset, Some(CHUNK_BYTES - 1));
        let bytes = [0xff, 0xfe, b'x', 0, 0, 0xd8];
        let error = Document::parse(Source::new(&bytes), &limits).unwrap_err();
        assert_eq!(error.encoding.as_deref(), Some("utf-16le"));
        assert_eq!(error.offset, Some(4));
        let mut source = Source::new(b"\xef\xbb\xbf<title>ok</title>");
        source.encoding = Some("klingon");
        assert!(Document::parse(source, &limits).is_ok());
    }
    #[test]
    fn document_base_uses_only_the_first_href_and_template_fragments_are_visible() {
        let mut source = Source::new(b"<base href='../cdn/'><base href='https://wrong.test/'><template><p>hidden</p></template>");
        source.base_url = Some("https://example.test/app/page");
        let document = Document::parse(source, &Limits::default()).unwrap();
        assert_eq!(
            document.base_url.unwrap().as_str(),
            "https://example.test/cdn/"
        );
        let template = document
            .html
            .select(&scraper::Selector::parse("template").unwrap())
            .next()
            .unwrap();
        assert!(matches!(
            template.first_child().unwrap().value(),
            scraper::Node::Fragment
        ));
        assert_eq!(template.text().collect::<String>(), "hidden");
        let mut source = Source::new(
            b"<template><base href='https://wrong.test/'></template><base href='../cdn/'>",
        );
        source.base_url = Some("https://example.test/app/page");
        assert_eq!(
            Document::parse(source, &Limits::default())
                .unwrap()
                .base_url
                .unwrap()
                .as_str(),
            "https://example.test/cdn/"
        );
    }
}
