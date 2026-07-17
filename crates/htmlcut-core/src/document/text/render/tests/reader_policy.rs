use super::super::*;
use crate::contracts::WhitespaceMode;
use crate::document::{parse_document_node, select_first};

use super::super::super::policy::{
    collect_notice_node_text, collect_notice_text, element_has_hidden_style,
    element_looks_like_auxiliary_section, element_looks_like_brief_reader_notice,
    element_looks_like_reader_auxiliary, element_looks_like_source_attribution,
    element_should_skip_in_reader_text, is_note_fragment_href, looks_like_note_fragment_anchor,
    should_skip_rendered_element, tokenize_notice_text,
};
use super::super::math::*;
use super::super::tree::*;

#[test]
fn reader_cleanup_and_math_helpers_cover_hidden_auxiliary_and_math_edges() {
    assert_eq!(
        render_html_as_text(
            "<article><p>Alpha <math><msup><mi>x</mi><mn>2</mn></msup></math> Beta</p></article>",
            WhitespaceMode::Rendered,
        ),
        "Alpha x^2 Beta"
    );
    assert_eq!(
        render_html_as_text(
            "<article><span style=\"display:none\"><math><mi>x</mi></math></span><span style=\"visibility:hidden\">Hidden</span><a href=\"#cite_note-1\">[1]</a><span class=\"backlink\">Back</span><p>Body</p></article>",
            WhitespaceMode::Rendered,
        ),
        "x\n\nBody"
    );

    let heading_math = parse_document_node(
        "<h2><math><mfrac><mi>a</mi><mi>b</mi></mfrac></math><span class=\"reference\">[1]</span><img alt=\"Hero\"></h2>",
    );
    let heading = select_first(&heading_math, "h2").expect("heading");
    assert_eq!(extract_heading_text(&heading).as_deref(), Some("a/b Hero"));
    let mut heading_math_rendered = String::new();
    render_heading_text_node(
        *select_first(&heading_math, "math").expect("math"),
        &mut heading_math_rendered,
        false,
    );
    assert_eq!(heading_math_rendered, "a/b");
    let hidden_heading_math =
        parse_document_node("<h2><span style=\"display:none\"><math><mi>x</mi></math></span></h2>");
    let mut hidden_heading_rendered = String::new();
    render_heading_text_node(
        *select_first(&hidden_heading_math, "span").expect("span"),
        &mut hidden_heading_rendered,
        false,
    );
    assert_eq!(hidden_heading_rendered, "x");

    let fallback_math_document =
        parse_document_node("<math alttext=\"x squared\"><annotation>ignored</annotation></math>");
    let fallback_math = select_first(&fallback_math_document, "math").expect("math");
    assert_eq!(
        render_math_element(&fallback_math).as_deref(),
        Some("x squared")
    );

    let hidden_with_math =
        parse_document_node("<span style=\"display:none\"><math><mi>z</mi></math></span>");
    let hidden_math = select_first(&hidden_with_math, "span").expect("hidden math");
    assert_eq!(hidden_math_replacement(&hidden_math).as_deref(), Some("z"));

    let hidden_style_false = parse_document_node("<span style=\"display\">Body</span>");
    let hidden_style_false_element = select_first(&hidden_style_false, "span").expect("span");
    assert!(!element_has_hidden_style(&hidden_style_false_element));

    let note_anchor = parse_document_node("<sup><a href=\"#cite_note-1\">[1]</a></sup>");
    let note_anchor_element = select_first(&note_anchor, "sup").expect("sup");
    assert!(element_looks_like_reader_auxiliary(&note_anchor_element));
    assert!(looks_like_note_fragment_anchor(&note_anchor_element));
    assert!(is_note_fragment_href("#CITE_NOTE-1"));

    let backlink = parse_document_node("<span class=\"backlink\">Back</span>");
    let backlink_element = select_first(&backlink, "span").expect("span");
    assert!(element_looks_like_reader_auxiliary(&backlink_element));
    let reference_list = parse_document_node("<ul class=\"references\"><li>Ref</li></ul>");
    let reference_list_element = select_first(&reference_list, "ul").expect("ul");
    assert!(element_looks_like_reader_auxiliary(&reference_list_element));
    let hatnote = parse_document_node(
        "<div class=\"hatnote navigation-not-searchable\">For other uses, see <a href=\"/wiki/Math_(disambiguation)\">Math (disambiguation)</a>.</div>",
    );
    let hatnote_element = select_first(&hatnote, "div").expect("hatnote");
    assert!(element_looks_like_reader_auxiliary(&hatnote_element));
    let subjectpageheader = parse_document_node(
        "<div class=\"mw-subjectpageheader\"><span>From Wikipedia, the free encyclopedia</span></div>",
    );
    let subjectpageheader_element =
        select_first(&subjectpageheader, "div").expect("subjectpageheader");
    assert!(element_looks_like_reader_auxiliary(
        &subjectpageheader_element
    ));
    let affiliate_notice = parse_document_node(
        "<span>When you purchase through links on our site, we may earn an affiliate commission. <a href=\"/terms\">Here’s how it works</a>.</span>",
    );
    let affiliate_notice_element =
        select_first(&affiliate_notice, "span").expect("affiliate notice");
    assert!(element_looks_like_brief_reader_notice(
        &affiliate_notice_element
    ));
    let ordinary_short_text = parse_document_node(
        "<span><a href=\"/guide\">Guide</a> to the latest experiment results.</span>",
    );
    let ordinary_short_text_element =
        select_first(&ordinary_short_text, "span").expect("ordinary short text");
    assert!(!element_looks_like_brief_reader_notice(
        &ordinary_short_text_element
    ));
    assert_eq!(
        collect_notice_text(*affiliate_notice_element, 240),
        "When you purchase through links on our site, we may earn an affiliate commission. Here’s how it works."
    );
    let long_strong_notice = parse_document_node(
        "<span>\
            This article was generated for demonstration purposes. \
            This article was generated for demonstration purposes. \
            This article was generated for demonstration purposes. \
            This article was generated for demonstration purposes. \
            This article was generated for demonstration purposes. \
            <a href=\"/terms\">Read more</a>.\
        </span>",
    );
    assert!(element_looks_like_brief_reader_notice(
        &select_first(&long_strong_notice, "span").expect("strong notice")
    ));
    assert!(element_should_skip_in_reader_text(
        &select_first(&long_strong_notice, "span").expect("strong notice")
    ));
    let source_attribution = parse_document_node(
        "<p>Source: <a href=\"https://example.test/feed\">Research Feed</a></p>",
    );
    let source_attribution_element =
        select_first(&source_attribution, "p").expect("source attribution");
    assert!(element_looks_like_source_attribution(
        &source_attribution_element
    ));
    assert!(element_should_skip_in_reader_text(
        &source_attribution_element
    ));
    let auxiliary_section = parse_document_node(
        "<section><h2>References</h2><ul><li><a href=\"/source\">Source</a></li></ul></section>",
    );
    let auxiliary_section_element =
        select_first(&auxiliary_section, "section").expect("auxiliary section");
    assert!(element_looks_like_auxiliary_section(
        &auxiliary_section_element
    ));
    assert!(element_should_skip_in_reader_text(
        &auxiliary_section_element
    ));
    assert!(should_skip_rendered_element(
        &source_attribution_element,
        TextRenderIntent::ReaderDocument,
        false,
    ));
    assert!(should_skip_rendered_element(
        &auxiliary_section_element,
        TextRenderIntent::ReaderDocument,
        false,
    ));
    assert_eq!(
        tokenize_notice_text("Terms apply; we may earn affiliate commission!"),
        vec![
            "terms",
            "apply",
            "we",
            "may",
            "earn",
            "affiliate",
            "commission"
        ]
    );
    let utility_heading =
        parse_document_node("<h2><div class=\"status pricing report\">Status</div></h2>");
    let mut utility_heading_rendered = String::new();
    render_heading_text_node(
        *select_first(&utility_heading, "div").expect("utility heading"),
        &mut utility_heading_rendered,
        false,
    );
    assert_eq!(utility_heading_rendered, "");
    let selected_notice =
        parse_document_node("<div class=\"status pricing report\">Selected body</div>");
    let selected_notice_element = select_first(&selected_notice, "div").expect("notice");
    assert!(!should_skip_rendered_element(
        &selected_notice_element,
        TextRenderIntent::SelectedFragment,
        true,
    ));
    assert!(should_skip_rendered_element(
        &selected_notice_element,
        TextRenderIntent::SelectedFragment,
        false,
    ));
    let hidden_selected = parse_document_node("<div hidden>Hidden body</div>");
    let hidden_selected_element = select_first(&hidden_selected, "div").expect("hidden");
    assert!(should_skip_rendered_element(
        &hidden_selected_element,
        TextRenderIntent::SelectedFragment,
        true,
    ));
    let nested_notice_document = parse_document_node(
        "<div><!--ignored--><script>skip</script><style>.x{}</style><template><span>Hidden</span></template><noscript>Fallback</noscript><span>Alpha</span><span>Beta</span></div>",
    );
    let nested_notice_element = select_first(&nested_notice_document, "div").expect("notice");
    assert_eq!(
        collect_notice_text(*nested_notice_element, 240),
        "Alpha Beta"
    );
    let whitespace_notice_document = parse_document_node("<div>   </div>");
    let whitespace_notice = select_first(&whitespace_notice_document, "div").expect("notice");
    let whitespace_text = whitespace_notice
        .children()
        .next()
        .expect("whitespace text child");
    let mut whitespace_output = String::new();
    collect_notice_node_text(whitespace_text, 240, &mut whitespace_output);
    assert!(whitespace_output.is_empty());
    let mut capped_output = "already enough".to_owned();
    collect_notice_node_text(*nested_notice_element, 5, &mut capped_output);
    assert_eq!(capped_output, "already enough");
    let mut document_root_output = String::new();
    collect_notice_node_text(
        nested_notice_document.tree.root(),
        240,
        &mut document_root_output,
    );
    assert!(document_root_output.contains("Alpha Beta"));
    let root_limited_document = parse_document_node("<div>Alpha</div><div>Beta</div>");
    let mut root_limited_output = String::new();
    collect_notice_node_text(
        root_limited_document.tree.root(),
        5,
        &mut root_limited_output,
    );
    assert_eq!(root_limited_output, "Alpha");

    let direct_math_root = parse_document_node("<math><msup><mi>x</mi><mn>2</mn></msup></math>");
    let mut direct_math_root_rendered = String::new();
    render_node(
        *select_first(&direct_math_root, "math").expect("math"),
        &mut direct_math_root_rendered,
        false,
        false,
    );
    assert_eq!(direct_math_root_rendered, "x^2");

    let direct_math = parse_document_node(
        "<math><msub><mi>x</mi><mi>i</mi></msub><msubsup><mi>y</mi><mi>i</mi><mn>2</mn></msubsup><msqrt><mi>z</mi></msqrt><mroot><mi>x</mi><mn>3</mn></mroot></math>",
    );
    let direct_math_element = select_first(&direct_math, "math").expect("math");
    assert_eq!(
        render_math_element(&direct_math_element).as_deref(),
        Some("x_i y_i^2 sqrt(z) root(x, 3)")
    );

    let incomplete_fraction = parse_document_node("<mfrac><mi>a</mi></mfrac>");
    assert_eq!(
        render_math_fraction(*select_first(&incomplete_fraction, "mfrac").expect("mfrac")),
        None
    );
    let incomplete_sub = parse_document_node("<msub><mi>a</mi></msub>");
    assert_eq!(
        render_math_binary_operator(*select_first(&incomplete_sub, "msub").expect("msub"), "_"),
        None
    );
    let incomplete_subsup = parse_document_node("<msubsup><mi>a</mi><mi>b</mi></msubsup>");
    assert_eq!(
        render_math_subsup(*select_first(&incomplete_subsup, "msubsup").expect("msubsup")),
        None
    );
    let empty_wrapped = parse_document_node("<msqrt><annotation>ignored</annotation></msqrt>");
    assert_eq!(
        render_math_wrapped(
            *select_first(&empty_wrapped, "msqrt").expect("msqrt"),
            "sqrt(",
            ")"
        ),
        None
    );
    let incomplete_root = parse_document_node("<mroot><mi>a</mi></mroot>");
    assert_eq!(
        render_math_root(*select_first(&incomplete_root, "mroot").expect("mroot")),
        None
    );
    let whitespace_math = parse_document_node("<math>   </math>");
    assert_eq!(
        render_math_children_to_string(*select_first(&whitespace_math, "math").expect("math")),
        ""
    );
    let numerator_empty =
        parse_document_node("<mfrac><annotation>ignored</annotation><mi>b</mi></mfrac>");
    assert_eq!(
        render_math_fraction(*select_first(&numerator_empty, "mfrac").expect("mfrac")),
        None
    );
    let left_empty = parse_document_node("<msub><annotation>ignored</annotation><mi>b</mi></msub>");
    assert_eq!(
        render_math_binary_operator(*select_first(&left_empty, "msub").expect("msub"), "_"),
        None
    );
    let sub_empty = parse_document_node(
        "<msubsup><mi>a</mi><annotation>ignored</annotation><mi>c</mi></msubsup>",
    );
    assert_eq!(
        render_math_subsup(*select_first(&sub_empty, "msubsup").expect("msubsup")),
        None
    );
    let root_value_empty =
        parse_document_node("<mroot><annotation>ignored</annotation><mn>3</mn></mroot>");
    assert_eq!(
        render_math_root(*select_first(&root_value_empty, "mroot").expect("mroot")),
        None
    );
    let rendered_fraction = parse_document_node("<mfrac><mi>a</mi><mi>b</mi></mfrac>");
    let mut fraction_output = String::new();
    render_math_node(
        *select_first(&rendered_fraction, "mfrac").expect("mfrac"),
        &mut fraction_output,
    );
    assert_eq!(fraction_output, "a/b");
    let rendered_sub = parse_document_node("<msub><mi>x</mi><mi>i</mi></msub>");
    let mut sub_output = String::new();
    render_math_node(
        *select_first(&rendered_sub, "msub").expect("msub"),
        &mut sub_output,
    );
    assert_eq!(sub_output, "x_i");
    let rendered_sup = parse_document_node("<msup><mi>x</mi><mn>2</mn></msup>");
    let mut sup_output = String::new();
    render_math_node(
        *select_first(&rendered_sup, "msup").expect("msup"),
        &mut sup_output,
    );
    assert_eq!(sup_output, "x^2");
    let rendered_subsup = parse_document_node("<msubsup><mi>y</mi><mi>i</mi><mn>2</mn></msubsup>");
    let mut subsup_output = String::new();
    render_math_node(
        *select_first(&rendered_subsup, "msubsup").expect("msubsup"),
        &mut subsup_output,
    );
    assert_eq!(subsup_output, "y_i^2");
    let rendered_sqrt = parse_document_node("<msqrt><mi>z</mi></msqrt>");
    let mut sqrt_output = String::new();
    render_math_node(
        *select_first(&rendered_sqrt, "msqrt").expect("msqrt"),
        &mut sqrt_output,
    );
    assert_eq!(sqrt_output, "sqrt(z)");
    let rendered_root = parse_document_node("<mroot><mi>x</mi><mn>3</mn></mroot>");
    let mut root_output = String::new();
    render_math_node(
        *select_first(&rendered_root, "mroot").expect("mroot"),
        &mut root_output,
    );
    assert_eq!(root_output, "root(x, 3)");
    let mut root_node_output = String::new();
    render_math_node(direct_math.tree.root(), &mut root_node_output);
    assert!(root_node_output.contains("x_i"));
    assert_eq!(wrap_math_operand("a/b"), "(a/b)");

    let mut inline = String::new();
    push_inline_text(&mut inline, "");
    assert!(inline.is_empty());
}
