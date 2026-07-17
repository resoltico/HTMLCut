use super::*;

#[test]
fn element_rendering_handles_empty_and_detached_document_nodes() {
    let mut empty_output = String::new();
    let whitespace_fragment = parse_wrapped_fragment("   ");
    let whitespace_node = first_body(&whitespace_fragment)
        .expect("body")
        .first_child()
        .expect("text child");
    render_node(whitespace_node, &mut empty_output, false, false);
    assert!(empty_output.is_empty());

    let comment_fragment = parse_wrapped_fragment("<!-- keep nothing -->");
    let comment_node = first_body(&comment_fragment)
        .expect("body")
        .first_child()
        .expect("comment child");
    render_node(comment_node, &mut empty_output, false, false);
    assert!(empty_output.is_empty());

    let script_document = parse_wrapped_fragment("<script>alert(1)</script>");
    let script = select_first(&script_document, "script").expect("script");
    render_node(*script, &mut empty_output, false, false);
    assert!(empty_output.is_empty());

    let empty_code_document = parse_wrapped_fragment("<p>Use <code>   </code></p>");
    let empty_code = select_first(&empty_code_document, "code").expect("code");
    let mut empty_code_output = String::from("Use");
    render_node(*empty_code, &mut empty_code_output, false, false);
    assert_eq!(empty_code_output, "Use");

    let inline_code_document = parse_wrapped_fragment("<p>Use <code>cargo test</code></p>");
    let inline_code = select_first(&inline_code_document, "code").expect("code");
    let mut inline_code_output = String::from("Use");
    render_node(*inline_code, &mut inline_code_output, false, false);
    assert_eq!(inline_code_output, "Use `cargo test`");
    let mut inline_code_without_extra_space = String::from("Use ");
    render_node(
        *inline_code,
        &mut inline_code_without_extra_space,
        false,
        false,
    );
    assert_eq!(inline_code_without_extra_space, "Use `cargo test`");
    let pre_code_document = parse_wrapped_fragment("<pre><code>cargo test</code></pre>");
    let pre_code = select_first(&pre_code_document, "code").expect("code");
    let mut pre_code_output = String::new();
    render_node(*pre_code, &mut pre_code_output, true, false);
    assert_eq!(pre_code_output, "cargo test");

    let br_document = parse_wrapped_fragment("<br>");
    let br = select_first(&br_document, "br").expect("br");
    let mut line_output = String::from("Hello");
    render_node(*br, &mut line_output, false, false);
    assert_eq!(line_output, "Hello\n");

    let mut default_output = String::new();
    let default_document = parse_wrapped_fragment("<p>Hello</p>");
    let default_node = first_body(&default_document)
        .expect("body")
        .first_child()
        .expect("first child");
    render_node(default_node, &mut default_output, false, false);
    assert!(default_output.contains("Hello"));
    let list_item_document = parse_wrapped_fragment("<li><p>Hello</p></li>");
    let list_item = select_first(&list_item_document, "li").expect("list item");
    let mut list_item_output = String::new();
    render_node(*list_item, &mut list_item_output, false, false);
    assert!(list_item_output.contains("- Hello"));
    let mut detached_list_document = parse_wrapped_fragment("<ol><li>Detached</li></ol>");
    let detached_list_id = {
        let detached = select_first(&detached_list_document, "li").expect("list item");
        detached.id()
    };
    detached_list_document
        .tree
        .get_mut(detached_list_id)
        .expect("detached list item")
        .detach();
    let detached_list_item = ElementRef::wrap(
        detached_list_document
            .tree
            .get(detached_list_id)
            .expect("detached list ref"),
    )
    .expect("detached list element");
    let mut detached_list_output = String::new();
    render_node(*detached_list_item, &mut detached_list_output, false, false);
    assert!(detached_list_output.contains("- Detached"));
    let selected_semantics_document =
        parse_document_node("<img src=\"hero.png\" alt=\"Hero\"><pre>line 1\n  line 2</pre>");
    let selected_image = select_first(&selected_semantics_document, "img").expect("img");
    let selected_pre = select_first(&selected_semantics_document, "pre").expect("pre");
    assert_eq!(
        render_element_as_text(&selected_image, WhitespaceMode::Rendered),
        "Hero"
    );
    assert_eq!(
        render_element_as_text(&selected_pre, WhitespaceMode::Rendered),
        "line 1\n  line 2"
    );
    let utility_like_fragment =
        parse_wrapped_fragment("<p class=\"status pricing report\">All Systems Operational</p>");
    let utility_like_root = select_first(&utility_like_fragment, "p").expect("status root");
    assert_eq!(
        render_document_body_as_text(&utility_like_fragment, WhitespaceMode::Rendered),
        ""
    );
    assert_eq!(
        render_selected_document_body_as_text(&utility_like_fragment, WhitespaceMode::Rendered),
        "All Systems Operational"
    );
    assert_eq!(
        render_element_as_text(&utility_like_root, WhitespaceMode::Rendered),
        "All Systems Operational"
    );
    let selected_nav_document = parse_document_node("<nav><a href=\"/docs\">Docs</a></nav>");
    let selected_nav = select_first(&selected_nav_document, "nav").expect("nav");
    assert_eq!(
        render_element_as_text(&selected_nav, WhitespaceMode::Rendered),
        "Docs [/docs]"
    );
    let pre_document = parse_wrapped_fragment("<pre>  keep   spacing</pre>");
    let pre = select_first(&pre_document, "pre").expect("pre");
    let mut pre_output = String::new();
    render_node(*pre, &mut pre_output, false, false);
    assert!(pre_output.contains("  keep   spacing"));
    let nested_pre_document = parse_wrapped_fragment("<pre><span>Hello</span></pre>");
    let span = select_first(&nested_pre_document, "span").expect("span");
    let mut nested_pre_output = String::new();
    render_node(*span, &mut nested_pre_output, true, false);
    assert_eq!(nested_pre_output, "Hello");
    let mut document_output = String::new();
    render_node(
        default_document.tree.root(),
        &mut document_output,
        false,
        false,
    );
    assert!(document_output.contains("Hello"));
}
