use super::*;

fn render(html: &str, selector: &str, projection: Projection, normalize: bool) -> String {
    let mut plan = ExtractionPlan::css(selector).unwrap();
    plan.projection = projection;
    if normalize {
        plan.transforms.push(Transform::NormalizeWhitespace {});
    }
    prepared(html)
        .execute(&CompiledPlan::compile(&plan).unwrap())
        .unwrap()
        .values
        .remove(0)
}

#[test]
fn selected_pre_descendants_preserve_payload_and_only_selected_framing() {
    let html =
        "<pre>OUTSIDE<code id='target'><span>first line\n    second line</span></code>AFTER</pre>";
    for selector in ["#target", "#target span"] {
        for normalize in [false, true] {
            assert_eq!(
                render(html, selector, Projection::DomText {}, normalize),
                "first line\n    second line"
            );
            assert_eq!(
                render(html, selector, Projection::DocumentText {}, normalize),
                "```\nfirst line\n    second line\n```"
            );
        }
    }
    assert_eq!(
        render("<p>A   B</p>", "p", Projection::DomText {}, true),
        "A B"
    );
}

#[test]
fn original_list_ordinals_survive_fragment_selection_and_exclusions() {
    for (html, expected) in [
        (
            "<ol start='7'><li>A</li><li id='target'>B</li></ol>",
            "8. B",
        ),
        ("<ol><li id='target' value='21'>B</li></ol>", "21. B"),
        (
            "<ol start='10' reversed><li>A</li><li id='target'>B</li></ol>",
            "9. B",
        ),
        (
            "<ol><li value='20'>SECRET</li><li id='target'>B</li></ol>",
            "21. B",
        ),
    ] {
        assert_eq!(
            render(html, "#target", Projection::DocumentText {}, false),
            expected
        );
    }
    let mut plan = ExtractionPlan::css("ol").unwrap();
    plan.projection = Projection::DocumentText {};
    plan.exclude.push(".omit".into());
    assert_eq!(
        prepared("<ol start='7'><li class='omit'>SECRET</li><li>B</li></ol>")
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap()
            .values,
        ["8. B"]
    );
    assert_eq!(
        prepared("<ol reversed><li>A</li><li class='omit'>SECRET</li><li>C</li></ol>")
            .execute(&CompiledPlan::compile(&plan).unwrap())
            .unwrap()
            .values,
        ["3. A\n1. C"]
    );
}

#[test]
fn table_fragments_are_closed_cells_and_source_delimiters_are_payload() {
    let html = "<table><thead><tr><th>A</th><th>B</th></tr></thead><tbody><tr><td>Taxi</td><td>180</td></tr><tr><td>C</td><td>D</td></tr></tbody></table>";
    assert_eq!(
        render(html, "thead", Projection::DocumentText {}, false),
        "[cell][header] A[/cell] | [cell][header] B[/cell]"
    );
    assert_eq!(
        render(html, "tbody", Projection::DocumentText {}, false),
        "[cell]Taxi[/cell] | [cell]180[/cell]\n[cell]C[/cell] | [cell]D[/cell]"
    );
    assert_eq!(
        render(
            "<table><tr><td>A | B</td></tr></table>",
            "tr",
            Projection::DocumentText {},
            false
        ),
        "[cell]A \\| B[/cell]"
    );
    assert_eq!(
        render(
            "<table><tr><td>A</td><td>B</td></tr></table>",
            "tr",
            Projection::DocumentText {},
            false
        ),
        "[cell]A[/cell] | [cell]B[/cell]"
    );
    assert_eq!(
        render(
            "<table><tr><td><span>A | B</span></td></tr></table>",
            "span",
            Projection::DomText {},
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
            "Name\nAlice\nAmount\n180",
        ),
        (
            "<details><summary>Title</summary><div>A</div><div>B</div></details>",
            "details",
            "Title\nA\nB",
        ),
        (
            "<main><address>A</address><address>B</address></main>",
            "main",
            "A\nB",
        ),
    ] {
        assert_eq!(
            render(html, selector, Projection::DocumentText {}, false),
            expected
        );
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
                    Projection::DocumentText {},
                    false
                ),
                expected
            );
        }
    }
}

// This inverse reads the documented frame grammar, without calling render/escape/fence helpers.
// It intentionally accepts only rows of plain or fenced-pre cells for this property family.
fn decode_row_cells(mut input: &str) -> Vec<String> {
    let mut cells = Vec::new();
    while !input.is_empty() {
        input = input.strip_prefix("[cell]").expect("cell opening");
        let mut payload = String::new();
        if let Some(framed) = input.strip_prefix('\n')
            && let Some((fence, rest)) = framed.split_once('\n')
            && fence.len() >= 3
            && fence.bytes().all(|byte| byte == b'`')
        {
            let closing = format!("\n{fence}[/cell]");
            let (literal, remaining) = rest.split_once(&closing).expect("balanced pre/cell");
            payload.push_str(literal);
            input = remaining;
        } else {
            loop {
                if let Some(remaining) = input.strip_prefix("[/cell]") {
                    input = remaining;
                    break;
                }
                let character = input.chars().next().expect("balanced plain cell");
                input = &input[character.len_utf8()..];
                if character == '\\' {
                    let escaped = input.chars().next().expect("complete escape");
                    assert!("\\[]()|`".contains(escaped), "reserved escape only");
                    payload.push(escaped);
                    input = &input[escaped.len_utf8()..];
                } else {
                    assert_ne!(character, '[', "unescaped structural delimiter");
                    payload.push(character);
                }
            }
        }
        cells.push(payload);
        if !input.is_empty() {
            input = input.strip_prefix(" | ").expect("separator outside cells");
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
            let output = render(&html, "tr", Projection::DocumentText {}, false);
            assert_eq!(decode_row_cells(&output), [first, second], "{html}");
            let html = format!(
                "<table><tr><td><pre><code>{first}</code></pre></td><td>{second}</td></tr></table>"
            );
            let output = render(&html, "tr", Projection::DocumentText {}, false);
            assert_eq!(decode_row_cells(&output), [first, second], "{html}");
        }
    }
    // Distinct cell arrangements cannot collapse to the same escaped payload representation.
    assert_ne!(
        render(
            "<table><tr><td>A | B</td></tr></table>",
            "tr",
            Projection::DocumentText {},
            false
        ),
        render(
            "<table><tr><td>A</td><td>B</td></tr></table>",
            "tr",
            Projection::DocumentText {},
            false
        ),
    );
}

#[test]
fn caption_header_and_span_metadata_have_complete_balanced_frames() {
    let html = "<table><caption>Title | [caption]</caption><tr><th rowspan='2' colspan='3'>H</th><td rowspan='[x]'>V</td></tr></table>";
    assert_eq!(
        render(html, "caption", Projection::DocumentText {}, false),
        "[caption]Title \\| \\[caption\\][/caption]"
    );
    assert_eq!(
        render(html, "tr", Projection::DocumentText {}, false),
        "[cell][header] [rowspan=2] [colspan=3] H[/cell] | [cell][rowspan=\\[x\\]] V[/cell]"
    );
}

#[test]
fn foreign_namespace_names_do_not_invent_html_table_frames() {
    for namespace in ["svg", "math"] {
        let html = format!(
            "<{namespace}><caption>Title</caption><tr><th>A</th><td>B</td></tr></{namespace}>"
        );
        assert_eq!(
            render(&html, namespace, Projection::DocumentText {}, false),
            "TitleAB"
        );
        assert_eq!(render(&html, "td", Projection::DocumentText {}, false), "B");
    }
}

#[test]
fn html_table_inside_a_foreign_integration_point_keeps_its_roles() {
    let html =
        "<svg><foreignObject><table><tr><td>A</td><td>B</td></tr></table></foreignObject></svg>";
    assert_eq!(
        render(html, "svg", Projection::DocumentText {}, false),
        "[table]\n[cell]A[/cell] | [cell]B[/cell]\n[/table]"
    );
}

#[test]
fn excluded_pre_fragment_root_emits_no_synthetic_fence_or_payload() {
    let document = prepared("<pre>OUTSIDE<code id='target'>kept\n</code>AFTER</pre>");
    let mut plan = ExtractionPlan::css("#target").unwrap();
    plan.projection = Projection::DocumentText {};
    plan.exclude.push("#target".into());
    let result = document
        .execute(&CompiledPlan::compile(&plan).unwrap())
        .unwrap();
    assert_eq!(result.values, [""]);
    assert_eq!((result.candidate_count, result.selected_count), (1, 1));
}

#[test]
fn fragment_list_depth_follows_selected_root_and_its_parent_role() {
    for (html, selector, expected) in [
        (
            "<ol><li id='target'>A<ul><li>B</li></ul></li></ol>",
            "#target",
            "1. A\n  - B",
        ),
        (
            "<ol><div id='target'><ul><li>B</li></ul></div></ol>",
            "#target",
            "- B",
        ),
        (
            "<main><li id='target'>A<ul><li>B</li></ul></li></main>",
            "#target",
            "- A\n- B",
        ),
    ] {
        assert_eq!(
            render(html, selector, Projection::DocumentText {}, false),
            expected
        );
    }
}
