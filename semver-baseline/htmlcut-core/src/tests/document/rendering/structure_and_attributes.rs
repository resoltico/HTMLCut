use super::*;

#[test]
fn document_structure_and_url_attributes_preserve_semantics() {
    let node = parse_document_node("<article data-id=\"7\"><p>Hello</p></article>");
    assert!(serialize_document(&node).contains("Hello"));
    assert!(
        serialize_children(&select_first(&node, "article").expect("article"))
            .contains("<p>Hello</p>")
    );
    let article = select_first(&node, "article").expect("article");
    assert_eq!(
        build_node_path(&select_first(&node, "p").expect("p")),
        "html:nth-of-type(1) > body:nth-of-type(1) > article:nth-of-type(1) > p:nth-of-type(1)"
    );
    assert_eq!(element_name(node.tree.root()), None);
    assert_eq!(element_name(*article), Some("article".to_owned()));
    assert_eq!(
        element_attributes(&article, Some("https://example.com/base/"), false).get("data-id"),
        Some(&"7".to_owned())
    );
    let linked = parse_document_node(
        "<a class=\"card featured\" href=\"guide.html\" data-track=\"hero\">Guide</a>",
    );
    let anchor = select_first(&linked, "a").expect("anchor");
    let rewritten_attributes = element_attributes(&anchor, Some("https://example.com/base/"), true);
    assert_eq!(
        rewritten_attributes.get("href"),
        Some(&"https://example.com/base/guide.html".to_owned())
    );
    assert_eq!(
        rewritten_attributes.get("class"),
        Some(&"card featured".to_owned())
    );
    assert_eq!(
        rewritten_attributes.get("data-track"),
        Some(&"hero".to_owned())
    );
    assert!(attribute_supports_url_rewrite("href"));
    assert!(attribute_supports_url_rewrite("srcset"));
    assert!(attribute_supports_url_rewrite("ping"));
    assert!(attribute_supports_url_rewrite("style"));
    assert!(!attribute_supports_url_rewrite("class"));
    assert_eq!(
        rewrite_css_urls_for_tests(
            "background-image: url('../img/hero.png'); @import \"theme.css\";",
            Some("https://example.com/docs/articles/")
        ),
        "background-image: url('https://example.com/docs/img/hero.png'); @import \"https://example.com/docs/articles/theme.css\";"
    );
    assert!(first_fragment_attributes("plain text", None, false).is_empty());
    assert_eq!(
        first_fragment_attributes(
            "<a href=\"guide.html\" title=\"Guide\">Guide</a>",
            Some("https://example.com/base/"),
            true
        )
        .get("href"),
        Some(&"https://example.com/base/guide.html".to_owned())
    );
    let non_refresh_meta = parse_document_node(
        "<meta http-equiv=\"content-security-policy\" content=\"0; url=next.html\">",
    );
    let meta = select_first(&non_refresh_meta, "meta").expect("meta");
    assert_eq!(
        element_attributes(&meta, Some("https://example.com/base/"), true).get("content"),
        Some(&"0; url=next.html".to_owned())
    );
    let disguised_refresh_meta =
        parse_document_node("<meta data-http-equiv=\"refresh\" content=\"0; url=next.html\">");
    let disguised_meta = select_first(&disguised_refresh_meta, "meta").expect("meta");
    assert_eq!(
        element_attributes(&disguised_meta, Some("https://example.com/base/"), true).get("content"),
        Some(&"0; url=next.html".to_owned())
    );

    let mut detached_document = parse_document_node("<article><p>Hello</p></article>");
    let detached_id = {
        let detached = select_first(&detached_document, "p").expect("p");
        detached.id()
    };
    detached_document
        .tree
        .get_mut(detached_id)
        .expect("detached node")
        .detach();
    let detached = ElementRef::wrap(
        detached_document
            .tree
            .get(detached_id)
            .expect("detached ref"),
    )
    .expect("element ref");
    assert_eq!(build_node_path(&detached), "p:nth-of-type(1)");
}
