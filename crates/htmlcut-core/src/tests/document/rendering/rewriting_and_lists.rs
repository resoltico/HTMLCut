use super::*;

#[test]
fn url_rewriting_and_list_rendering_cover_edge_cases() {
    let rewritten_document = rewrite_html_urls(
        "<!DOCTYPE html><html><body><a href=\"guide.html\">Guide</a></body></html>",
        Some("https://example.com/docs/"),
        false,
    );
    assert!(rewritten_document.contains("https://example.com/docs/guide.html"));
    let forced_document = rewrite_html_urls(
        "<img src=\"asset.png\">",
        Some("https://example.com/docs/"),
        true,
    );
    assert!(forced_document.contains("https://example.com/docs/asset.png"));
    let rewritten_url_attributes = rewrite_html_urls(
        "<img srcset=\"small.png 1x, large.png 2x\"><form action=\"submit\"><button formaction=\"override\"></button></form><video poster=\"poster.png\"></video><a ping=\"/hit-one /hit-two\">Track</a><meta http-equiv=\"refresh\" content=\"0; url=next.html\"><div style=\"background-image:url('../img/hero.png')\"></div><style>@import \"theme.css\"; .hero { background: url(\"../img/card.png\") }</style>",
        Some("https://example.com/docs/"),
        false,
    );
    assert!(rewritten_url_attributes.contains(
        "srcset=\"https://example.com/docs/small.png 1x, https://example.com/docs/large.png 2x\""
    ));
    assert!(rewritten_url_attributes.contains("action=\"https://example.com/docs/submit\""));
    assert!(rewritten_url_attributes.contains("formaction=\"https://example.com/docs/override\""));
    assert!(rewritten_url_attributes.contains("poster=\"https://example.com/docs/poster.png\""));
    assert!(
        rewritten_url_attributes
            .contains("ping=\"https://example.com/hit-one https://example.com/hit-two\"")
    );
    assert!(
        rewritten_url_attributes.contains("content=\"0; url=https://example.com/docs/next.html\"")
    );
    assert!(
        rewritten_url_attributes
            .contains("style=\"background-image:url('https://example.com/img/hero.png')\"")
    );
    assert!(rewritten_url_attributes.contains(
        "@import \"https://example.com/docs/theme.css\"; .hero { background: url(\"https://example.com/img/card.png\") }"
    ));
    let nested_only_list_rendered = render_html_as_text(
        "<article><ul><li><ul><li>Nested</li></ul></li></ul></article>",
        WhitespaceMode::Rendered,
    );
    assert_eq!(nested_only_list_rendered, "    - Nested");
    let multiline_list_rendered = render_html_as_text(
        "<article><ul><li><p>First</p><p>Second</p></li></ul></article>",
        WhitespaceMode::Rendered,
    );
    assert_eq!(multiline_list_rendered, "- First\n\n  Second");
    let inline_semantics_list_rendered = render_html_as_text(
        "<article><ul><li><strong>Accommodation:</strong> Accommodation requires consent to the <a href=\"https://www.airbnb.com/terms\">Airbnb Terms of Service</a>.</li></ul></article>",
        WhitespaceMode::Rendered,
    );
    assert_eq!(
        inline_semantics_list_rendered,
        "- Accommodation: Accommodation requires consent to the Airbnb Terms of Service [https://www.airbnb.com/terms]."
    );
    let inline_link_list_rendered = render_html_as_text(
        "<article><ul><li>By email: <a href=\"mailto:Central.Complaints@aon.co.uk\">Central.Complaints@aon.co.uk</a></li></ul></article>",
        WhitespaceMode::Rendered,
    );
    assert_eq!(
        inline_link_list_rendered,
        "- By email: Central.Complaints@aon.co.uk [mailto:Central.Complaints@aon.co.uk]"
    );
    let inline_then_nested_list_rendered = render_html_as_text(
        "<article><ul><li><strong>Recording:</strong> damages that arise from:<ol><li>One</li><li>Two</li></ol></li></ul></article>",
        WhitespaceMode::Rendered,
    );
    assert_eq!(
        inline_then_nested_list_rendered,
        "- Recording: damages that arise from:\n    1. One\n    2. Two"
    );
    let rewritten_compact_refresh = rewrite_html_urls(
        "<meta http-equiv=\"refresh\" content=\"0;url=next.html\">",
        Some("https://example.com/docs/"),
        false,
    );
    assert!(
        rewritten_compact_refresh.contains("content=\"0;url=https://example.com/docs/next.html\"")
    );
    let rewritten_single_srcset = rewrite_html_urls(
        "<img srcset=\"plain.png\">",
        Some("https://example.com/docs/"),
        false,
    );
    assert!(rewritten_single_srcset.contains("srcset=\"https://example.com/docs/plain.png\""));
    let foreign_node_ids = parse_document_node(
        "<div><a href=\"one.html\">One</a><a href=\"two.html\">Two</a><a href=\"three.html\">Three</a></div>",
    )
    .tree
    .nodes()
    .map(|node| node.id())
    .collect::<Vec<_>>();
    let mut tiny_document = parse_document_node("<p>Small</p>");
    rewrite_urls_in_document_with_node_ids_for_tests(
        &mut tiny_document,
        "https://example.com/docs/",
        foreign_node_ids,
    );
    assert!(serialize_document(&tiny_document).contains("Small"));
    let unchanged_empty_srcset = rewrite_html_urls(
        "<img srcset=\" , \">",
        Some("https://example.com/docs/"),
        false,
    );
    assert!(unchanged_empty_srcset.contains("srcset=\" , \""));
    assert_eq!(collapse_blank_lines_for_tests("A\n\n\n\nB"), "A\n\nB");
    assert_eq!(
        rewrite_srcset_for_tests("plain.png, second.png", Some("https://example.com/docs/")),
        "https://example.com/docs/plain.png, https://example.com/docs/second.png"
    );
    assert_eq!(
        rewrite_srcset_for_tests(
            "data:image/svg+xml,<svg></svg> 1x, plain.png 2x",
            Some("https://example.com/docs/"),
        ),
        "data:image/svg+xml,<svg></svg> 1x, https://example.com/docs/plain.png 2x"
    );
    let rewritten_double_quoted_refresh = rewrite_html_urls(
        "<meta http-equiv=\"refresh\" content='0; url=\"next.html\"'>",
        Some("https://example.com/docs/"),
        false,
    );
    assert!(
        rewritten_double_quoted_refresh
            .contains("content=\"0; url=&quot;https://example.com/docs/next.html&quot;\"")
    );
    let rewritten_single_quoted_refresh = rewrite_html_urls(
        "<meta http-equiv=\"refresh\" content=\"0; url='other.html'\">",
        Some("https://example.com/docs/"),
        false,
    );
    assert!(
        rewritten_single_quoted_refresh
            .contains("content=\"0; url='https://example.com/docs/other.html'\"")
    );
    let non_meta_content = rewrite_html_urls(
        "<div content=\"0; url=next.html\"></div>",
        Some("https://example.com/docs/"),
        false,
    );
    assert!(non_meta_content.contains("content=\"0; url=next.html\""));
    let meta_without_refresh = rewrite_html_urls(
        "<meta content=\"0; url=next.html\">",
        Some("https://example.com/docs/"),
        false,
    );
    assert!(meta_without_refresh.contains("content=\"0; url=next.html\""));
    let mut mutable_document =
        parse_wrapped_fragment("<img src=\"asset.png\"><a href=\"guide.html\">Guide</a>");
    rewrite_urls_in_document(&mut mutable_document, "https://example.com/docs/");
    let rewritten_body = first_body(&mutable_document).expect("body");
    assert!(serialize_children(&rewritten_body).contains("https://example.com/docs/asset.png"));
    assert!(serialize_children(&rewritten_body).contains("https://example.com/docs/guide.html"));

    let source_meta = source_metadata(
        &LoadedSource {
            kind: SourceKind::Memory,
            value: "inline".to_owned(),
            bytes_read: 5,
            text: "Hello".to_owned(),
            input_base_url: None,
            load_steps: Vec::new(),
        },
        false,
        None,
    );
    assert!(source_meta.text.is_none());
}
