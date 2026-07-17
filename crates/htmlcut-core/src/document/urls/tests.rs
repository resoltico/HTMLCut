//! Focused URL-rewrite scenarios.

use super::*;
use crate::document::{parse_document_node, select_first};
use scraper::{Node, StrTendril, node::Comment};

#[test]
fn meta_refresh_helper_detects_exact_attribute_names_and_values() {
    let base_document =
        parse_document_node("<base href=\" ../help/ \"><a href=\"guide.html\">Guide</a>");
    assert_eq!(
        document_base_href(&base_document).as_deref(),
        Some("../help/")
    );
    let blank_base_document = parse_document_node("<base href=\"   \">");
    assert_eq!(document_base_href(&blank_base_document), None);
    let fragment_base_document = parse_document_node("<base href=\"#content\">");
    assert_eq!(
        resolve_document_base_url(
            &fragment_base_document,
            Some("https://example.test/docs/start.html")
        )
        .as_deref(),
        Some("https://example.test/docs/start.html")
    );
    let absolute_base_document =
        parse_document_node("<base href=\"https://docs.example.test/help/\">");
    assert_eq!(
        resolve_document_base_url(
            &absolute_base_document,
            Some("https://example.test/docs/start.html")
        )
        .as_deref(),
        Some("https://docs.example.test/help/")
    );
    let relative_base_document = parse_document_node("<base href=\"../help/\">");
    assert_eq!(
        resolve_document_base_url(
            &relative_base_document,
            Some("https://example.test/docs/start.html")
        )
        .as_deref(),
        Some("https://example.test/help/")
    );
    let unsupported_base_document = parse_document_node("<base href=\"ftp://example.test/help/\">");
    assert_eq!(
        resolve_document_base_url(
            &unsupported_base_document,
            Some("https://example.test/docs/start.html")
        )
        .as_deref(),
        Some("https://example.test/docs/start.html")
    );
    assert!(!href_is_meaningful_destination("   "));
    assert!(!href_is_meaningful_destination("#"));
    assert!(!href_is_meaningful_destination("javascript:void(0)"));
    assert!(href_is_meaningful_destination("/guide"));
    assert_eq!(
        rewrite_html_urls("<a href=\"/guide\">Guide</a>", None, false),
        "<a href=\"/guide\">Guide</a>"
    );
    assert_eq!(resolve_url("", Some("https://example.test/base/")), "");
    assert_eq!(
        resolve_url("#fragment", Some("https://example.test/base/")),
        "#fragment"
    );
    assert_eq!(
        resolve_url(
            "mailto:help@example.test",
            Some("https://example.test/base/")
        ),
        "mailto:help@example.test"
    );
    assert_eq!(
        rewrite_attribute_value(
            "img",
            "srcset",
            "hero.jpg 1x, hero@2x.jpg 2x",
            Some("https://example.test/assets/"),
            false,
        ),
        "https://example.test/assets/hero.jpg 1x, https://example.test/assets/hero@2x.jpg 2x"
    );
    assert_eq!(
        rewrite_attribute_value(
            "a",
            "ping",
            "/a /b",
            Some("https://example.test/base/"),
            false,
        ),
        "https://example.test/a https://example.test/b"
    );
    assert_eq!(
        rewrite_attribute_value(
            "meta",
            "content",
            "0; url= /next",
            Some("https://example.test/base/"),
            true,
        ),
        "0; url= https://example.test/next"
    );
    assert_eq!(
        rewrite_attribute_value(
            "meta",
            "content",
            "0; URL=\"/quoted\"",
            Some("https://example.test/base/"),
            true,
        ),
        "0; URL=\"https://example.test/quoted\""
    );
    assert_eq!(
        rewrite_attribute_value(
            "div",
            "data-href",
            "/keep",
            Some("https://example.test/"),
            false
        ),
        "/keep"
    );
    assert_eq!(
        rewrite_srcset_for_tests(
            "data:image/gif;base64,AAAA 1x, /hero@2x.jpg 2x",
            Some("https://example.test/assets/")
        ),
        "data:image/gif;base64,AAAA 1x, https://example.test/hero@2x.jpg 2x"
    );

    let refresh = parse_document_node("<meta http-equiv=\"refresh\" content=\"0; url=next.html\">");
    let refresh_meta = select_first(&refresh, "meta").expect("refresh meta");
    assert!(raw_element_is_meta_refresh(refresh_meta.value()));
    let refresh_upper =
        parse_document_node("<meta http-equiv=\"REFRESH\" content=\"0; url=next.html\">");
    let refresh_upper_meta = select_first(&refresh_upper, "meta").expect("refresh meta");
    assert!(raw_element_is_meta_refresh(refresh_upper_meta.value()));

    let disguised =
        parse_document_node("<meta data-http-equiv=\"refresh\" content=\"0; url=next.html\">");
    let disguised_meta = select_first(&disguised, "meta").expect("disguised meta");
    assert!(!raw_element_is_meta_refresh(disguised_meta.value()));

    assert!(starts_with_ignore_ascii_case("<HTML", "<html"));
    assert!(!starts_with_ignore_ascii_case("ht", "<html"));
}

#[test]
fn css_rewrite_helpers_cover_comments_escapes_and_invalid_forms() {
    assert_eq!(rewrite_css_urls("url(hero.png)", None), "url(hero.png)");
    assert_eq!(
        rewrite_css_urls(
            "/* keep url(old.png) */ url(\"hero.png\")",
            Some("https://example.test/assets/")
        ),
        "/* keep url(old.png) */ url(\"https://example.test/assets/hero.png\")"
    );
    assert_eq!(
        rewrite_css_urls(
            "@import /* note */ 'theme.css' screen; body { background: url( hero.png  ) }",
            Some("https://example.test/assets/")
        ),
        "@import /* note */ 'https://example.test/assets/theme.css' screen; body { background: url( https://example.test/assets/hero.png  ) }"
    );
    assert_eq!(
        rewrite_css_urls(
            "background: myurl(icon.png); list-style: url( \"icon\\\"2.png\" )",
            Some("https://example.test/assets/")
        ),
        "background: myurl(icon.png); list-style: url( \"https://example.test/assets/icon/%222.png\" )"
    );
    assert_eq!(
        rewrite_css_urls(
            "@import url(theme.css); background: url(\"unterminated.png\";",
            Some("https://example.test/assets/")
        ),
        "@import url(https://example.test/assets/theme.css); background: url(\"unterminated.png\";"
    );
    assert_eq!(
        rewrite_css_urls("background: url(   )", Some("https://example.test/assets/")),
        "background: url(   )"
    );
    assert_eq!(
        rewrite_css_urls(
            "background: url hero.png); color: red;",
            Some("https://example.test/assets/")
        ),
        "background: url hero.png); color: red;"
    );
    assert_eq!(
        rewrite_css_urls(
            "background: url(hero.png",
            Some("https://example.test/assets/")
        ),
        "background: url(hero.png"
    );
    assert_eq!(find_css_string_end("\"a\\\"b\"", 0), Some("\"a\\\"b".len()));
    assert_eq!(find_css_string_end("\"escape-at-end\\", 0), None);
    assert_eq!(find_css_string_end("\"unterminated", 0), None);
    assert_eq!(
        css_comment_end("/* unterminated", 0),
        Some("/* unterminated".len())
    );
    assert_eq!(
        rewrite_css_url_function_at("url hero.png)", 0, "https://example.test/assets/"),
        None
    );
    assert_eq!(
        rewrite_css_url_function_at("url(hero.png", 0, "https://example.test/assets/"),
        None
    );
    assert_eq!(
        rewrite_css_import_string_at("@media screen", 0, "https://example.test/assets/"),
        None
    );
    assert_eq!(skip_ascii_whitespace("x", 1), 1);
    assert!(is_css_identifier_char('-'));
    assert!(is_css_identifier_char('_'));

    let mut document = parse_document_node(
        "<style>/* keep */ @import \"theme.css\"; .hero { background: url('../img/card.png') }</style>",
    );
    let style_id = select_first(&document, "style").expect("style").id();
    rewrite_urls_in_document_with_node_ids_for_tests(
        &mut document,
        "https://example.test/docs/articles/",
        vec![style_id],
    );
    let serialized = crate::document::serialize_document(&document);
    assert!(serialized.contains("@import \"https://example.test/docs/articles/theme.css\""));
    assert!(serialized.contains("url('https://example.test/docs/img/card.png')"));

    let mut style_document =
        parse_document_node("<style>.hero { background: url(hero.png) }</style>");
    let style_id = select_first(&style_document, "style").expect("style").id();
    rewrite_urls_in_document_with_node_ids_for_tests(
        &mut style_document,
        "https://example.test/assets/",
        vec![style_id],
    );
    assert!(
        crate::document::serialize_document(&style_document)
            .contains("url(https://example.test/assets/hero.png)")
    );

    let mut unchanged_style_document = parse_document_node("<style>.hero { color: red }</style>");
    let style_id = select_first(&unchanged_style_document, "style")
        .expect("style")
        .id();
    rewrite_urls_in_document_with_node_ids_for_tests(
        &mut unchanged_style_document,
        "https://example.test/assets/",
        vec![style_id],
    );
    assert!(
        crate::document::serialize_document(&unchanged_style_document)
            .contains(".hero { color: red }")
    );

    let mut style_with_comment =
        parse_document_node("<style>.hero { background: url(hero.png) }</style>");
    let style_id = select_first(&style_with_comment, "style")
        .expect("style")
        .id();
    style_with_comment
        .tree
        .get_mut(style_id)
        .expect("style node")
        .append(Node::Comment(Comment {
            comment: StrTendril::from("kept"),
        }));
    rewrite_urls_in_document_with_node_ids_for_tests(
        &mut style_with_comment,
        "https://example.test/assets/",
        vec![style_id],
    );
    assert!(
        crate::document::serialize_document(&style_with_comment)
            .contains("https://example.test/assets/hero.png")
    );
}
