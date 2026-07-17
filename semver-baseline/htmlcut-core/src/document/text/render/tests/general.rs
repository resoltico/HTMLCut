use super::super::*;
use crate::contracts::WhitespaceMode;
use crate::document::{parse_document_node, select_first};

use super::super::format::*;
use super::super::list::*;
use super::super::table::*;
use super::super::tree::*;

#[test]
fn helper_branches_cover_heading_table_banner_and_spacing_edges() {
    assert_eq!(
        render_html_as_text(
            "<script>ignore</script><style>.x{}</style><p>Body</p>",
            WhitespaceMode::Rendered,
        ),
        "Body"
    );
    assert_eq!(
        render_html_as_text("<p>Alpha<br>Beta</p>", WhitespaceMode::Rendered),
        "Alpha\nBeta"
    );
    assert_eq!(
        render_html_as_text("<p>Alpha</p><hr><p>Beta</p>", WhitespaceMode::Rendered),
        "Alpha\n\n---\n\nBeta"
    );
    assert_eq!(
        render_html_as_text(
            "<p><a href=\"/empty\"><img alt=\"\" src=\"hero.png\"></a>After</p>",
            WhitespaceMode::Rendered,
        ),
        "After"
    );
    assert_eq!(
        render_html_as_text("<h2>   </h2><p>Body</p>", WhitespaceMode::Rendered),
        "Body"
    );
    assert_eq!(
        render_html_as_text(
            "<dl><dt>Term</dt><dd>Definition</dd></dl>",
            WhitespaceMode::Rendered,
        ),
        "Term\n: Definition"
    );

    let heading_document = parse_document_node(
        "<h2><script>ignored</script><pre>  Keep\n  spacing</pre><br><img alt=\"Hero\">Trail<!--note--></h2>",
    );
    let heading = select_first(&heading_document, "h2").expect("heading");
    assert_eq!(
        extract_heading_text(&heading).as_deref(),
        Some("Keep spacing Hero Trail")
    );
    let empty_heading_image = parse_document_node("<h2><img alt=\"   \"><span>Trail</span></h2>");
    let empty_heading_image_heading = select_first(&empty_heading_image, "h2").expect("heading");
    assert_eq!(
        extract_heading_text(&empty_heading_image_heading).as_deref(),
        Some("Trail")
    );
    let heading_image_spacing = parse_document_node("<h2>Title<img alt=\"Hero\"></h2>");
    let heading_image_spacing_heading =
        select_first(&heading_image_spacing, "h2").expect("heading");
    assert_eq!(
        extract_heading_text(&heading_image_spacing_heading).as_deref(),
        Some("Title Hero")
    );
    let pre_only_heading =
        parse_document_node("<h2><pre><span>  Keep\n  spacing</span></pre></h2>");
    let mut pre_only_rendered = String::new();
    render_heading_text_node(
        *select_first(&pre_only_heading, "pre").expect("pre"),
        &mut pre_only_rendered,
        true,
    );
    assert!(pre_only_rendered.contains("Keep"));
    let mut root_heading_rendered = String::new();
    render_heading_text_node(
        heading_document.tree.root(),
        &mut root_heading_rendered,
        false,
    );
    assert!(root_heading_rendered.contains("Keep"));

    assert_eq!(
        render_html_as_text(
            "<article><table><tr></tr></table><p>Body.</p></article>",
            WhitespaceMode::Rendered,
        ),
        "Body."
    );
    assert_eq!(
        render_html_as_text(
            "<article><table>\n<tr><td>Alpha</td></tr>\n</table></article>",
            WhitespaceMode::Rendered,
        ),
        "Alpha"
    );
    assert_eq!(
        render_html_as_text(
            "<article><table><caption>Windows builds</caption><tr><td>Alpha</td></tr></table></article>",
            WhitespaceMode::Rendered,
        ),
        "Windows builds\nAlpha"
    );
    assert_eq!(
        render_html_as_text(
            "<article><table><caption>Caption only</caption></table></article>",
            WhitespaceMode::Rendered,
        ),
        "Caption only"
    );
    assert_eq!(
        render_html_as_text(
            "<article><figure><img alt=\"Hero\" src=\"hero.jpg\"><figcaption>Caption</figcaption></figure><div class=\"caption-box\"><img alt=\"Hero Two\" src=\"hero2.jpg\"><div class=\"caption\">Caption</div></div></article>",
            WhitespaceMode::Rendered,
        ),
        ""
    );
    assert_eq!(
        render_html_as_text(
            "<article><div><img alt=\"Hero\" src=\"hero.jpg\"><figcaption>Caption</figcaption></div></article>",
            WhitespaceMode::Rendered,
        ),
        ""
    );
    let single_anchor_parent =
        parse_document_node("<p><a href=\"https://example.com\">Link</a></p>");
    assert!(
        direct_anchor_child(*select_first(&single_anchor_parent, "p").expect("paragraph"))
            .is_some()
    );
    let multiple_anchor_parent = parse_document_node(
        "<p><a href=\"https://example.com/one\">One</a><a href=\"https://example.com/two\">Two</a></p>",
    );
    assert!(
        direct_anchor_child(*select_first(&multiple_anchor_parent, "p").expect("paragraph"))
            .is_none()
    );
    let text_before_anchor =
        parse_document_node("<p>Intro <a href=\"https://example.com\">Link</a></p>");
    assert!(
        direct_anchor_child(*select_first(&text_before_anchor, "p").expect("paragraph")).is_none()
    );
    let non_anchor_parent = parse_document_node("<p><span>Not a link</span></p>");
    assert!(
        direct_anchor_child(*select_first(&non_anchor_parent, "p").expect("paragraph")).is_none()
    );
    let label_value_section = parse_document_node(
        "<section><div><p>Release Date:</p></div><div><p>4/14/2026</p></div></section>",
    );
    assert_eq!(
        render_label_value_row(
            *select_first(&label_value_section, "section").expect("section"),
            false,
        )
        .as_deref(),
        Some("Release Date: 4/14/2026")
    );
    let label_value_missing_right =
        parse_document_node("<section><div><p>Release Date:</p></div><div></div></section>");
    assert_eq!(
        render_label_value_row(
            *select_first(&label_value_missing_right, "section").expect("section"),
            false,
        ),
        None
    );
    let label_value_with_direct_text = parse_document_node(
        "<section>Lead<div><p>Release Date:</p></div><div><p>4/14/2026</p></div></section>",
    );
    assert_eq!(
        render_label_value_row(
            *select_first(&label_value_with_direct_text, "section").expect("section"),
            false,
        ),
        None
    );
    let label_value_missing_colon = parse_document_node(
        "<section><div><p>Release Date</p></div><div><p>4/14/2026</p></div></section>",
    );
    assert_eq!(
        render_label_value_row(
            *select_first(&label_value_missing_colon, "section").expect("section"),
            false,
        ),
        None
    );
    let label_value_three_children = parse_document_node(
        "<section><div><p>Left:</p></div><div><p>Middle</p></div><div><p>Right</p></div></section>",
    );
    assert_eq!(
        render_label_value_row(
            *select_first(&label_value_three_children, "section").expect("section"),
            false,
        ),
        None
    );
    let label_value_piped =
        parse_document_node("<section><div><p>Left:</p></div><div><p>A | B</p></div></section>");
    assert_eq!(
        render_label_value_row(
            *select_first(&label_value_piped, "section").expect("section"),
            false,
        ),
        None
    );
    let long_label_row = parse_document_node(&format!(
        "<section><div><p>{}:</p></div><div><p>Value</p></div></section>",
        "L".repeat(61)
    ));
    assert_eq!(
        render_label_value_row(
            *select_first(&long_label_row, "section").expect("section"),
            false,
        ),
        None
    );
    let long_value_row = parse_document_node(&format!(
        "<section><div><p>Label:</p></div><div><p>{}</p></div></section>",
        "V".repeat(161)
    ));
    assert_eq!(
        render_label_value_row(
            *select_first(&long_value_row, "section").expect("section"),
            false,
        ),
        None
    );
    let banner_paragraph =
        parse_document_node("<p><a href=\"https://example.com\">BREAKING NEWS</a></p>");
    assert!(paragraph_looks_like_shouty_link_banner(
        *select_first(&banner_paragraph, "p").expect("paragraph"),
        false,
    ));
    let normal_paragraph =
        parse_document_node("<p><a href=\"https://example.com\">Normal headline</a></p>");
    assert!(!paragraph_looks_like_shouty_link_banner(
        *select_first(&normal_paragraph, "p").expect("paragraph"),
        false,
    ));
    let mut prefixed_block = String::new();
    push_prefixed_block(&mut prefixed_block, "", "> ");
    assert!(prefixed_block.is_empty());
    push_prefixed_block(&mut prefixed_block, "Alpha\n\nBeta", "> ");
    assert_eq!(prefixed_block, "> Alpha\n> \n> Beta");
    let mut newline_output = String::new();
    push_newline(&mut newline_output, 2);
    assert!(newline_output.is_empty());
    newline_output.push_str("Alpha\n\n");
    push_newline(&mut newline_output, 1);
    assert_eq!(newline_output, "Alpha\n");
    assert!(!needs_space("(", "word"));
    assert!(!needs_space("word", "."));
    let list_item_block_document = parse_document_node("<li><hr></li><li><span>Body</span></li>");
    assert!(is_list_item_block_segment(
        *select_first(&list_item_block_document, "hr").expect("hr")
    ));
    assert!(!is_list_item_block_segment(
        *select_first(&list_item_block_document, "span").expect("span")
    ));
    let non_table_document = parse_document_node("<div><span>Alpha</span></div>");
    let mut rows = Vec::new();
    collect_table_rows(non_table_document.tree.root(), false, &mut rows);
    assert!(rows.is_empty());
    collect_table_rows(
        *select_first(&non_table_document, "div").expect("div"),
        false,
        &mut rows,
    );
    assert!(rows.is_empty());
    let stray_cells =
        parse_document_node("<table><tr><td>Alpha</td><div>Ignored</div></tr></table>");
    rows.clear();
    collect_table_rows(
        *select_first(&stray_cells, "table").expect("table"),
        false,
        &mut rows,
    );
    assert_eq!(rows, vec![vec!["Alpha".to_owned()]]);

    let banner_document =
        parse_document_node("<p><a href=\"/promo\">READ THE FULL TRANSCRIPT HERE</a></p>");
    let banner = select_first(&banner_document, "p").expect("banner");
    assert!(paragraph_looks_like_shouty_link_banner(*banner, false));
    assert!(looks_like_shouty_banner("READ THE FULL TRANSCRIPT HERE"));
    assert!(!looks_like_shouty_banner("Read THE FULL TRANSCRIPT HERE"));
    assert!(!looks_like_shouty_banner("汉字汉字汉字汉字"));
    let non_banner_document =
        parse_document_node("<p><a href=\"/promo\">Read more</a><span>now</span></p>");
    let non_banner = select_first(&non_banner_document, "p").expect("paragraph");
    assert!(!paragraph_looks_like_shouty_link_banner(*non_banner, false));
    let spaced_anchor_document =
        parse_document_node("<p>  <a href=\"/promo\">LOUD BANNER COPY</a>  </p>");
    let spaced_anchor = select_first(&spaced_anchor_document, "p").expect("paragraph");
    assert!(paragraph_looks_like_shouty_link_banner(
        *spaced_anchor,
        false
    ));

    let right_pipe_row = parse_document_node("<div><div>Label:</div><div>Value | more</div></div>");
    let right_pipe = select_first(&right_pipe_row, "div").expect("row");
    assert_eq!(render_label_value_row(*right_pipe, false), None);

    let left_pipe_row = parse_document_node("<div><div>La|bel:</div><div>Value</div></div>");
    let left_pipe = select_first(&left_pipe_row, "div").expect("row");
    assert_eq!(render_label_value_row(*left_pipe, false), None);

    let list_document = parse_document_node("<ul><li></li><li>Text</li></ul>");
    let list_items = list_document
        .select(&scraper::Selector::parse("li").expect("li selector"))
        .collect::<Vec<_>>();
    let mut empty_item = String::new();
    render_list_item(*list_items[0], &mut empty_item, false);
    assert!(empty_item.trim().is_empty());

    let mut inline_segment = "  Text  ".to_owned();
    let mut body_segments = Vec::new();
    flush_list_item_inline_segment(&mut inline_segment, &mut body_segments);
    assert_eq!(body_segments, vec!["Text"]);
    flush_list_item_inline_segment(&mut inline_segment, &mut body_segments);
    assert_eq!(body_segments, vec!["Text"]);
    let direct_text_document = parse_document_node("<section>Lead<div>Value</div></section>");
    let direct_text_section = select_first(&direct_text_document, "section").expect("section");
    assert!(!direct_text_is_whitespace_only(*direct_text_section));
    let whitespace_only_document = parse_document_node("<section>\n  <div>Value</div>\n</section>");
    let whitespace_only_section =
        select_first(&whitespace_only_document, "section").expect("section");
    assert!(direct_text_is_whitespace_only(*whitespace_only_section));
    let ordered_document = parse_document_node(
        "<ol reversed start=\"5\"><li>One</li><li value=\"10\">Two</li><li>Three</li></ol>",
    );
    let ordered_items = ordered_document
        .select(&scraper::Selector::parse("li").expect("li selector"))
        .collect::<Vec<_>>();
    assert_eq!(list_item_marker(*ordered_items[0]), "5. ");
    assert_eq!(list_item_marker(*ordered_items[1]), "10. ");
    assert_eq!(list_item_marker(*ordered_items[2]), "9. ");
    assert!(is_list_item_block_segment(
        *select_first(
            &parse_document_node("<blockquote><p>Quote</p></blockquote>"),
            "blockquote",
        )
        .expect("blockquote")
    ));
    assert!(!is_list_item_block_segment(
        *select_first(&parse_document_node("<span>Inline</span>"), "span").expect("span")
    ));

    assert_eq!(
        collapse_inline_whitespace("  Hello   world "),
        "Hello world"
    );
    assert!(!needs_space("", "world"));
    assert!(!needs_space("Hello", ""));
    assert!(!needs_space("(", "world"));
    assert!(!needs_space("Hello", "."));
    assert!(needs_space("Hello", "world"));

    let mut output = "Hello".to_owned();
    push_newline(&mut output, 2);
    assert_eq!(output, "Hello\n\n");
    assert_eq!(
        apply_whitespace_mode("Alpha\n\n\nBeta\n", WhitespaceMode::Normalize),
        "Alpha\n\nBeta"
    );
    assert_eq!(
        apply_whitespace_mode("Alpha\n\nBeta\n", WhitespaceMode::Rendered),
        "Alpha\n\nBeta"
    );
    assert_eq!(normalize_structured_line("   "), "");
    assert_eq!(
        remove_immediate_heading_echoes("# Heading\n\nHeading\n\n\nBody"),
        "# Heading\n\n\nBody"
    );
    assert_eq!(
        remove_immediate_heading_echoes("# Heading\n\nHeading\nBody"),
        "# Heading\n\nBody"
    );
}
