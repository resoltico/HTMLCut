// SPDX-License-Identifier: MPL-2.0
//! Independent CommonMark interpretation of semantic inline and Unicode reading.

use htmlcut_core::*;
use pulldown_cmark::{Event, Parser, Tag};
use serde_json::json;

fn read(source: &str, selector: &str, normalized: bool) -> Result<String, ExtractionError> {
    let mut plan = ExtractionPlan::css(selector)?;
    plan.read = Some(if normalized {
        Reading::Text
    } else {
        Reading::Markdown
    });
    let document = PreparedDocument::new(
        SourceSnapshot::new(source, SnapshotMetadata::default())?,
        PreparationLimits::default(),
    )?;
    Ok(document
        .execute(&CompiledPlan::compile(&plan)?)?
        .data()
        .as_values()
        .unwrap()[0]
        .clone())
}

#[test]
fn inline_roles_preserve_punctuation_adjacency_and_code() {
    for (source, expected) in [
        (
            "<p>Use <code>re.sub()</code>, <strong>bold</strong> and <em>emphasis</em>.</p>",
            "Use `re.sub()`, <strong>bold</strong> and <em>emphasis</em>.",
        ),
        ("<p>x<em>!</em>y</p>", "x<em>\\!</em>y"),
        (
            "<p><strong>A <a href='next'><em>B</em> C</a></strong></p>",
            "<strong>A [<em>B</em> C](<next>)</strong>",
        ),
        ("<p><em>A</em><em>B</em>C</p>", "<em>AB</em>C"),
        (
            "<p><strong><strong>x</strong></strong></p>",
            "<strong>x</strong>",
        ),
    ] {
        assert_eq!(read(source, "p", false).unwrap(), expected);
    }
}

#[test]
fn inline_code_chooses_safe_delimiters_and_preserves_normative_spaces() {
    for payload in [
        "x", "`", "y`", "````", " a ", " a", "  ", " \t ", " \u{a0} ", "a\nb", "&copy;", "[x](y)",
    ] {
        let escaped = payload.replace('&', "&amp;").replace('<', "&lt;");
        let md = read(&format!("<p><code>{escaped}</code></p>"), "p", false).unwrap();
        let codes = Parser::new(&md)
            .filter_map(|event| {
                if let Event::Code(value) = event {
                    Some(value.to_string())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        assert_eq!(codes, vec![payload.replace('\n', " ")], "{md}");
    }
    assert_eq!(read("<p>A<code></code>B</p>", "p", false).unwrap(), "AB");
}

#[test]
fn selected_fragments_inherit_original_inline_and_pre_context() {
    for (source, whole, expected) in [
        (
            "<p><code><em><span id=x>!</span></em></code></p>",
            "p",
            "`!`",
        ),
        (
            "<p><code><strong><em><span id=x>x</span></em></strong></code></p>",
            "p",
            "`x`",
        ),
        (
            "<p><strong><code><em><span id=x>x</span></em></code></strong></p>",
            "p",
            "<strong>`x`</strong>",
        ),
        (
            "<p><em><code><strong><code><span id=x>x</span></code></strong></code></em></p>",
            "p",
            "<em>`x`</em>",
        ),
        (
            "<pre><strong><code><em><span id=x>x</span></em></code></strong></pre>",
            "pre",
            "```\nx\n```",
        ),
    ] {
        assert_eq!(read(source, whole, false).unwrap(), expected);
        assert_eq!(read(source, "#x", false).unwrap(), expected);
    }
    assert_eq!(read("<pre class='language-invalid!'><pre class=language-rust><span id=x>x</span></pre></pre>", "#x", false).unwrap(), "```rust\nx\n```");
    assert_eq!(
        read("<em><strong><span id=x>!</span></strong></em>", "#x", false).unwrap(),
        "<em><strong>\\!</strong></em>"
    );
    let md = read(
        "<code>OUTSIDE<span id=x> `x` </span>TAIL</code>",
        "#x",
        false,
    )
    .unwrap();
    assert_eq!(
        Parser::new(&md)
            .filter_map(|e| if let Event::Code(v) = e {
                Some(v.to_string())
            } else {
                None
            })
            .collect::<Vec<_>>(),
        vec![" `x` "]
    );
    let md = read(
        "<pre class=language-python><code><span id=x> x\n</span></code></pre>",
        "#x",
        false,
    )
    .unwrap();
    assert!(md.starts_with("```python\n"));
    let contents = Parser::new(&md)
        .filter_map(|e| {
            if let Event::Text(v) = e {
                Some(v.to_string())
            } else {
                None
            }
        })
        .collect::<String>();
    assert_eq!(contents, " x\n\n");
}

#[test]
fn decoded_character_references_remain_literal_in_text_and_destinations() {
    let md = read("<p>&amp;copy; &amp;#x41; &amp;amp;</p>", "p", false).unwrap();
    let text = Parser::new(&md)
        .filter_map(|e| {
            if let Event::Text(v) = e {
                Some(v.to_string())
            } else {
                None
            }
        })
        .collect::<String>();
    assert_eq!(text, "&copy; &#x41; &amp;");
    let md = read(
        "<p><a href='x&amp;amp;copy;'>x</a><img src='i&amp;amp;copy;' alt='&amp;copy;'></p>",
        "p",
        false,
    )
    .unwrap();
    let mut targets = Vec::new();
    let mut alternative = String::new();
    for event in Parser::new(&md) {
        match event {
            Event::Start(Tag::Link { dest_url, .. })
            | Event::Start(Tag::Image { dest_url, .. }) => targets.push(dest_url.to_string()),
            Event::Text(value) => alternative.push_str(&value),
            _ => (),
        }
    }
    assert_eq!(targets, vec!["x&amp;copy;", "i&amp;copy;"]);
    assert_eq!(alternative, "x&copy;");
}

#[test]
fn explicit_language_metadata_is_finite_unambiguous_and_namespace_aware() {
    assert_eq!(
        read("<svg><text id=x>A</text></svg>", "#x", false).unwrap(),
        "A"
    );
    assert_eq!(read("<pre><svg class='language-invalid!'><text>x</text></svg><code class=language-rust>y</code></pre>", "pre", false).unwrap(), "```rust\nxy\n```");
    for language in ["r_+.-1".to_string(), "a".repeat(64)] {
        let source = format!("<pre class='ordinary language-{language}'>x</pre>");
        assert_eq!(
            read(&source, "pre", false).unwrap(),
            format!("```{language}\nx\n```")
        );
    }
    for language in ["a".repeat(65), "-rust".into(), "é".into(), "a/b".into()] {
        let source = format!("<pre class='language-{language}'>x</pre>");
        assert_eq!(
            read(&source, "pre", false).unwrap_err().code,
            ErrorCode::InvalidRepresentation
        );
    }
    for source in [
        "<pre class='language-python'><code>x</code></pre>",
        "<pre><code class='language-python language-python'>x</code></pre>",
    ] {
        assert!(
            read(source, "pre", false)
                .unwrap()
                .starts_with("```python\n")
        );
    }
    for source in [
        "<pre class='language-'><code>x</code></pre>",
        "<pre class='language-python'><code class='language-rust'>x</code></pre>",
        "<pre class='language-`x'>x</pre>",
    ] {
        assert_eq!(
            read(source, "pre", false).unwrap_err().code,
            ErrorCode::InvalidRepresentation
        );
    }
    assert_eq!(
        read(
            "<p><svg><a href=next><text>x</text></a></svg></p>",
            "p",
            false
        )
        .unwrap(),
        "x"
    );
}

#[test]
fn inline_code_annotations_preserve_literal_payload_and_decoded_metadata() {
    let markdown = read(
        "<p><code><a href='x&amp;amp;'>A</a><img src='i&amp;amp;' alt='&amp;copy;'></code></p>",
        "p",
        false,
    )
    .unwrap();
    let mut code = Vec::new();
    let mut destinations = Vec::new();
    let mut labels = String::new();
    for event in Parser::new(&markdown) {
        match event {
            Event::Code(value) => code.push(value.to_string()),
            Event::Start(Tag::Link { dest_url, .. } | Tag::Image { dest_url, .. }) => {
                destinations.push(dest_url.to_string())
            }
            Event::Text(value) => labels.push_str(&value),
            _ => (),
        }
    }
    assert_eq!(code, ["A"]);
    assert_eq!(destinations, ["x&amp;", "i&amp;"]);
    assert!(labels.contains("link"));
    assert!(labels.contains("&copy;"));
    assert_eq!(
        read("<p><code><img alt='empty'></code></p>", "p", false).unwrap(),
        "empty"
    );
}

#[test]
fn explicit_unicode_normalization_trims_only_unprotected_generated_space() {
    // Independent Unicode White_Space property set, rather than the implementation predicate.
    for point in [
        0x0009, 0x000a, 0x000b, 0x000c, 0x000d, 0x0020, 0x0085, 0x00a0, 0x1680, 0x2000, 0x2001,
        0x2002, 0x2003, 0x2004, 0x2005, 0x2006, 0x2007, 0x2008, 0x2009, 0x200a, 0x2028, 0x2029,
        0x202f, 0x205f, 0x3000,
    ] {
        let space = char::from_u32(point).unwrap();
        let source = format!("<p>{space}a{space}<span>{space}b</span>{space}</p>");
        assert_eq!(read(&source, "p", true).unwrap(), "a b", "U+{point:04X}");
    }
    for (source, expected) in [
        ("<p>\u{a0}World\u{2003}</p>", "World"),
        ("<div> <pre>  code  </pre> </div>", "  code  "),
        ("<div>a <pre></pre>  b</div>", "a b"),
        ("<p>\u{200b}x\u{200b}</p>", "\u{200b}x\u{200b}"),
    ] {
        assert_eq!(read(source, "p, div", true).unwrap(), expected);
    }
    let plan = json!({"version":7,"select":"p","match":"one","read":"literal"});
    let doc = PreparedDocument::new(
        SourceSnapshot::new("<p>\u{a0}World\u{2003}</p>", SnapshotMetadata::default()).unwrap(),
        PreparationLimits::default(),
    )
    .unwrap();
    let query = CompiledPlan::compile(
        &ExtractionPlan::from_json(&serde_json::to_vec(&plan).unwrap()).unwrap(),
    )
    .unwrap();
    assert_eq!(
        doc.execute(&query).unwrap().data().as_values().unwrap(),
        &["\u{a0}World\u{2003}"]
    );
}

#[test]
fn inline_atoms_and_empty_links_keep_following_block_and_prose_boundaries() {
    for (source, expected) in [
        ("<div><a href=next></a><p>Y</p></div>", "[](<next>)\n\nY"),
        ("<div><code>x</code>8. y</div>", "`x`8. y"),
        ("<div><img src=x alt=''> 8. y</div>", "![](<x>) 8. y"),
        (
            "<div><a href=next><em>A</em></a>B</div>",
            "[<em>A</em>](<next>)B",
        ),
        (
            "<div><ul><li>A<code><a href=next>B</a></code>C</li><li>D</li></ul></div>",
            "- A`B` [link](<next>)C\n- D",
        ),
    ] {
        assert_eq!(read(source, "div", false).unwrap(), expected, "{source}");
    }
}
