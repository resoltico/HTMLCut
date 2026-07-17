use super::super::*;
use crate::contracts::WhitespaceMode;
use crate::document::{parse_document_node, select_first};

use super::super::math::*;

#[test]
fn math_fallback_paths_cover_unrenderable_nodes_and_operand_guards() {
    assert_eq!(
        render_html_as_text(
            "<article><math><annotation>ignored</annotation></math><p>Body</p></article>",
            WhitespaceMode::Rendered,
        ),
        "Body"
    );
    assert_eq!(
        render_html_as_text(
            "<h2><math><annotation>ignored</annotation></math>Heading</h2>",
            WhitespaceMode::Rendered,
        ),
        "## Heading"
    );

    let hidden_heading =
        parse_document_node("<h2><span style=\"display:none\">Hidden</span>Visible</h2>");
    let hidden_heading_element = select_first(&hidden_heading, "h2").expect("heading");
    assert_eq!(
        extract_heading_text(&hidden_heading_element).as_deref(),
        Some("Visible")
    );

    let malformed_fraction =
        parse_document_node("<mfrac><annotation>ignored</annotation><mi>b</mi></mfrac>");
    let mut malformed_fraction_output = String::new();
    render_math_node(
        *select_first(&malformed_fraction, "mfrac").expect("mfrac"),
        &mut malformed_fraction_output,
    );
    assert_eq!(malformed_fraction_output, "b");

    let malformed_sub =
        parse_document_node("<msub><mi>a</mi><annotation>ignored</annotation></msub>");
    let mut malformed_sub_output = String::new();
    render_math_node(
        *select_first(&malformed_sub, "msub").expect("msub"),
        &mut malformed_sub_output,
    );
    assert_eq!(malformed_sub_output, "a");

    let malformed_sup =
        parse_document_node("<msup><annotation>ignored</annotation><mn>2</mn></msup>");
    let mut malformed_sup_output = String::new();
    render_math_node(
        *select_first(&malformed_sup, "msup").expect("msup"),
        &mut malformed_sup_output,
    );
    assert_eq!(malformed_sup_output, "2");

    let malformed_subsup = parse_document_node(
        "<msubsup><annotation>ignored</annotation><mi>i</mi><mn>2</mn></msubsup>",
    );
    let mut malformed_subsup_output = String::new();
    render_math_node(
        *select_first(&malformed_subsup, "msubsup").expect("msubsup"),
        &mut malformed_subsup_output,
    );
    assert_eq!(malformed_subsup_output, "i 2");

    let malformed_sqrt = parse_document_node("<msqrt><annotation>ignored</annotation></msqrt>");
    let mut malformed_sqrt_output = String::new();
    render_math_node(
        *select_first(&malformed_sqrt, "msqrt").expect("msqrt"),
        &mut malformed_sqrt_output,
    );
    assert!(malformed_sqrt_output.is_empty());

    let malformed_root =
        parse_document_node("<mroot><mi>x</mi><annotation>ignored</annotation></mroot>");
    let mut malformed_root_output = String::new();
    render_math_node(
        *select_first(&malformed_root, "mroot").expect("mroot"),
        &mut malformed_root_output,
    );
    assert_eq!(malformed_root_output, "x");

    let denominator_empty =
        parse_document_node("<mfrac><mi>a</mi><annotation>ignored</annotation></mfrac>");
    assert_eq!(
        render_math_fraction(*select_first(&denominator_empty, "mfrac").expect("mfrac")),
        None
    );
    let right_empty =
        parse_document_node("<msub><mi>a</mi><annotation>ignored</annotation></msub>");
    assert_eq!(
        render_math_binary_operator(*select_first(&right_empty, "msub").expect("msub"), "_"),
        None
    );
    let base_empty = parse_document_node(
        "<msubsup><annotation>ignored</annotation><mi>b</mi><mi>c</mi></msubsup>",
    );
    assert_eq!(
        render_math_subsup(*select_first(&base_empty, "msubsup").expect("msubsup")),
        None
    );
    let sup_empty = parse_document_node(
        "<msubsup><mi>a</mi><mi>b</mi><annotation>ignored</annotation></msubsup>",
    );
    assert_eq!(
        render_math_subsup(*select_first(&sup_empty, "msubsup").expect("msubsup")),
        None
    );
    let root_degree_empty =
        parse_document_node("<mroot><mi>x</mi><annotation>ignored</annotation></mroot>");
    assert_eq!(
        render_math_root(*select_first(&root_degree_empty, "mroot").expect("mroot")),
        None
    );

    assert_eq!(wrap_math_operand("a b"), "(a b)");
    assert_eq!(wrap_math_operand("x^2"), "(x^2)");
    assert_eq!(wrap_math_operand("x_i"), "(x_i)");
}
