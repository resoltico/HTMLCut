use super::*;

#[test]
fn rendered_text_preserves_basic_blocks_lists_links_and_math() {
    let rendered = render_html_as_text(
        "<article><p>Hello</p><ul><li>One</li></ul><hr><pre>  keep\n  spacing</pre></article>",
        WhitespaceMode::Rendered,
    );
    assert!(rendered.contains("Hello"));
    assert!(rendered.contains("- One"));
    assert!(rendered.contains("---"));
    assert!(rendered.contains("  keep"));
    let semantic_rendered = render_html_as_text(
        "<article><ol><li>First</li><li><img src=\"hero.png\" alt=\"Hero\"></li></ol></article>",
        WhitespaceMode::Rendered,
    );
    assert!(semantic_rendered.contains("1. First"));
    assert!(semantic_rendered.contains("2. Hero"));
    let heading_and_link_rendered = render_html_as_text(
        "<article><h2>Coverage</h2><p><a href=\"https://example.com/guide\">Guide</a></p></article>",
        WhitespaceMode::Rendered,
    );
    assert_eq!(
        heading_and_link_rendered,
        "## Coverage\n\nGuide [https://example.com/guide]"
    );
    let nested_list_rendered = render_html_as_text(
        "<article><ol><li>Primary<ul><li>Nested</li></ul></li><li>Second</li></ol></article>",
        WhitespaceMode::Rendered,
    );
    assert_eq!(nested_list_rendered, "1. Primary\n    - Nested\n2. Second");
    assert_eq!(
        render_html_as_text(
            "<article><p><a href=\"https://example.com/guide\"></a></p></article>",
            WhitespaceMode::Rendered,
        ),
        ""
    );
    assert!(
        render_html_as_text(
            "<article><p><a></a></p></article>",
            WhitespaceMode::Rendered
        )
        .is_empty()
    );
    assert_eq!(
        render_html_as_text(
            "<article><p><a href=\"https://example.com/guide\">https://example.com/guide</a></p></article>",
            WhitespaceMode::Rendered,
        ),
        "https://example.com/guide"
    );
    assert_eq!(
        render_html_as_text(
            "<article><p><a href=\"#\">Comments</a></p></article>",
            WhitespaceMode::Rendered,
        ),
        "Comments"
    );
    assert_eq!(
        render_html_as_text(
            "<article><p><a href=\"javascript:void(0)\">Share</a></p></article>",
            WhitespaceMode::Rendered,
        ),
        "Share"
    );
    assert_eq!(
        render_html_as_text(
            "<article><p><a href=\"#history\">History</a></p></article>",
            WhitespaceMode::Rendered,
        ),
        "History [#history]"
    );
    assert_eq!(
        render_html_as_text(
            "<article>\
                <p>Value <span class=\"mwe-math-element\"><span class=\"mwe-math-mathml-inline\" style=\"display: none;\"><math><mrow><mo>(</mo><mi>N</mi><mo>)</mo></mrow></math></span><img aria-hidden=\"true\" alt=\"{\\\\displaystyle (\\\\mathbb {N})}\" src=\"math.svg\"></span> end<sup class=\"reference\"><a href=\"#cite_note-1\"><span class=\"cite-bracket\">[</span>1<span class=\"cite-bracket\">]</span></a></sup>.</p>\
                <section class=\"references\"><ol><li id=\"cite_note-1\">Footnote text.</li></ol></section>\
             </article>",
            WhitespaceMode::Rendered,
        ),
        "Value (N) end."
    );
}
