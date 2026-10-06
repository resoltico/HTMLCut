// SPDX-License-Identifier: MPL-2.0
use super::*;

fn render(html: &str, selector: &str, reading: Reading, normalize: bool) -> String {
    let mut plan = ExtractionPlan::css(selector).unwrap();
    plan.read = Some(reading);
    if normalize && matches!(plan.read, Some(Reading::Literal)) {
        plan.read = Some(Reading::Text);
    }
    prepared(html)
        .execute(&CompiledPlan::compile(&plan).unwrap())
        .unwrap()
        .data()
        .as_values()
        .unwrap()[0]
        .clone()
}

#[test]
fn selected_pre_descendants_preserve_payload_and_only_selected_framing() {
    let html =
        "<pre>OUTSIDE<code id='target'><span>first line\n    second line</span></code>AFTER</pre>";
    for selector in ["#target", "#target span"] {
        for normalize in [false, true] {
            assert_eq!(
                render(html, selector, Reading::Literal, normalize),
                "first line\n    second line"
            );
            assert_eq!(
                render(html, selector, Reading::Markdown, normalize),
                "```\nfirst line\n    second line\n```"
            );
        }
    }
    assert_eq!(render("<p>A   B</p>", "p", Reading::Literal, true), "A B");
}

#[test]
fn original_list_ordinals_survive_fragment_selection_and_exclusions() {
    for (html, expected) in [
        (
            "<ol start='7'><li>A</li><li id='target'>B</li></ol>",
            "- 8\\. B",
        ),
        ("<ol><li id='target' value='21'>B</li></ol>", "- 21\\. B"),
        (
            "<ol start='10' reversed><li>A</li><li id='target'>B</li></ol>",
            "- 9\\. B",
        ),
        (
            "<ol><li value='20'>SECRET</li><li id='target'>B</li></ol>",
            "- 21\\. B",
        ),
    ] {
        assert_eq!(render(html, "#target", Reading::Markdown, false), expected);
    }
    let mut plan = ExtractionPlan::css("ol").unwrap();
    plan.read = Some(Reading::Markdown);
    plan.exclude.get_or_insert_default().push(".omit".into());
    assert_eq!(
        prepared("<ol start='7'><li class='omit'>SECRET</li><li>B</li></ol>")
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap()
            .data()
            .as_values()
            .unwrap(),
        ["- 8\\. B"]
    );
    assert_eq!(
        prepared("<ol reversed><li>A</li><li class='omit'>SECRET</li><li>C</li></ol>")
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap()
            .data()
            .as_values()
            .unwrap(),
        ["- 3\\. A\n- 1\\. C"]
    );
}

#[test]
fn table_fragments_preserve_cells_and_literal_delimiters_as_nested_lists() {
    let html = "<table><thead><tr><th>A</th><th>B</th></tr></thead><tbody><tr><td>Taxi</td><td>180</td></tr><tr><td>C</td><td>D</td></tr></tbody></table>";
    assert_eq!(
        render(html, "thead", Reading::Markdown, false),
        "-\n  - <strong>A</strong>\n  - <strong>B</strong>"
    );
    assert_eq!(
        render(html, "tbody", Reading::Markdown, false),
        "-\n  - Taxi\n  - 180\n-\n  - C\n  - D"
    );
    assert_eq!(
        render(
            "<table><tr><td>A | B</td></tr></table>",
            "tr",
            Reading::Markdown,
            false
        ),
        "-\n  - A | B"
    );
    assert_eq!(
        render(
            "<table><tr><td>A</td><td>B</td></tr></table>",
            "tr",
            Reading::Markdown,
            false
        ),
        "-\n  - A\n  - B"
    );
    assert_eq!(
        render(
            "<table><tr><td><span>A | B</span></td></tr></table>",
            "span",
            Reading::Literal,
            false
        ),
        "A | B"
    );
}

#[test]
fn semantic_blocks_have_boundaries_without_visibility_inference() {
    for (html, selector, expected) in [
        (
            "<dl><dt>Name</dt><dd>Alice</dd><dt>Amount</dt><dd>180</dd></dl>",
            "dl",
            "Name\n\nAlice\n\nAmount\n\n180",
        ),
        (
            "<details><summary>Title</summary><div>A</div><div>B</div></details>",
            "details",
            "Title\n\nA\n\nB",
        ),
        (
            "<main><address>A</address><address>B</address></main>",
            "main",
            "A\n\nB",
        ),
    ] {
        assert_eq!(render(html, selector, Reading::Markdown, false), expected);
    }
}

#[test]
fn pre_framing_keeps_payload_trailing_newlines_distinguishable() {
    for (payload, expected) in [
        ("x", "```\nx\n```"),
        ("x\n", "```\nx\n\n```"),
        ("x\n\n", "```\nx\n\n\n```"),
        ("", "```\n\n```"),
    ] {
        for selector in ["pre", "pre code"] {
            assert_eq!(
                render(
                    &format!("<pre><code>{payload}</code></pre>"),
                    selector,
                    Reading::Markdown,
                    false
                ),
                expected
            );
        }
    }
}

// Independent CommonMark events recover plain/fenced cells; no renderer helper is called.
fn decode_row_cells(input: &str) -> Vec<String> {
    use pulldown_cmark::{Event, Parser, Tag, TagEnd};
    let mut depth = 0;
    let mut cells = Vec::new();
    let mut current = None::<String>;
    for event in Parser::new(input) {
        match event {
            Event::Start(Tag::List(_)) => depth += 1,
            Event::End(TagEnd::List(_)) => depth -= 1,
            Event::Start(Tag::Item) if depth == 2 => current = Some(String::new()),
            Event::End(TagEnd::Item) if depth == 2 => cells.push(current.take().unwrap()),
            Event::Text(text) if current.is_some() => current.as_mut().unwrap().push_str(&text),
            Event::End(TagEnd::CodeBlock) => {
                current
                    .as_mut()
                    .unwrap_or_else(|| panic!("Code escaped cell: {input:?}"))
                    .pop();
            }
            _ => (),
        }
    }
    cells
}

#[test]
fn independent_cell_inverse_preserves_collision_payloads_and_pre_newlines() {
    let payloads = [
        "",
        "A | B",
        "[cell][/cell][caption][/table]",
        "\\n\nactual LF",
        "[]()\\|`",
        "```````",
        "✓ € 東京",
        "x\n",
        "x\n\n",
        "\nleading",
    ];
    for first in payloads {
        for second in payloads {
            let html = format!("<table><tr><td>{first}</td><td>{second}</td></tr></table>");
            let output = render(&html, "tr", Reading::Markdown, false);
            assert_eq!(
                decode_row_cells(&output),
                [
                    first.split_ascii_whitespace().collect::<Vec<_>>().join(" "),
                    second
                        .split_ascii_whitespace()
                        .collect::<Vec<_>>()
                        .join(" ")
                ],
                "{html}"
            );
            let html = format!(
                "<table><tr><td><pre><code>{first}</code></pre></td><td>{second}</td></tr></table>"
            );
            let output = render(&html, "tr", Reading::Markdown, false);
            assert_eq!(
                decode_row_cells(&output),
                [
                    first.to_owned(),
                    second
                        .split_ascii_whitespace()
                        .collect::<Vec<_>>()
                        .join(" ")
                ],
                "{html}"
            );
        }
    }
    // Distinct cell arrangements cannot collapse to the same escaped payload representation.
    assert_ne!(
        render(
            "<table><tr><td>A | B</td></tr></table>",
            "tr",
            Reading::Markdown,
            false
        ),
        render(
            "<table><tr><td>A</td><td>B</td></tr></table>",
            "tr",
            Reading::Markdown,
            false
        ),
    );
}

#[test]
fn caption_and_header_content_use_reading_format_without_inventing_span_layout() {
    let html = "<table><caption>Title | [caption]</caption><tr><th rowspan='2' colspan='3'>H</th><td rowspan='[x]'>V</td></tr></table>";
    assert_eq!(
        render(html, "caption", Reading::Markdown, false),
        "Title | \\[caption\\]"
    );
    assert_eq!(
        render(html, "tr", Reading::Markdown, false),
        "-\n  - <strong>H</strong>\n  - V"
    );
}

#[test]
fn foreign_namespace_names_do_not_invent_html_table_frames() {
    for namespace in ["svg", "math"] {
        let html = format!(
            "<{namespace}><caption>Title</caption><tr><th>A</th><td>B</td></tr></{namespace}>"
        );
        assert_eq!(
            render(&html, namespace, Reading::Markdown, false),
            "TitleAB"
        );
        assert_eq!(render(&html, "td", Reading::Markdown, false), "B");
    }
}

#[test]
fn html_table_inside_a_foreign_integration_point_keeps_its_roles() {
    let html =
        "<svg><foreignObject><table><tr><td>A</td><td>B</td></tr></table></foreignObject></svg>";
    assert_eq!(
        render(html, "svg", Reading::Markdown, false),
        "-\n  - A\n  - B"
    );
}

#[test]
fn excluded_pre_fragment_root_emits_no_synthetic_fence_or_payload() {
    let document = prepared("<pre>OUTSIDE<code id='target'>kept\n</code>AFTER</pre>");
    let mut plan = ExtractionPlan::css("#target").unwrap();
    plan.read = Some(Reading::Markdown);
    plan.exclude.get_or_insert_default().push("#target".into());
    let result = document
        .execute(&CompiledPlan::compile(&plan).unwrap())
        .unwrap();
    assert_eq!(result.data().as_values().unwrap(), [""]);
    assert_eq!(
        (
            result.receipt().unwrap().candidate_count,
            result.receipt().unwrap().selected_count
        ),
        (1, 1)
    );
}

#[test]
fn fragment_list_depth_follows_selected_root_and_its_parent_role() {
    for (html, selector, expected) in [
        (
            "<ol><li id='target'>A<ul><li>B</li></ul></li></ol>",
            "#target",
            "- 1\\. A\n  - B",
        ),
        (
            "<ol><div id='target'><ul><li>B</li></ul></div></ol>",
            "#target",
            "- B",
        ),
        (
            "<main><li id='target'>A<ul><li>B</li></ul></li></main>",
            "#target",
            "- A\n  - B",
        ),
    ] {
        assert_eq!(render(html, selector, Reading::Markdown, false), expected);
    }
}
