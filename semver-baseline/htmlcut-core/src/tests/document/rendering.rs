use super::*;
use scraper::Html;

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
    assert_eq!(
        render_html_as_text(
            "<article><p>Ratio <span style=\"display:none\"><math><mfrac><mn>3</mn><mn>2</mn></mfrac></math></span><img aria-hidden=\"true\" alt=\"{\\\\textstyle {\\\\frac {3}{2}}}\" src=\"math.svg\"></p></article>",
            WhitespaceMode::Rendered,
        ),
        "Ratio 3/2"
    );
    assert_eq!(
        render_html_as_text(
            "<main>\
                <nav class=\"page-tools\"><a href=\"/edit\">Edit</a></nav>\
                <div class=\"live-story-filter-tags\"><button>All</button><button>catch up</button></div>\
                <div class=\"live-story__post-count\"><span>7 Posts</span></div>\
                <div class=\"social-share_compact\">\
                    <a class=\"social-share_compact__share\" href=\"mailto:?subject=Hello&amp;body=World\"><svg></svg></a>\
                    <div class=\"social-share_compact__copied\">Link Copied!</div>\
                </div>\
                <div class=\"featured-video\" data-video-id=\"123\"><div class=\"video-player\"><a href=\"https://example.com/video\">Watch</a></div><div class=\"caption\"><h4>Video title</h4><p>Video caption.</p></div></div>\
                <div class=\"mw-editsection-bracket\">[</div><div class=\"mw-editsection-bracket\">]</div>\
                <article><h2>Story</h2><p>Body <a href=\"https://example.com/guide\">Guide</a></p></article>\
                <section class=\"related-topics\"><h3>Related Topics</h3><a href=\"/other\">Other</a></section>\
                <footer><h3>More from here</h3><a href=\"/other\">Other</a></footer>\
             </main>",
            WhitespaceMode::Rendered,
        ),
        "## Story\n\nBody Guide [https://example.com/guide]"
    );
    assert_eq!(
        render_html_as_text(
            "<article>\
                <header class=\"article-header\">\
                    <div class=\"eyebrow\"><a href=\"/category\">Updates</a></div>\
                    <h1>Primary Title</h1>\
                    <div class=\"author-byline\">By Reporter</div>\
                </header>\
                <div class=\"notice\"><span class=\"flag\">NEW</span> playback available</div>\
                <p>Body paragraph.</p>\
                <p><a href=\"/background\"><strong>BACKGROUND READING FOR THIS TOPIC</strong></a></p>\
                <div class=\"author-bio\"><p>Reporter bio.</p></div>\
                <div class=\"catlinks\"><a href=\"/category\">Category</a></div>\
            </article>",
            WhitespaceMode::Rendered,
        ),
        "# Primary Title\n\nBody paragraph."
    );
    assert_eq!(
        render_html_as_text(
            "<article><span>When you purchase through links on our site, we may earn an affiliate commission. <a href=\"/terms\">Here’s how it works</a>.</span><h1>Story</h1><p>Body paragraph.</p></article>",
            WhitespaceMode::Rendered,
        ),
        "# Story\n\nBody paragraph."
    );
    assert_eq!(
        render_html_as_text(
            "<article><p><a href=\"/promo\"><strong>READ THE FULL TRANSCRIPT HERE</strong></a></p><p>Body paragraph.</p></article>",
            WhitespaceMode::Rendered,
        ),
        "Body paragraph."
    );
    assert_eq!(
        render_html_as_text(
            "<article><div class=\"hatnote\">For other uses, see <a href=\"/wiki/Math_(disambiguation)\">Math (disambiguation)</a>.</div><p>Body paragraph.</p></article>",
            WhitespaceMode::Rendered,
        ),
        "Body paragraph."
    );
    assert_eq!(
        render_html_as_text(
            "<article><pre><a href=\"https://example.com/guide\">\nhttps://example.com/guide\n</a></pre></article>",
            WhitespaceMode::Rendered,
        ),
        "https://example.com/guide"
    );
    assert!(
        render_html_as_text("<article><h2>   </h2></article>", WhitespaceMode::Rendered).is_empty()
    );
    assert_eq!(
        render_html_as_text(
            "<article><h2>Heading <span class=\"mw-editsection-bracket\">[</span><span class=\"mw-editsection-bracket\">]</span></h2></article>",
            WhitespaceMode::Rendered,
        ),
        "## Heading"
    );
    assert_eq!(
        render_html_as_text(
            "<article><h2 class=\"editable-heading\">Heading</h2><p>Body.</p></article>",
            WhitespaceMode::Rendered,
        ),
        "## Heading\n\nBody."
    );
    assert_eq!(
        render_html_as_text(
            "<article><h2 data-editable=\"headline\">Heading</h2><p>Body.</p></article>",
            WhitespaceMode::Rendered,
        ),
        "## Heading\n\nBody."
    );
    assert_eq!(
        render_html_as_text(
            "<article><h1><div><div>Host Liability Insurance Program Summary</div></div></h1><div class=\"notice\"><span class=\"flag\">NEW</span> playback available</div><p>Body paragraph.</p></article>",
            WhitespaceMode::Rendered,
        ),
        "# Host Liability Insurance Program Summary\n\nBody paragraph."
    );
    assert_eq!(
        render_html_as_text(
            "<article><div><div><p>Release Date:</p></div><div><p><b>4/14/2026</b></p></div></div><div><div><p>Version:</p></div><div><p><b>OS Build 20348.5020</b></p></div></div></article>",
            WhitespaceMode::Rendered,
        ),
        "Release Date: 4/14/2026\n\nVersion: OS Build 20348.5020"
    );
    assert_eq!(
        render_html_as_text(
            "<article><table><tr><td><p><b>Change date</b></p></td><td><p><b>Change description</b></p></td></tr><tr><td><p>May 1, 2026</p></td><td><ul><li><p>Improvement added: <b>[Vulnerable driver blocklist]</b></p></li></ul></td></tr><tr><td><p>April 27, 2026</p></td><td><p>Corrected the known issue.</p></td></tr></table></article>",
            WhitespaceMode::Rendered,
        ),
        "Change date    | Change description\nMay 1, 2026    | - Improvement added: [Vulnerable driver blocklist]\nApril 27, 2026 | Corrected the known issue."
    );
    assert_eq!(
        render_html_as_text(
            "<article><table><caption>Windows builds</caption><tr><th>Date</th><th>Build</th></tr><tr><td>April 14, 2026</td><td>20348.5020</td></tr></table></article>",
            WhitespaceMode::Rendered,
        ),
        "Windows builds\nDate           | Build\nApril 14, 2026 | 20348.5020"
    );
    assert_eq!(
        render_html_as_text(
            "<article><h3><button type=\"button\"><div aria-hidden=\"true\">Chevron</div><div>Windows Secure Boot certificate expiration</div></button></h3><p>Body.</p></article>",
            WhitespaceMode::Rendered,
        ),
        "### Windows Secure Boot certificate expiration\n\nBody."
    );
    assert_eq!(
        render_html_as_text(
            "<article><h3><button type=\"button\"><div>Windows Secure Boot certificate expiration</div></button></h3><p><b>Windows Secure Boot certificate expiration</b></p><p>Body.</p></article>",
            WhitespaceMode::Rendered,
        ),
        "### Windows Secure Boot certificate expiration\n\nBody."
    );
    assert_eq!(
        render_html_as_text(
            "<article><div class=\"image-ct inline\"><div class=\"m\"><img alt=\"Rudy Giuliani attending ceremony\" src=\"hero.jpg\"></div><div class=\"info\"><div class=\"caption\"><p>Photo caption.</p></div></div></div><p>Body.</p></article>",
            WhitespaceMode::Rendered,
        ),
        "Body."
    );
    assert_eq!(
        render_html_as_text(
            "<article><div class=\"side-box plainlinks\"><div class=\"side-box-image\"><a href=\"https://example.com/file\"><img alt=\"Wiktionary logo\" src=\"logo.png\"></a></div><div class=\"side-box-text\">Look up <a href=\"https://example.com/help\">help</a> in the dictionary.</div></div><p>Body.</p></article>",
            WhitespaceMode::Rendered,
        ),
        "Body."
    );
    assert_eq!(
        render_html_as_text(
            "<main><h1>Help</h1><p>Body.</p><div class=\"printfooter\" data-nosnippet=\"\">Retrieved from <a href=\"https://example.com/oldid\">old revision</a></div></main>",
            WhitespaceMode::Rendered,
        ),
        "# Help\n\nBody."
    );
    assert_eq!(
        render_html_as_text(
            "<main id=\"main\"><section class=\"section section-design section-product-story\"><h2>Design</h2><p>All-screen front.</p></section><section id=\"accessories\" class=\"section section-accessories section-product-story\"><h2>Accessories</h2><p><a href=\"/shop\">Shop all iPhone accessories</a></p></section><section class=\"section section-faq\"><h2>Questions? Answers.</h2><p>FAQ body.</p></section><section class=\"section section-upgrade\"><h2>Upgrade</h2><p><a href=\"/trade\">Find your trade-in value</a></p></section></main>",
            WhitespaceMode::Rendered,
        ),
        "## Design\n\nAll-screen front."
    );
    assert_eq!(
        render_html_as_text(
            "<main><h1>April 14, 2026—KB5082142</h1><p>Body.</p><div class=\"ocArticleFooterSection articleFooterBridge\"><h3>Need more help?</h3><a href=\"https://example.com/support\">Contact support</a></div></main>",
            WhitespaceMode::Rendered,
        ),
        "# April 14, 2026—KB5082142\n\nBody."
    );
    assert_eq!(
        render_html_as_text(
            "<article><ol start=\"5\"><li>Five</li><li>Six</li></ol></article>",
            WhitespaceMode::Rendered,
        ),
        "5. Five\n6. Six"
    );
    assert_eq!(
        render_html_as_text(
            "<article><ol reversed><li>Two</li><li>One</li></ol></article>",
            WhitespaceMode::Rendered,
        ),
        "2. Two\n1. One"
    );
    assert_eq!(
        render_html_as_text(
            "<article><ol><li value=\"7\">Seven</li><li>Eight</li></ol></article>",
            WhitespaceMode::Rendered,
        ),
        "7. Seven\n8. Eight"
    );
    let pre_image_rendered = render_html_as_text(
        "<pre><img src=\"hero.png\" alt=\"  Hero  \"></pre>",
        WhitespaceMode::Rendered,
    );
    assert_eq!(pre_image_rendered, "Hero");
    let empty_alt_rendered = render_html_as_text(
        "<p><img src=\"hero.png\" alt=\"   \"></p>",
        WhitespaceMode::Rendered,
    );
    assert!(empty_alt_rendered.is_empty());
    let richer_rendered = render_html_as_text(
        "<blockquote><p>Quote</p></blockquote><dl><dt>Term</dt><dd>Definition</dd></dl><p>Use <code>cargo test</code></p>",
        WhitespaceMode::Rendered,
    );
    assert!(richer_rendered.contains("> Quote"));
    assert!(richer_rendered.contains("Term\n: Definition"));
    assert!(richer_rendered.contains("`cargo test`"));
    let block_definition_rendered = render_html_as_text(
        "<dl><dt>Term</dt><dd><p>Definition</p></dd></dl>",
        WhitespaceMode::Rendered,
    );
    assert_eq!(block_definition_rendered, "Term\n: Definition");
    let collapsed_blockquote = render_html_as_text(
        "<blockquote><p>First</p><p></p><p></p><p>Second</p></blockquote>",
        WhitespaceMode::Rendered,
    );
    assert_eq!(collapsed_blockquote, "> First\n>\n> Second");
    let empty_blockquote =
        render_html_as_text("<blockquote>   </blockquote>", WhitespaceMode::Rendered);
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
        " Hello\n\n World"
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
    assert_eq!(resolve_url("   ", Some("https://example.com")), "   ");
    assert_eq!(resolve_url("guide.html", None), "guide.html");
    assert_eq!(resolve_url("guide.html", Some("not a url")), "guide.html");
    assert_eq!(
        rewrite_html_urls("<p>Hello</p>", None, false),
        "<p>Hello</p>"
    );
    let fragment_without_body = Html::parse_fragment("<p>Fragment only</p>");
    assert_eq!(
        render_document_body_as_text(&fragment_without_body, WhitespaceMode::Rendered),
        "Fragment only"
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
