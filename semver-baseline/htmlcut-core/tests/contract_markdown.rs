// SPDX-License-Identifier: MPL-2.0
//! Independent complete-format and CommonMark interpretation checks.

use htmlcut_core::*;
use pulldown_cmark::{Event, Parser, Tag};

fn extract(html: &str, css: &str) -> String {
    extract_excluding(html, css, &[])
}

fn extract_excluding(html: &str, css: &str, excluded: &[&str]) -> String {
    let document = PreparedDocument::new(
        SourceSnapshot::new(html, SnapshotMetadata::default()).unwrap(),
        PreparationLimits::default(),
    )
    .unwrap();
    let mut plan = ExtractionPlan::css(css).unwrap();
    plan.projection = Projection::Value(ValueProjection::Markdown {});
    plan.exclude = excluded.iter().map(|selector| (*selector).into()).collect();
    match document
        .execute(&CompiledPlan::compile(&plan).unwrap())
        .unwrap()
        .data
    {
        ExtractionData::Values(mut values) => values.remove(0),
        _ => panic!("expected flat data"),
    }
}

#[test]
fn headings_blocks_and_literal_punctuation_have_no_forged_markup() {
    let value = extract(
        "<article><h2>Title</h2><p> A   [x](y) # *fake* &lt;tag&gt; </p><dl><dt>Name</dt><dd>Alice</dd></dl></article>",
        "article",
    );
    assert_eq!(
        value,
        "## Title\n\nA \\[x\\](y) # \\*fake\\* \\<tag\\>\n\nName\n\nAlice"
    );
    assert_eq!(
        Parser::new(&value)
            .filter(|e| matches!(e, Event::Start(Tag::Link { .. })))
            .count(),
        0
    );
    let text = Parser::new(&value)
        .filter_map(|e| match e {
            Event::Text(t) => Some(t.to_string()),
            _ => None,
        })
        .collect::<String>();
    assert!(text.contains("[x](y) # *fake* <tag>"));
}

#[test]
fn ordered_source_ordinals_are_literal_bullet_labels() {
    let value = extract(
        "<ol start='3'><li>A</li><li value='8'>B</li><li>C</li></ol>",
        "ol",
    );
    assert_eq!(value, "- 3\\. A\n- 8\\. B\n- 9\\. C");
    assert!(Parser::new(&value).any(|e| matches!(e, Event::Start(Tag::List(None)))));
    assert!(!Parser::new(&value).any(|e| matches!(e, Event::Start(Tag::List(Some(_))))));
    assert_eq!(
        extract(
            "<ol reversed start='-2'><li>A</li><li id='x'>B</li></ol>",
            "#x"
        ),
        "- -3\\. B"
    );
}

#[test]
fn every_ragged_cell_is_retained_in_nested_lists() {
    let value = extract(
        "<table><caption>Costs</caption><tr><th>A</th><td>B</td></tr><tr><td>one</td><td>two</td><td>IMPORTANT</td></tr></table>",
        "table",
    );
    assert_eq!(
        value,
        "Costs\n\n-\n  - <strong>A</strong>\n  - B\n-\n  - one\n  - two\n  - IMPORTANT"
    );
    let text = Parser::new(&value)
        .filter_map(|e| match e {
            Event::Text(t) => Some(t.to_string()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(text, ["Costs", "A", "B", "one", "two", "IMPORTANT"]);
    assert_eq!(
        extract(
            "<table><tr><td>X</td><td>OUTSIDE</td></tr></table>",
            "td:first-child"
        ),
        "-\n  - X"
    );
}

#[test]
fn code_payload_is_literal_with_one_structural_lf() {
    for payload in ["", "x", "x\n", "x\n\n", "  x\n    y\n", "```\n~~~~\n"] {
        let value = extract(&format!("<pre>{payload}</pre>"), "pre");
        let code = Parser::new(&value)
            .filter_map(|e| match e {
                Event::Text(t) => Some(t.to_string()),
                _ => None,
            })
            .collect::<String>();
        assert_eq!(code, format!("{payload}\n"), "{value:?}");
    }
    let value = extract("<pre>OUTSIDE<code id='x'> x\n  y\n</code>TAIL</pre>", "#x");
    assert_eq!(value, "```\n x\n  y\n\n```");
}

#[test]
fn links_images_and_preformatted_metadata_keep_destinations() {
    let value = extract(
        "<article><p>Read <a href='next.html'>Guide</a><img src='image.png' alt='Picture'></p><pre><a href='type.html'>Type</a>(x)</pre></article>",
        "article",
    );
    assert_eq!(
        value,
        "Read [Guide](<next.html>)![Picture](<image.png>)\n\n```\nType(x)\n```\n\n- [Type](<type.html>)"
    );
    let destinations = Parser::new(&value)
        .filter_map(|e| match e {
            Event::Start(Tag::Link { dest_url, .. })
            | Event::Start(Tag::Image { dest_url, .. }) => Some(dest_url.to_string()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(destinations, ["next.html", "image.png", "type.html"]);
}

#[test]
fn hidden_content_is_retained_but_script_and_template_payloads_are_excluded() {
    assert_eq!(
        extract(
            "<p>A<span hidden>B</span><script>SECRET</script><template>SECRET</template><style>SECRET</style>C</p>",
            "p"
        ),
        "ABC"
    );
    assert_eq!(
        extract("<svg><g>A  B</g><script>SECRET</script></svg>", "svg"),
        "A B"
    );
}

#[test]
fn nested_code_cells_parse_as_code_and_keep_every_payload_newline() {
    let value = extract(
        "<table><tr><td><p>Intro</p><pre> x\n</pre></td><td>Z</td></tr></table>",
        "table",
    );
    let parsed = Parser::new(&value).collect::<Vec<_>>();
    assert!(
        parsed
            .iter()
            .any(|e| matches!(e, Event::Start(Tag::CodeBlock(_)))),
        "{value:?}"
    );
    let text = parsed
        .iter()
        .filter_map(|e| match e {
            Event::Text(t) => Some(t.as_ref()),
            _ => None,
        })
        .collect::<String>();
    assert!(text.contains(" x\n\n"), "{text:?}");
    assert!(text.contains("Intro") && text.contains('Z'));
}

#[test]
fn complex_anchor_blocks_and_quotes_have_valid_commonmark_structure() {
    let value = extract("<a href='next'><h2>Title</h2><p>Body</p></a>", "a");
    assert_eq!(value, "## Title\n\nBody\n\n[link](<next>)");
    assert!(Parser::new(&value).any(|e| matches!(e, Event::Start(Tag::Heading { .. }))));
    assert!(Parser::new(&value).any(|e| matches!(e, Event::Start(Tag::Link { .. }))));
    for (source, css) in [
        ("<blockquote><p>A</p><p>B</p></blockquote>", "blockquote"),
        (
            "<table><tr><td><blockquote>Q</blockquote></td><td>Z</td></tr></table>",
            "table",
        ),
    ] {
        let value = extract(source, css);
        assert!(
            Parser::new(&value).any(|e| matches!(e, Event::Start(Tag::BlockQuote(_)))),
            "{value:?}"
        );
    }
}

#[test]
fn original_code_metadata_resolves_urls_without_altering_code_payload() {
    let document=PreparedDocument::new(SourceSnapshot::new(
        "<article><p><img src='photo.png' alt='Photo'></p><pre><a href='next'><span> Name </span><span class='omit'>DROP</span><script>DROP</script></a><img src='photo.png' alt='Photo'><img src='empty.png'></pre></article>",
        SnapshotMetadata{base_url:Some("https://example.test/path/".into())}).unwrap(),Default::default()).unwrap();
    let mut plan = ExtractionPlan::css("article").unwrap();
    plan.projection = Projection::Value(ValueProjection::Markdown {});
    plan.transforms = vec![Transform::ResolveUrls {}];
    plan.exclude = vec![".omit".into()];
    let result = document
        .execute(&CompiledPlan::compile(&plan).unwrap())
        .unwrap();
    let value = &result.data.as_values().unwrap()[0];
    let destinations = Parser::new(value)
        .filter_map(|e| match e {
            Event::Start(Tag::Link { dest_url, .. })
            | Event::Start(Tag::Image { dest_url, .. }) => Some(dest_url.to_string()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        destinations,
        [
            "https://example.test/path/photo.png",
            "https://example.test/path/next",
            "https://example.test/path/photo.png",
            "https://example.test/path/empty.png"
        ]
    );
    assert!(!value.contains("DROP"));
    assert!(value.contains("```\n Name \n```"), "{value:?}");
}

#[test]
fn unrepresentable_destinations_fail_and_literal_destination_escapes_roundtrip() {
    for destination in ["x\ny", "x\ry", "x&#0;y"] {
        let source = format!("<a href='{destination}'>label</a>");
        let document = PreparedDocument::new(
            SourceSnapshot::new(source, Default::default()).unwrap(),
            Default::default(),
        )
        .unwrap();
        let mut plan = ExtractionPlan::css("a").unwrap();
        plan.projection = Projection::Value(ValueProjection::Markdown {});
        let outcome = document.execute(&CompiledPlan::compile(&plan).unwrap());
        if destination != "x&#0;y" {
            assert_eq!(outcome.unwrap_err().code, ErrorCode::InvalidRepresentation);
        } else {
            assert!(outcome.is_ok());
        } // HTML parsing repairs NUL to the replacement character.
    }
    let value = extract("<a href='x&lt;y&gt;\\z'>label</a>", "a");
    let targets = Parser::new(&value)
        .filter_map(|e| match e {
            Event::Start(Tag::Link { dest_url, .. }) => Some(dest_url.to_string()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(targets, ["x<y>\\z"]);
}

#[test]
fn inline_anchor_analysis_preserves_foreign_text_and_skips_excluded_blocks() {
    let source = "<a href='next'><script>SECRET</script><span class='omit'><b>DROP</b></span><svg><g>Y</g></svg><span> Z</span></a>";
    let document = PreparedDocument::new(
        SourceSnapshot::new(source, Default::default()).unwrap(),
        Default::default(),
    )
    .unwrap();
    let mut plan = ExtractionPlan::css("a").unwrap();
    plan.projection = Projection::Value(ValueProjection::Markdown {});
    plan.exclude = vec![".omit".into()];
    let result = document
        .execute(&CompiledPlan::compile(&plan).unwrap())
        .unwrap();
    assert_eq!(result.data.as_values().unwrap(), ["[Y Z](<next>)"]);
    assert_eq!(
        extract("<a href='next'><div>Body</div></a>", "a"),
        "Body\n\n[link](<next>)"
    );
}

#[test]
fn untransformed_code_images_keep_metadata_and_literal_marker_text_cannot_forge_blocks() {
    let value = extract(
        "<pre><img src='photo.png' alt='Photo'><img alt='Only alt'></pre>",
        "pre",
    );
    assert_eq!(value, "```\n\n```\n\n- ![Photo](<photo.png>)\n- Only alt");
    let images = Parser::new(&value)
        .filter_map(|event| match event {
            Event::Start(Tag::Image { dest_url, .. }) => Some(dest_url.to_string()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(images, ["photo.png"]);
    for marker in [
        "# Title",
        "+ Bullet",
        "- Bullet",
        "= Underline",
        "1. Number",
        "2) Number",
    ] {
        let value = extract(&format!("<p>{marker}</p>"), "p");
        assert!(
            !Parser::new(&value)
                .any(|event| matches!(event, Event::Start(Tag::Heading { .. } | Tag::List(_))))
        );
        let text = Parser::new(&value)
            .filter_map(|event| match event {
                Event::Text(text) => Some(text.to_string()),
                _ => None,
            })
            .collect::<String>();
        assert_eq!(text, marker);
    }
}

#[test]
fn a_fence_that_cannot_fit_the_value_budget_fails_before_partial_output() {
    let document = PreparedDocument::new(
        SourceSnapshot::new("<pre></pre>", Default::default()).unwrap(),
        Default::default(),
    )
    .unwrap();
    let mut plan = ExtractionPlan::css("pre").unwrap();
    plan.projection = Projection::Value(ValueProjection::Markdown {});
    plan.limits.max_value_bytes = 2;
    assert_eq!(
        document
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap_err()
            .code,
        ErrorCode::ResourceLimit
    );
}

#[test]
fn selected_code_fragment_cannot_hide_an_unrepresentable_link_annotation() {
    let document = PreparedDocument::new(
        SourceSnapshot::new(
            "<pre><code id='part'><a href='x&#10;y'>Name</a></code></pre>",
            Default::default(),
        )
        .unwrap(),
        Default::default(),
    )
    .unwrap();
    let mut plan = ExtractionPlan::css("#part").unwrap();
    plan.projection = Projection::Value(ValueProjection::Markdown {});
    assert_eq!(
        document
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap_err()
            .code,
        ErrorCode::InvalidRepresentation
    );
}

#[test]
fn omitted_nested_blocks_do_not_change_anchor_kind_or_swallow_following_blocks() {
    for (source, expected) in [
        (
            "<a href='next'><div class='omit'><p>DROP</p><section>DROP</section></div>Label</a>",
            "[Label](<next>)",
        ),
        (
            "<a href='next'><div class='omit'><p>DROP</p></div><template><section>DROP</section></template><div>Body</div></a>",
            "Body\n\n[link](<next>)",
        ),
        (
            "<a href='next'><template><div>DROP</div></template><div class='omit'><p>DROP</p></div>Label</a>",
            "[Label](<next>)",
        ),
    ] {
        let value = extract_excluding(source, "a", &[".omit"]);
        assert_eq!(value, expected);
        assert!(!value.contains("DROP"));
        assert_eq!(
            Parser::new(&value)
                .filter(|event| matches!(event, Event::Start(Tag::Link { .. })))
                .count(),
            1
        );
    }
}

#[test]
fn literal_fence_runs_cross_filtered_descendants_and_nested_code_closes_once() {
    for source in [
        "<pre>``<span class='omit'><b>DROP</b></span>``</pre>",
        "<pre>``<template><div>DROP</div></template>``</pre>",
        "<pre><code>``</code><span>``</span></pre>",
    ] {
        let value = extract_excluding(source, "pre", &[".omit"]);
        assert_eq!(value, "`````\n````\n`````");
        assert_eq!(
            Parser::new(&value)
                .filter_map(|event| match event {
                    Event::Text(text) => Some(text.to_string()),
                    _ => None,
                })
                .collect::<String>(),
            "````\n"
        );
    }
    assert_eq!(
        extract("<pre><pre>A</pre>B</pre><p>After</p>", "body"),
        "```\nAB\n```\n\nAfter"
    );
}

#[test]
fn generated_header_annotations_keep_emphasis_outside_link_and_image_labels() {
    let value = extract(
        "<table><tr><th><pre><a href='dest'>Code</a><img src='img' alt='Photo'></pre>After</th></tr></table>",
        "table",
    );
    let events = Parser::new(&value).collect::<Vec<_>>();
    let labels = events
        .iter()
        .filter_map(|event| match event {
            Event::Text(text) => Some(text.as_ref()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(labels, ["Code\n", "Code", "Photo", "After"], "{value:?}");
    assert!(!labels.iter().any(|label| label.contains("**")));
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::Html(value) | Event::InlineHtml(value) if value.as_ref() == "<strong>"))
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, Event::Start(Tag::Link { .. })))
            .count(),
        1
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, Event::Start(Tag::Image { .. })))
            .count(),
        1
    );
}

#[test]
fn cell_text_after_code_annotations_has_its_own_paragraph() {
    let value = extract(
        "<table><tr><td><pre><a href='dest'>Code</a></pre>After</td><td>Z</td></tr></table>",
        "table",
    );
    assert_eq!(
        value,
        "-\n  - ```\n    Code\n    ```\n    \n    - [Code](<dest>)\n    \n    After\n  - Z"
    );
    assert!(
        Parser::new(&value)
            .any(|event| matches!(event, Event::Text(text) if text.as_ref() == "After"))
    );
}

#[test]
fn empty_image_labels_do_not_join_following_blocks_and_lists_open_block_boundaries() {
    assert_eq!(
        extract("<article><img src='x'><p>After</p></article>", "article"),
        "![](<x>)\n\nAfter"
    );
    assert_eq!(
        extract(
            "<article>Before<ul><li>One</li></ul>After<ol><li>Two</li></ol>Tail</article>",
            "article"
        ),
        "Before\n\n- One\n\nAfter\n\n- 1\\. Two\n\nTail"
    );
}
