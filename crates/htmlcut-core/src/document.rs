// SPDX-License-Identifier: MPL-2.0
//! Byte decoding and incremental upstream tree admission.
use crate::dom::{Html, TreeBuilder};
use crate::{Error, ParseOptions, stop::Clock};
use encoding_rs::{
    DecoderResult, EncoderResult, Encoding, UTF_8, UTF_16BE, UTF_16LE, WINDOWS_1252, X_USER_DEFINED,
};
use html5ever::{
    driver,
    tendril::{StrTendril, TendrilSink},
};
use std::borrow::Cow;
use url::Url;

const CHUNK_BYTES: usize = 1_024;
#[cfg(test)]
use crate::bounds::MAX_DOM_DEPTH as MAX_DEPTH;

/// Caller-owned document bytes and optional decoding/base metadata.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
struct Input<'a> {
    /// Exact input bytes.
    bytes: &'a [u8],
    /// A WHATWG encoding label, overridden by a byte order mark.
    encoding: Option<&'a str>,
    /// The document's fallback base URL.
    base_url: Option<&'a str>,
}
/// An immutable admitted document, reusable for contracts and outlines.
#[derive(Debug)]
pub struct Document {
    pub(crate) html: Html,
    pub(crate) positions: std::collections::HashMap<ego_tree::NodeId, (usize, usize)>,
    pub(crate) base_url: Option<Url>,
    pub(crate) query_encoding: &'static Encoding,
}
impl Document {
    /// Decodes without replacement, parses incrementally and admits node/depth limits.
    pub fn parse(
        bytes: &[u8],
        limits: &ParseOptions,
        context: &crate::Stop,
    ) -> Result<Self, Error> {
        let source = Input {
            bytes,
            encoding: limits.encoding.as_deref(),
            base_url: limits.base_url.as_deref(),
        };
        let clock = Clock::for_stop(context);
        clock.check()?;
        if source.bytes.len() > limits.max_input_bytes {
            return Err(Error::LimitExceeded {
                limit: crate::Limit::InputBytes,
                bound: (limits.max_input_bytes).into(),
            });
        }
        let fallback = source
            .base_url
            .map(|base| {
                Url::parse(base).map_err(|e| Error::InvalidBaseUrl {
                    value: base.into(),
                    reason: e.to_string(),
                })
            })
            .transpose()?;
        let (decoded, query_encoding) = decode(source, &clock)?;
        let html = parse_admitted(&decoded, limits, CHUNK_BYTES, &clock)?;
        // The first base with href governs, even when its URL is unusable.
        let mut document_base = None;
        let mut fragment_depth = 0;
        for edge in html.tree.root().traverse() {
            clock.tick()?;
            match edge {
                ego_tree::iter::Edge::Open(node) => {
                    if fragment_depth > 0 || matches!(node.value(), crate::dom::Node::Fragment) {
                        fragment_depth += 1;
                    } else if let Some(e) = crate::dom::ElementRef::wrap(node)
                        && e.value().name.ns == html5ever::ns!(html)
                        && e.value().name() == "base"
                        && let Some(href) = e.value().attr("href")
                    {
                        document_base = resolve(href, fallback.as_ref(), query_encoding)
                            .ok()
                            .filter(|u| !matches!(u.scheme(), "data" | "javascript"));
                        break;
                    }
                }
                ego_tree::iter::Edge::Close(_) if fragment_depth > 0 => fragment_depth -= 1,
                ego_tree::iter::Edge::Close(_) => (),
            }
        }
        let base_url = document_base.or(fallback);
        let mut positions = std::collections::HashMap::new();
        let mut index = 0;
        for edge in html.tree.root().traverse() {
            clock.tick()?;
            match edge {
                ego_tree::iter::Edge::Open(n) => {
                    positions.insert(n.id(), (index, index));
                    index += 1;
                }
                ego_tree::iter::Edge::Close(n) => {
                    positions.get_mut(&n.id()).unwrap().1 = index;
                }
            }
        }
        clock.check()?;
        Ok(Self {
            positions,
            html,
            base_url,
            query_encoding,
        })
    }
}

pub(crate) fn resolve(
    value: &str,
    base: Option<&Url>,
    query_encoding: &'static Encoding,
) -> Result<Url, url::ParseError> {
    let value = value.trim_matches(|c: char| c.is_ascii_whitespace());
    let options = Url::options().base_url(base);
    if query_encoding == UTF_8 {
        return options.parse(value);
    }
    // URL preprocessing removes ASCII tabs/newlines. Do this before the hook:
    // url's query iterator otherwise invokes it on separate slices, which would
    // restart a stateful ISO-2022-JP encoder in the middle of the same query.
    let value = if value.contains(['\t', '\r', '\n']) {
        Cow::Owned(
            value
                .chars()
                .filter(|c| !matches!(c, '\t' | '\r' | '\n'))
                .collect::<String>(),
        )
    } else {
        Cow::Borrowed(value)
    };
    let encode: &dyn Fn(&str) -> Cow<'_, [u8]> =
        &|query| Cow::Owned(encode_url_query(query, query_encoding));
    options.encoding_override(Some(encode)).parse(&value)
}

// https://url.spec.whatwg.org/#string-percent-encode-after-encoding
// Keep one encoder across errors; HTML replacement encoding would leave '&'
// and ';' literal instead of the URL algorithm's percent-escaped NCR fallback.
fn encode_url_query(mut input: &str, encoding: &'static Encoding) -> Vec<u8> {
    let mut encoder = encoding.new_encoder();
    let mut output = Vec::with_capacity(input.len());
    let mut buffer = [0; 1024];
    loop {
        let (result, read, written) =
            encoder.encode_from_utf8_without_replacement(input, &mut buffer, true);
        output.extend_from_slice(&buffer[..written]);
        input = &input[read..];
        match result {
            EncoderResult::InputEmpty => return output,
            EncoderResult::OutputFull => (),
            EncoderResult::Unmappable(character) => {
                // The URL crate's query percent-encode set leaves '%' intact.
                // These ASCII bytes are emitted outside the encoder, preserving
                // its state exactly as encode-or-fail requires.
                output.extend_from_slice(format!("%26%23{}%3B", u32::from(character)).as_bytes());
            }
        }
    }
}

fn parse_admitted(
    text: &str,
    limits: &ParseOptions,
    chunk: usize,
    clock: &Clock,
) -> Result<Html, Error> {
    if limits.max_nodes == 0 {
        return Err(Error::LimitExceeded {
            limit: crate::Limit::Nodes,
            bound: (0).into(),
        });
    }
    let mut parser =
        driver::parse_document(TreeBuilder::new(limits, clock.clone()), Default::default());
    feed(&mut parser, text, chunk, clock)?;
    clock.check()?;
    if let Some(error) = parser.tokenizer.sink.sink.refusal() {
        return Err(error);
    }
    let html = parser.finish()?;
    clock.check()?;
    Ok(html)
}

fn feed(
    parser: &mut html5ever::Parser<TreeBuilder>,
    text: &str,
    chunk: usize,
    clock: &Clock,
) -> Result<(), Error> {
    let mut remaining = text;
    while !remaining.is_empty() {
        clock.check()?;
        let end = utf8_feed_end(remaining, chunk);
        parser.process(StrTendril::from_slice(&remaining[..end]));
        remaining = &remaining[end..];
        if let Some(error) = parser.tokenizer.sink.sink.refusal() {
            return Err(error);
        }
    }
    Ok(())
}

fn utf8_feed_end(text: &str, maximum: usize) -> usize {
    let mut end = maximum.min(text.len());
    // The caller has nonempty text. A split stays within the byte ceiling,
    // except test feeds too small to contain their first Unicode scalar.
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    if end == 0 {
        end = text.chars().next().unwrap().len_utf8();
    }
    end
}

fn decode(source: Input<'_>, clock: &Clock) -> Result<(String, &'static Encoding), Error> {
    let (encoding, skip) = if let Some(bom) = Encoding::for_bom(source.bytes) {
        bom
    } else if let Some(label) = source.encoding {
        (
            Encoding::for_label(label.as_bytes()).ok_or_else(|| Error::InvalidEncoding {
                label: label.into(),
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
                .ok_or_else(|| Error::LimitExceeded {
                    limit: crate::Limit::InputBytes,
                    bound: (source.bytes.len()).into(),
                })?,
        );
        let (result, read) = decoder.decode_to_string_without_replacement(input, &mut output, last);
        consumed += read;
        match result {
            DecoderResult::InputEmpty if last => return Ok((output, encoding.output_encoding())),
            DecoderResult::InputEmpty | DecoderResult::OutputFull => (),
            DecoderResult::Malformed(length, after) => {
                let offset = skip + consumed - usize::from(length) - usize::from(after);
                return Err(Error::UndecodableInput {
                    encoding: encoding.name(),
                    offset,
                });
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

enum PrescanCharset {
    Unset,
    Encoding(&'static Encoding),
    Failed,
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
            let mut charset = PrescanCharset::Unset;
            for (name, value) in attrs {
                if !seen.insert(name.clone()) {
                    continue;
                }
                match name.as_slice() {
                    b"http-equiv" => got_pragma = value == b"content-type",
                    b"content" if matches!(charset, PrescanCharset::Unset) => {
                        if let Some(found) = content_encoding(&value) {
                            charset = PrescanCharset::Encoding(found);
                            need_pragma = Some(true);
                        }
                    }
                    b"charset" => {
                        charset = Encoding::for_label(&value)
                            .map(PrescanCharset::Encoding)
                            .unwrap_or(PrescanCharset::Failed);
                        need_pragma = Some(false);
                    }
                    _ => (),
                }
            }
            if (need_pragma == Some(false) || (need_pragma == Some(true) && got_pragma))
                && let PrescanCharset::Encoding(encoding) = charset
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
    fn decode(
        input: crate::test_support::DocumentFixture<'_>,
        clock: &crate::stop::Clock,
    ) -> Result<(String, &'static encoding_rs::Encoding), crate::Error> {
        super::decode(
            super::Input {
                bytes: input.bytes,
                encoding: input.encoding,
                base_url: input.base_url,
            },
            clock,
        )
    }

    use super::*;
    use crate::ErrorCode;
    #[allow(unused_imports)]
    use crate::{
        stop::Clock,
        test_support::{DocumentFixture, TestOptions, parse_fixture},
    };
    #[test]
    fn utf8_feeds_respect_the_byte_ceiling_without_losing_text() {
        for scalar in ["é", "€", "🦀"] {
            let text = format!("{}{scalar}tail", "x".repeat(1023));
            // Each scalar straddles the documented 1024-byte feed ceiling.
            // Completing it forwards would admit 1025, 1026 or 1027 bytes.
            assert_eq!(utf8_feed_end(&text, CHUNK_BYTES), 1023);
            let mut remaining = text.as_str();
            let mut reconstructed = String::new();
            while !remaining.is_empty() {
                let end = utf8_feed_end(remaining, CHUNK_BYTES);
                assert!(end > 0 && end <= 1024);
                reconstructed.push_str(&remaining[..end]);
                remaining = &remaining[end..];
            }
            assert_eq!(reconstructed, text);
        }
        // Existing parser tests deliberately use feeds smaller than a scalar.
        for (text, expected) in [("éx", 2), ("€x", 3), ("🦀x", 4)] {
            assert_eq!(utf8_feed_end(text, 1), expected);
        }
    }

    #[test]
    fn strict_decode_retains_only_a_usable_query_output_encoding() {
        for label in ["utf-16le", "utf-16be", "iso-2022-kr"] {
            let mut source = DocumentFixture::new(b"");
            source.encoding = Some(label);
            let clock = Clock::new(TestOptions::default().time_limit).unwrap();
            let (text, encoding) = decode(source, &clock).unwrap();
            assert!(text.is_empty());
            assert_eq!(encoding, UTF_8, "{label}");
        }
        let mut source = DocumentFixture::new(b"");
        source.encoding = Some("windows-1252");
        assert_eq!(
            decode(
                source,
                &Clock::new(TestOptions::default().time_limit).unwrap()
            )
            .unwrap()
            .1,
            WINDOWS_1252
        );
    }

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
        let limits = TestOptions {
            time_limit: None,
            ..TestOptions::default()
        };
        for document in documents {
            let one = Html::parse_document(&document);
            for chunk in [1, 7, CHUNK_BYTES] {
                let many = parse_admitted(
                    &document,
                    &limits.preparation(),
                    chunk,
                    &Clock::new(limits.time_limit).unwrap(),
                )
                .unwrap();
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
        let limits = TestOptions {
            time_limit: None,
            ..TestOptions::default()
        };
        assert!(input.len() <= limits.max_input_bytes);
        let mut parser = driver::parse_document(
            TreeBuilder::new(
                &limits.preparation(),
                Clock::new(limits.time_limit).unwrap(),
            ),
            Default::default(),
        );
        let error = feed(
            &mut parser,
            &input,
            CHUNK_BYTES,
            &Clock::new(limits.time_limit).unwrap(),
        )
        .unwrap_err();
        assert_eq!(
            serde_json::to_value(&error).unwrap()["limit"].as_str(),
            Some("nodes")
        );
        assert_eq!(
            serde_json::to_value(&error).unwrap().get("bound").cloned(),
            Some(limits.max_nodes.into())
        );
        let observed = parser.tokenizer.sink.sink.node_count();
        assert_eq!(observed, limits.max_nodes);
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
            let result = parse_fixture(
                DocumentFixture::new(html.as_bytes()),
                &TestOptions::default().preparation(),
                &crate::stop::Clock::new(TestOptions::default().time_limit).unwrap(),
            );
            if depth == MAX_DEPTH {
                assert!(result.is_ok());
            } else {
                assert_eq!(
                    serde_json::to_value(result.unwrap_err()).unwrap()["limit"].as_str(),
                    Some("depth")
                );
            }
        }
        let input = "<p></p><frameset>";
        let parsed = Html::parse_document(input);
        let attached = parsed.tree.root().descendants().count();
        assert!(parsed.tree.nodes().len() > attached);
        let limits = TestOptions {
            max_nodes: attached,
            ..TestOptions::default()
        };
        assert_eq!(
            serde_json::to_value(
                parse_fixture(
                    DocumentFixture::new(input.as_bytes()),
                    &limits.preparation(),
                    &crate::stop::Clock::new(limits.time_limit).unwrap()
                )
                .unwrap_err()
            )
            .unwrap()["limit"]
                .as_str(),
            Some("nodes")
        );
    }
    #[test]
    fn standard_prescan_handles_pragmas_substitutions_boundaries_and_false_declarations() {
        for (bytes, expected) in [
            (b"<meta charset='' http-equiv='content-type' content='text/html; charset=koi8-r'>".as_slice(), None),
            (b"<meta charset=bogus http-equiv='content-type' content='text/html; charset=koi8-r'>".as_slice(), None),
            (b"<meta content='text/html; charset=koi8-r' http-equiv='content-type' charset=bogus>".as_slice(), None),

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
        let limits = TestOptions::default();
        let mut bad = vec![b'a'; CHUNK_BYTES - 1];
        bad.extend_from_slice(&[0xe2, 0x82, b'x']);
        let error = parse_fixture(
            DocumentFixture::new(&bad),
            &limits.preparation(),
            &crate::stop::Clock::new(limits.time_limit).unwrap(),
        )
        .unwrap_err();
        assert_eq!(
            serde_json::to_value(&error)
                .unwrap()
                .get("offset")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize),
            Some(CHUNK_BYTES - 1)
        );
        let bytes = [0xff, 0xfe, b'x', 0, 0, 0xd8];
        let error = parse_fixture(
            DocumentFixture::new(&bytes),
            &limits.preparation(),
            &crate::stop::Clock::new(limits.time_limit).unwrap(),
        )
        .unwrap_err();
        assert_eq!(
            serde_json::to_value(&error).unwrap()["encoding"].as_str(),
            Some("utf-16le")
        );
        assert_eq!(
            serde_json::to_value(&error)
                .unwrap()
                .get("offset")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize),
            Some(4)
        );
        let mut source = DocumentFixture::new(b"\xef\xbb\xbf<title>ok</title>");
        source.encoding = Some("klingon");
        assert!(
            parse_fixture(
                source,
                &limits.preparation(),
                &crate::stop::Clock::new(limits.time_limit).unwrap()
            )
            .is_ok()
        );
    }
    #[test]
    fn document_base_uses_only_the_first_href_and_template_fragments_are_visible() {
        let mut source = DocumentFixture::new(b"<base href='../cdn/'><base href='https://wrong.test/'><template><p>hidden</p></template>");
        source.base_url = Some("https://example.test/app/page");
        let document = parse_fixture(
            source,
            &TestOptions::default().preparation(),
            &crate::stop::Clock::new(TestOptions::default().time_limit).unwrap(),
        )
        .unwrap();
        assert_eq!(
            document.base_url.unwrap().as_str(),
            "https://example.test/cdn/"
        );
        let template = document
            .html
            .select(&crate::select::Selector::parse("template").unwrap())
            .next()
            .unwrap();
        assert!(matches!(
            template.first_child().unwrap().value(),
            crate::dom::Node::Fragment
        ));
        assert_eq!(template.text().collect::<String>(), "hidden");
        let mut source = DocumentFixture::new(
            b"<template><base href='https://wrong.test/'></template><base href='../cdn/'>",
        );
        source.base_url = Some("https://example.test/app/page");
        assert_eq!(
            parse_fixture(
                source,
                &TestOptions::default().preparation(),
                &crate::stop::Clock::new(TestOptions::default().time_limit).unwrap()
            )
            .unwrap()
            .base_url
            .unwrap()
            .as_str(),
            "https://example.test/cdn/"
        );
    }

    #[test]
    fn prescan_distinguishes_real_declarations_from_tag_and_comment_contents() {
        for (bytes, expected) in [
            (
                b"<!-- --x> --> <meta charset=windows-1252>".as_slice(),
                Some(WINDOWS_1252),
            ),
            (b"<!--<meta charset=windows-1252>-->", None),
            (b"<!--<meta charset=windows-1252>", None),
            (b"<!-- -->", None),
            (b"<!-->", None),
            (b"<!--><meta charset=windows-1252>", Some(WINDOWS_1252)),
            (b"<!---><meta charset=windows-1252>", Some(WINDOWS_1252)),
            (b"<meta", None),
            (b"<META", None),
            (b"<meta/charset=windows-1252>", Some(WINDOWS_1252)),
            (b"<meta! charset=windows-1252>", None),
            (b"<metadata charset=windows-1252>", None),
            (
                b"<x a=\"<meta charset=windows-1252>\"><meta charset=utf-8>",
                Some(UTF_8),
            ),
            (
                b"</x a=\"<meta charset=windows-1252>\"><meta charset=utf-8>",
                Some(UTF_8),
            ),
            (b"<!fake <meta charset=windows-1252>>", None),
            (b"<?fake?><meta charset=windows-1252>", Some(WINDOWS_1252)),
            (b"<1 a='>' <meta charset=windows-1252>>", Some(WINDOWS_1252)),
            (
                b"<?x a='>' <meta charset=windows-1252>>",
                Some(WINDOWS_1252),
            ),
            (
                b"</1 a='>' <meta charset=windows-1252>>",
                Some(WINDOWS_1252),
            ),
            (
                b"<!x a='>' <meta charset=windows-1252>>",
                Some(WINDOWS_1252),
            ),
            (b"x<meta charset=windows-1252>", Some(WINDOWS_1252)),
            (b"xx<meta charset=windows-1252>", Some(WINDOWS_1252)),
            (b"<x='>'<meta charset=windows-1252>>", Some(WINDOWS_1252)),
            (
                b"<?<meta charset=windows-1252>><meta charset=utf-8>",
                Some(UTF_8),
            ),
            (
                b"<!<meta charset=windows-1252>><meta charset=utf-8>",
                Some(UTF_8),
            ),
            (
                b"</1<meta charset=windows-1252>><meta charset=utf-8>",
                Some(UTF_8),
            ),
            (b"</1fake><meta charset=windows-1252>", Some(WINDOWS_1252)),
            (b"<x><meta charset=windows-1252>", Some(WINDOWS_1252)),
            (
                b"<x a = ' <meta charset=windows-1252> '><meta charset=utf-8>",
                Some(UTF_8),
            ),
            (
                b"<x a=<meta charset=windows-1252>><meta charset=utf-8>",
                Some(UTF_8),
            ),
            (b"<meta / / charset \t=\n windows-1252>", Some(WINDOWS_1252)),
            (b"<meta =charset=windows-1252>", None),
            (b"<meta a='' charset=\"windows-1252\">", Some(WINDOWS_1252)),
            (b"<meta a='x'charset=windows-1252>", Some(WINDOWS_1252)),
            (b"<meta a=\"x\"charset=windows-1252>", Some(WINDOWS_1252)),
            (b"<meta x/charset=windows-1252>", Some(WINDOWS_1252)),
            (b"<meta charset=windows-1252 ignored>", Some(WINDOWS_1252)),
            (b"<meta a='unterminated", None),
            (b"<meta charset", None),
            (b"<meta charset=", None),
            (b"<meta a>", None),
            (b"<x", None),
            (b"</x", None),
            (b"<!x", None),
            (b"<?x", None),
        ] {
            assert_eq!(prescan(bytes), expected, "{bytes:?}");
        }
    }

    #[test]
    fn content_charset_extraction_obeys_pragma_order_and_label_boundaries() {
        for (bytes, expected) in [
            (b"<meta http-equiv=content-type content='charset=windows-1252' content='charset=utf-8'>".as_slice(), Some(WINDOWS_1252)),
            (b"<meta http-equiv=content-type content='charset=utf-8' charset=windows-1252>", Some(WINDOWS_1252)),
            (b"<meta http-equiv=content-type charset=windows-1252 content='charset=utf-8'>", Some(WINDOWS_1252)),
            (b"<meta http-equiv=other content='charset=windows-1252'>", None),
            (b"<meta http-equiv=content-type content='charset ignored; charset = windows-1252;'>", Some(WINDOWS_1252)),
            (b"<meta http-equiv=content-type content=\"charset = 'windows-1252'\">", Some(WINDOWS_1252)),
            (b"<meta http-equiv=content-type content='charset = \"windows-1252\"'>", Some(WINDOWS_1252)),
            (b"<meta http-equiv=content-type content='charset=unknown; charset=windows-1252'>", None),
            (b"<meta http-equiv=content-type content='charset=windows-1252 other'>", Some(WINDOWS_1252)),
            (b"<meta http-equiv=content-type content='charset'>", None),
            (b"<meta http-equiv=content-type content='charset='>", None),
            (b"<meta http-equiv=content-type content=\"charset='windows-1252\">", None),
        ] {
            // Decoding admission is independent of parser construction.
            assert_eq!(prescan(bytes), expected, "{bytes:?}");
        }
    }

    #[test]
    fn xml_encoding_is_a_fallback_with_strict_declaration_and_label_syntax() {
        for (bytes, expected) in [
            (
                b"<?xml encoding \t= \r'windows-1252'?>".as_slice(),
                Some(WINDOWS_1252),
            ),
            (b"<?xml encoding=\"windows-1252\"?>", Some(WINDOWS_1252)),
            (b"<?xml encoding=windows-1252?>", None),
            (b"<?xml encoding='windows-1252 '?>", None),
            (b"<?XML encoding='windows-1252'?>", None),
            (b"<?xml encoding:'windows-1252'?>", None),
            (b"<?xml encoding='unknown'?>", None),
            (b"<?xml?>", None),
            (b"<?xml encoding='windows-1252", None),
            (b"<?xml encoding='windows-1252>", None),
            (b"<?xml encoding='utf-16le'?>", Some(UTF_8)),
            (b"<?xml encoding='utf-16be'?>", Some(UTF_8)),
            (
                b"<?xml encoding='windows-1252'?><meta charset=utf-8>",
                Some(UTF_8),
            ),
            (b"<\0?\0x\0", Some(UTF_16LE)),
            (b"\0<\0?\0x", Some(UTF_16BE)),
        ] {
            assert_eq!(prescan(bytes), expected, "{bytes:?}");
        }
    }

    #[test]
    fn closed_body_elements_do_not_make_template_base_urls_active() {
        let mut source = DocumentFixture::new(b"<body><p>before</p><template><base href='https://wrong.test/'></template><base href='../cdn/'><a href='item'>x</a>");
        source.base_url = Some("https://example.test/app/page");
        let document = parse_fixture(
            source,
            &TestOptions::default().preparation(),
            &crate::stop::Clock::new(TestOptions::default().time_limit).unwrap(),
        )
        .unwrap();
        let contract = crate::Contract::from_json(
            r#"{"select":"a","read":"url:href"}"#,
            &crate::stop::Clock::new(None).unwrap(),
        )
        .unwrap();
        assert_eq!(
            document
                .extract(
                    &contract,
                    crate::ExportFormat::Json,
                    &crate::test_support::TestOptions::default().execution(),
                    &crate::stop::Clock::new(
                        (crate::test_support::TestOptions::default()).time_limit
                    )
                    .unwrap()
                )
                .unwrap(),
            crate::Value::String("https://example.test/cdn/item".into())
        );
    }

    #[test]
    fn chunk_node_admission_accepts_equality_and_refuses_larger_trees() {
        // Document, html, head, body and p are five arena nodes. One chunk
        // crosses both refused bounds before the incremental guard runs.
        let input = "<html><head></head><body><p></p></body></html>";
        for maximum in [5, 4, 3] {
            let limits = TestOptions {
                max_nodes: maximum,
                time_limit: None,
                ..TestOptions::default()
            };
            let mut parser = driver::parse_document(
                TreeBuilder::new(
                    &limits.preparation(),
                    Clock::new(limits.time_limit).unwrap(),
                ),
                Default::default(),
            );
            let result = feed(
                &mut parser,
                input,
                input.len(),
                &Clock::new(limits.time_limit).unwrap(),
            );
            assert_eq!(parser.tokenizer.sink.sink.node_count(), maximum);
            if maximum == 5 {
                result.unwrap();
            } else {
                let error = result.unwrap_err();
                assert_eq!(error.code(), ErrorCode::LimitExceeded);
                assert_eq!(
                    serde_json::to_value(&error).unwrap()["limit"].as_str(),
                    Some("nodes")
                );
                assert_eq!(
                    serde_json::to_value(&error).unwrap().get("bound").cloned(),
                    Some(maximum.into())
                );
            }
        }
    }

    #[test]
    fn finishing_node_admission_counts_implicit_nodes_without_input_chunks() {
        // Finishing an empty stream creates html, head and body in addition
        // to the document node; feed performs no incremental node check.
        for maximum in [4, 3, 2] {
            let limits = TestOptions {
                max_nodes: maximum,
                time_limit: None,
                ..TestOptions::default()
            };
            let result = parse_admitted(
                "",
                &limits.preparation(),
                CHUNK_BYTES,
                &Clock::new(limits.time_limit).unwrap(),
            );
            if maximum == 4 {
                assert_eq!(result.unwrap().tree.nodes().len(), 4);
            } else {
                let error = result.unwrap_err();
                assert_eq!(error.code(), ErrorCode::LimitExceeded);
                assert_eq!(
                    serde_json::to_value(&error).unwrap()["limit"].as_str(),
                    Some("nodes")
                );
                assert_eq!(
                    serde_json::to_value(&error).unwrap().get("bound").cloned(),
                    Some(maximum.into())
                );
            }
        }
    }

    #[test]
    fn chunk_refusal_keeps_unicode_text_beyond_the_chunk_boundary_unfed() {
        let limits = TestOptions {
            max_nodes: 4,
            time_limit: None,
            ..TestOptions::default()
        };
        let mut parser = driver::parse_document(
            TreeBuilder::new(
                &limits.preparation(),
                Clock::new(limits.time_limit).unwrap(),
            ),
            Default::default(),
        );
        // A four-byte chunk ends inside é. Admission must see only <p>,
        // rather than consuming text past the chunk ceiling before refusing.
        let error = feed(
            &mut parser,
            "<p>é</p><p>later</p>",
            4,
            &Clock::new(limits.time_limit).unwrap(),
        )
        .unwrap_err();
        assert_eq!(
            serde_json::to_value(&error).unwrap()["limit"].as_str(),
            Some("nodes")
        );
        assert_eq!(
            serde_json::to_value(&error).unwrap().get("bound").cloned(),
            Some(4.into())
        );
        assert_eq!(parser.tokenizer.sink.sink.node_count(), 4);
    }

    #[test]
    fn malformed_gb18030_offsets_exclude_reprocessed_trailing_bytes() {
        // These incomplete four-byte sequences designate their first byte
        // as malformed after consuming one or two bytes for reprocessing.
        for bytes in [
            &[b'x', 0x81, b'0', b' '][..],
            &[b'x', 0x81, b'0', 0x81, b' '][..],
        ] {
            let mut source = DocumentFixture::new(bytes);
            source.encoding = Some("gb18030");
            let error = parse_fixture(
                source,
                &TestOptions::default().preparation(),
                &crate::stop::Clock::new(TestOptions::default().time_limit).unwrap(),
            )
            .unwrap_err();
            assert_eq!(error.code(), ErrorCode::UndecodableInput);
            assert_eq!(
                serde_json::to_value(&error).unwrap()["encoding"].as_str(),
                Some("gb18030")
            );
            assert_eq!(
                serde_json::to_value(&error)
                    .unwrap()
                    .get("offset")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as usize),
                Some(1)
            );
        }
    }
}
