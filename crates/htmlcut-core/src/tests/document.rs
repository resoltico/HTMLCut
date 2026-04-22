use super::*;

#[test]
fn rendering_and_url_helpers_cover_remaining_paths() {
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
    assert!(!attribute_supports_url_rewrite("class"));
    assert!(first_fragment_attributes("plain text", None, false).is_empty());
    let non_refresh_meta = parse_document_node(
        "<meta http-equiv=\"content-security-policy\" content=\"0; url=next.html\">",
    );
    let meta = select_first(&non_refresh_meta, "meta").expect("meta");
    assert_eq!(
        element_attributes(&meta, Some("https://example.com/base/"), true).get("content"),
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

    let rendered = render_html_as_text(
        "<article><p>Hello</p><ul><li>One</li></ul><hr><pre>  keep\n  spacing</pre></article>",
        WhitespaceMode::Preserve,
    );
    assert!(rendered.contains("Hello"));
    assert!(rendered.contains("- One"));
    assert!(rendered.contains("---"));
    assert!(rendered.contains("  keep"));
    let richer_rendered = render_html_as_text(
        "<blockquote><p>Quote</p></blockquote><dl><dt>Term</dt><dd>Definition</dd></dl><p>Use <code>cargo test</code></p>",
        WhitespaceMode::Preserve,
    );
    assert!(richer_rendered.contains("> Quote"));
    assert!(richer_rendered.contains("Term\n: Definition"));
    assert!(richer_rendered.contains("`cargo test`"));
    let collapsed_blockquote = render_html_as_text(
        "<blockquote><p>First</p><p></p><p></p><p>Second</p></blockquote>",
        WhitespaceMode::Preserve,
    );
    assert_eq!(collapsed_blockquote, "> First\n>\n> Second");
    let empty_blockquote =
        render_html_as_text("<blockquote>   </blockquote>", WhitespaceMode::Preserve);
    assert!(empty_blockquote.is_empty());

    assert_eq!(
        collapse_inline_whitespace("  Hello   world "),
        "Hello world"
    );
    assert!(needs_space("Hello", "world"));
    assert!(!needs_space("", "world"));
    assert!(!needs_space("Hello", ""));
    assert!(!needs_space("Hello", "."));
    assert!(!needs_space("Hello ", "world"));
    assert!(!needs_space("-", "world"));

    let mut output = String::from("Hello\n\n");
    push_newline(&mut output, 2);
    assert_eq!(output, "Hello\n\n");

    assert_eq!(
        apply_whitespace_mode(" Hello \n\n World ", WhitespaceMode::Normalize),
        "Hello\n\nWorld"
    );
    assert_eq!(
        apply_whitespace_mode("A\n\nB", WhitespaceMode::Normalize),
        "A\n\nB"
    );
    assert!(looks_like_full_document("<html><body></body></html>"));
    assert_eq!(
        rewrite_html_urls(
            "<a href=\"guide.html\">Guide</a>",
            Some("https://example.com/docs/"),
            false
        ),
        "<a href=\"https://example.com/docs/guide.html\">Guide</a>"
    );
    assert_eq!(resolve_url("#frag", Some("https://example.com")), "#frag");
    assert_eq!(
        resolve_url("https://openai.com", Some("https://example.com")),
        "https://openai.com"
    );
    assert_eq!(resolve_url("guide.html", None), "guide.html");
    assert_eq!(resolve_url("guide.html", Some("not a url")), "guide.html");
    assert_eq!(
        rewrite_html_urls("<p>Hello</p>", None, false),
        "<p>Hello</p>"
    );
    assert!(!looks_like_full_document("<body>Hello</body>"));
    assert!(first_body(&parse_wrapped_fragment("<p>Hello</p>")).is_some());
    assert!(first_body_child_element(&parse_wrapped_fragment("plain")).is_none());
    assert!(build_preview(&json!({"k": "v"}), 5).ends_with(ELLIPSIS));
    assert!(!has_errors(&[warning_diagnostic(
        DiagnosticCode::MultipleMatches,
        "x",
        None
    )]));
    assert!(has_errors(&[error_diagnostic(
        DiagnosticCode::SourceLoadFailed,
        "x",
        None
    )]));
    assert_eq!(
        warning_diagnostic(DiagnosticCode::MultipleMatches, "x", None).level,
        DiagnosticLevel::Warning
    );

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
        "<img srcset=\"small.png 1x, large.png 2x\"><form action=\"submit\"><button formaction=\"override\"></button></form><video poster=\"poster.png\"></video><a ping=\"/hit-one /hit-two\">Track</a><meta http-equiv=\"refresh\" content=\"0; url=next.html\">",
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
    let rewritten_single_srcset = rewrite_html_urls(
        "<img srcset=\"plain.png\">",
        Some("https://example.com/docs/"),
        false,
    );
    assert!(rewritten_single_srcset.contains("srcset=\"https://example.com/docs/plain.png\""));
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

#[test]
fn document_base_resolution_covers_absolute_relative_and_fallback_paths() {
    let absolute_document = parse_document_node(
        "<html><head><base href=\"https://cdn.example.com/shared/\"></head><body></body></html>",
    );
    assert_eq!(
        resolve_document_base_url(
            &absolute_document,
            Some("https://example.com/docs/start.html")
        )
        .as_deref(),
        Some("https://cdn.example.com/shared/")
    );

    let relative_document =
        parse_document_node("<html><head><base href=\"../shared/\"></head><body></body></html>");
    assert_eq!(
        resolve_document_base_url(
            &relative_document,
            Some("https://example.com/docs/start.html")
        )
        .as_deref(),
        Some("https://example.com/shared/")
    );

    assert_eq!(
        resolve_document_base_url(&relative_document, Some("not a url")).as_deref(),
        Some("not a url")
    );
}

#[test]
fn document_base_resolution_ignores_fragment_only_base_hrefs() {
    let document = parse_document_node(
        "<html><head><base href=\"#chapter-1\"></head><body><a href=\"guide.html\">Guide</a></body></html>",
    );

    assert_eq!(
        resolve_document_base_url(&document, Some("https://example.com/docs/start.html"))
            .as_deref(),
        Some("https://example.com/docs/start.html")
    );
}

#[test]
fn document_base_resolution_rejects_unsupported_absolute_schemes() {
    let document = parse_document_node(
        "<html><head><base href=\"mailto:owner@example.com\"></head><body></body></html>",
    );

    assert_eq!(
        resolve_document_base_url(&document, Some("https://example.com/docs/start.html"))
            .as_deref(),
        Some("https://example.com/docs/start.html")
    );
}

#[test]
fn selector_and_slice_runs_collect_builder_errors() {
    let selector_request = selector_request("<article data-id=\"7\">Hello</article>");
    let selector_loaded =
        load_source(&selector_request.source, &RuntimeOptions::default()).expect("loaded");
    let mut invalid_selector_request = selector_request.clone();
    invalid_selector_request.extraction = ExtractionSpec::selector(selector_query("["));
    let selector_run = run_selector_extraction(&invalid_selector_request, &selector_loaded);
    assert!(selector_run.matches.is_empty());
    assert_eq!(selector_run.diagnostics[0].code, "INVALID_SELECTOR");

    let slice_request = ExtractionRequest::new(
        memory_source_with_base(
            "inline",
            "<a href=\"/x\">Hello</a>",
            "https://example.com/base/",
        ),
        ExtractionSpec::slice(slice_spec("<a", "</a>").with_boundary_inclusion(true, true))
            .with_value(attribute_value("title")),
    );
    let selector_loaded =
        load_source(&slice_request.source, &RuntimeOptions::default()).expect("loaded");
    let slice_run = run_slice_extraction(&slice_request, &selector_loaded);
    assert!(slice_run.matches.is_empty());
    assert_eq!(slice_run.diagnostics[0].code, "MISSING_ATTRIBUTE");

    let selector_missing_attribute_request = ExtractionRequest::new(
        memory_source("inline", "<article data-id=\"7\">Hello</article>"),
        ExtractionSpec::selector(selector_query("article")).with_value(attribute_value("title")),
    );
    let selector_missing_attribute_loaded = load_source(
        &selector_missing_attribute_request.source,
        &RuntimeOptions::default(),
    )
    .expect("loaded");
    let selector_missing_attribute_run = run_selector_extraction(
        &selector_missing_attribute_request,
        &selector_missing_attribute_loaded,
    );
    assert!(selector_missing_attribute_run.matches.is_empty());
    assert_eq!(
        selector_missing_attribute_run.diagnostics[0].code,
        "MISSING_ATTRIBUTE"
    );
}

#[test]
fn source_helpers_cover_remaining_unreachable_and_locator_paths() {
    let wrong_url_kind = catch_unwind(AssertUnwindSafe(|| {
        let _ = read_url_source(&file_source("fixture.html"), &RuntimeOptions::default());
    }));
    assert!(wrong_url_kind.is_err());

    let wrong_file_kind = catch_unwind(AssertUnwindSafe(|| {
        let _ = read_file_source(
            &url_source("https://example.com"),
            &RuntimeOptions::default(),
        );
    }));
    assert!(wrong_file_kind.is_err());

    let file_metadata = empty_source_metadata(
        &file_source("fixtures/input.html")
            .with_base_url(Url::parse("https://example.com/base/").expect("base")),
    );
    assert_eq!(file_metadata.value, "fixtures/input.html");
    assert_eq!(
        file_metadata.input_base_url.as_deref(),
        Some("https://example.com/base/")
    );

    let stdin_metadata = empty_source_metadata(&SourceRequest::stdin());
    assert_eq!(stdin_metadata.value, "-");
    assert_eq!(stdin_metadata.kind, SourceKind::Stdin);

    let unnamed_memory_metadata =
        empty_source_metadata(&SourceRequest::memory("   ", "<article>Hello</article>"));
    assert_eq!(unnamed_memory_metadata.value, "memory");
}

#[cfg(unix)]
#[test]
fn read_file_source_reports_permission_denied_reads() {
    let tempdir = htmlcut_tempdir::tempdir().expect("tempdir");
    let unreadable_path = tempdir.path().join("unreadable.html");
    std::fs::write(&unreadable_path, "<article>Hello</article>").expect("write html");

    let mut permissions = std::fs::metadata(&unreadable_path)
        .expect("metadata")
        .permissions();
    permissions.set_mode(0o000);
    std::fs::set_permissions(&unreadable_path, permissions).expect("chmod 000");

    let error = read_file_source(&file_source(&unreadable_path), &RuntimeOptions::default())
        .expect_err("permission denied");
    assert_eq!(error.code, "SOURCE_LOAD_FAILED");
    assert!(error.message.contains("Could not read file"));
}
